// =============================================================================
// backend/src/services/matrix_detail.rs — the human's layer over the machine's
// =============================================================================
//
// PROOF_MATRIX_v2 §3, applied to a fetched [`ElementDetailResponse`]. The
// repository returns what the GRAPH says; this puts what a HUMAN said on top of
// it and hands back a list in reading order.
//
// Three things happen here, in this order and for this reason:
//
//   1. Each item is stamped with its ruling, if a human has made one. Nothing can
//      be ordered until this is known, because a `keep` is the first sort key.
//   2. [`crate::services::matrix_order::order_items`] decides visibility, folding
//      and order. That function is pure and is the SAME one the Word export
//      calls, which is what makes "the page and the document agree" a property of
//      the code rather than a coincidence.
//   3. Each surviving row gets its RFA line, if it is a Q&A card.
//
// ## Why this is a service and not a method on the repository
//
// It needs three things the repository has no handle on: the rulings (a Postgres
// read against the pipeline pool), the stored wording, and the pure ordering. A
// repository that reached for all three would be the composition root, and the
// handler is already that.

use std::collections::HashMap;

use crate::domain::matrix_edge::{EdgeRole, LinkConfidence};
use crate::domain::matrix_ruling::MatrixRuling;
use crate::domain::wording_matrix::MatrixWording;
use crate::repositories::element_detail_repository::{ElementDetailResponse, EvidenceRef};
use crate::repositories::pipeline_repository::evidence_allegation_rulings::RulingRecord;
use crate::services::matrix_order::{order_items, OrderInput};
use crate::services::matrix_rfa::rfa_line;

/// Apply human rulings, §3's ordering and the RFA rendering to a whole Element
/// detail payload, in place.
///
/// ## Rust Learning: `&mut` on the response instead of returning a new one
///
/// The response already owns a `Vec` per Allegation per leg; rebuilding the tree
/// to change a few fields would clone every quote. Taking `&mut` lets each `Vec`
/// be replaced in place. The function returns nothing — its whole effect is the
/// mutation, which the name says.
pub fn apply_rulings_and_order(
    response: &mut ElementDetailResponse,
    rulings: &[RulingRecord],
    wording: &MatrixWording,
) {
    // `(allegation_id, evidence_id)` → the human's verdict. Built once for the
    // whole payload: an Element can carry 40 Allegations, and a linear scan per
    // item would be quadratic over a list a reader is waiting on.
    let index = index_rulings(rulings);

    for allegation in &mut response.allegations {
        for leg in [
            &mut allegation.supporting_evidence,
            &mut allegation.disputing_evidence,
        ] {
            stamp_rulings(leg, &allegation.allegation_id, &index);
            reorder(leg);
            for item in leg.iter_mut() {
                item.rfa_line = rfa_line(
                    item.question.as_deref(),
                    item.answer.as_deref(),
                    item.paragraph.as_deref(),
                    wording,
                );
            }
        }
    }
}

/// `(allegation_id, evidence_id)` → verdict, skipping rows this build cannot
/// read.
///
/// ## Why an unreadable verdict is logged and dropped rather than propagated
///
/// The token is written only by this application, through a typed enum, so an
/// unreadable one means the row was edited around the API. Failing the request
/// would blank an Element's whole panel over one bad row; guessing would put a
/// verdict on screen that nobody reached. Dropping it renders the item as
/// UNRULED — the machine's own order, no confirmation mark — which is the state
/// that claims the least, and the `error!` names the pair and the token so an
/// operator can repair it.
fn index_rulings(rulings: &[RulingRecord]) -> HashMap<(&str, &str), &RulingRecord> {
    let mut index = HashMap::with_capacity(rulings.len());
    for record in rulings {
        match record.verdict() {
            Ok(_) => {
                index.insert(
                    (record.allegation_id.as_str(), record.evidence_id.as_str()),
                    record,
                );
            }
            Err(e) => tracing::error!(
                error = %e,
                allegation_id = %record.allegation_id,
                evidence_id = %record.evidence_id,
                token = %record.ruling,
                "a stored ruling carries a verdict this build cannot read — the item \
                 renders as UNRULED (machine order, no confirmation mark); repair the \
                 row or widen domain::matrix_ruling::MatrixRuling"
            ),
        }
    }
    index
}

/// Copy each item's ruling onto it, so the ordering and the renderer read one
/// source.
fn stamp_rulings(
    leg: &mut [EvidenceRef],
    allegation_id: &str,
    index: &HashMap<(&str, &str), &RulingRecord>,
) {
    for item in leg.iter_mut() {
        let record = index.get(&(allegation_id, item.id.as_str()));
        item.ruling = record.map(|r| r.ruling.clone());
        item.ruled_by = record.map(|r| r.ruled_by.clone());
    }
}

/// Reorder one leg in place: §3's order, with duplicates folded and hidden items
/// flagged and moved to the end.
///
/// ## Why the leg is rebuilt by id rather than sorted with a comparator
///
/// `order_items` does three things a `sort_by` cannot: it REMOVES folded
/// duplicates, it stamps an occurrence count on their target, and it partitions
/// visible from hidden. Rebuilding from its answer keeps all three decisions in
/// the one pure, tested function.
fn reorder(leg: &mut Vec<EvidenceRef>) {
    let inputs: Vec<OrderInput<'_>> = leg.iter().map(order_input).collect();
    let ordered = order_items(&inputs);

    // Drain into a lookup so each item can be MOVED into its new position
    // without cloning its quotes.
    let mut by_id: HashMap<String, EvidenceRef> =
        leg.drain(..).map(|item| (item.id.clone(), item)).collect();

    for placed in ordered {
        let Some(mut item) = by_id.remove(&placed.id) else {
            // Unreachable: every id came from this very list. Logged rather than
            // skipped silently, because if it happened a piece of evidence would
            // have vanished between two lines of one function — and a missing
            // item on a proof surface must never be something a reader has to
            // notice for themselves.
            tracing::error!(
                evidence_id = %placed.id,
                "the ordering names an evidence id that is not in the leg it was \
                 built from — the item has been dropped from the drill-down; this \
                 indicates a defect in matrix_order::order_items, not a data problem"
            );
            continue;
        };
        item.occurrences = placed.occurrences;
        item.hidden_reason = placed.hidden_reason.map(|r| r.code().to_string());
        leg.push(item);
    }
}

/// Project one row into the flat shape the ordering reads.
fn order_input(item: &EvidenceRef) -> OrderInput<'_> {
    // best-effort: all three tokens were already validated before they reached
    // this struct — the two graph vocabularies by `element_detail_fold::
    // readable_token`, the verdict by `index_rulings` — so each `try_from` is
    // total in practice and `.ok()` states that rather than hiding a failure.
    // If either guarantee were ever relaxed the effect is the SAFE direction: an
    // item ordered as though it carried no role, no confidence or no ruling is
    // an item that is SHOWN, in the machine's own order. Nothing is hidden, and
    // the layer that dropped the token has already logged it by id.
    OrderInput {
        id: &item.id,
        rank: item.rank,
        role: item.role.as_deref().and_then(parse_role),
        confidence: item.confidence.as_deref().and_then(parse_confidence),
        duplicate_of: item.duplicate_of_card_id.as_deref(),
        document_date: item.document_date.as_deref(),
        ruling: item.ruling.as_deref().and_then(parse_ruling),
    }
}

/// The three token parses, named so the `.ok()` above reads as one decision
/// rather than three closures.
///
/// Each is `Option`-returning on purpose: see the comment in [`order_input`] for
/// why `None` is the safe answer and where the loud one already happened.
fn parse_role(token: &str) -> Option<EdgeRole> {
    EdgeRole::try_from(token).ok()
}

fn parse_confidence(token: &str) -> Option<LinkConfidence> {
    LinkConfidence::try_from(token).ok()
}

fn parse_ruling(token: &str) -> Option<MatrixRuling> {
    MatrixRuling::try_from(token).ok()
}

#[cfg(test)]
#[path = "matrix_detail_tests.rs"]
mod tests;
