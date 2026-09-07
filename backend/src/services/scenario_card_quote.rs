//! The card's SOURCE block — where the quoted words came from and who said them.
//!
//! Split out of `scenario_card.rs` for the module-size limit (Rule 17). The four
//! builders here answer one question between them — *what is on the page, and on
//! whose authority* — so they travel together: the pinpoint (which document, which
//! page), the speaker (who is recorded as saying it), and the quote block (the
//! anchor plus its two flanks).
//!
//! ## Rust Learning: `pub(super)` visibility
//!
//! These functions are called only by `scenario_card::build_card`, its sibling in
//! `services`. `pub(super)` says exactly that — visible to the parent module
//! (`services`) and its descendants, invisible to the rest of the crate. It is the
//! narrowest visibility that still crosses a file boundary, and it keeps the split
//! an implementation detail rather than a new public surface.

use crate::bias::dto::BiasInstance;
use crate::dto::scenario_card::{CardPinpoint, CardQuote, CardSpeaker};
use crate::repositories::pipeline_repository::EvidenceSummaryOverrideRecord;
use crate::services::scenario_card_context::QuoteContext;
use crate::services::scenario_human_links::resolve_question;

/// The pinpoint line as the card shows it, composed server-side.
///
/// A page-less item reads as the document alone rather than "… at null" — the
/// absence is rendered by omission, never by a placeholder.
pub(super) fn pinpoint_label(document_title: &str, page: Option<i64>) -> String {
    match page {
        Some(page) => format!("{document_title} at {page}"),
        None => document_title.to_string(),
    }
}

/// The §7.2 pinpoint: where the quote is, and how to get there.
///
/// Split out of [`build_card`] for the function-size limit (Rule 18). The document
/// title is read twice — once composed into `label`, once on its own — because the
/// card renders the composed line but a client filtering or grouping by document
/// needs the bare title, and re-deriving it by parsing `label` would be the browser
/// taking a display string apart to recover data.
///
/// A document-less instance yields empty strings rather than `None`: these fields
/// are always present on the wire so the card's layout is stable, and an absent
/// title renders as omission (see [`pinpoint_label`]).
pub(super) fn build_pinpoint(instance: &BiasInstance) -> CardPinpoint {
    let document_title = instance
        .document
        .as_ref()
        .map(|d| d.title.clone())
        .unwrap_or_default();
    let document_id = instance
        .document
        .as_ref()
        .map(|d| d.id.clone())
        .unwrap_or_default();

    CardPinpoint {
        label: pinpoint_label(&document_title, instance.page_number),
        document_title,
        page: instance.page_number,
        viewer_href: crate::domain::card_language::viewer_href(&document_id, instance.page_number),
        document_id,
    }
}

/// The §7.3 speaker: who said it, and on what authority we say so.
///
/// Split out of [`build_card`] for the function-size limit (Rule 18).
pub(super) fn build_speaker(instance: &BiasInstance, extracted_label: &str) -> CardSpeaker {
    CardSpeaker {
        // An empty speaker name IS absent — `evidence_by_ids` decodes a missing
        // STATED_BY edge to `coalesce(…, '')`. Filtering the blank keeps "nobody is
        // recorded as saying this" distinct from "somebody said it and their name is
        // the empty string", which is the distinction a documentary exhibit needs.
        name: instance
            .stated_by
            .as_ref()
            .map(|a| a.name.clone())
            .filter(|n| !n.trim().is_empty()),
        attribution: extracted_label.to_string(),
    }
}

/// The §7.1 quote block: the anchor, its two flanks, and how each flank ended.
///
/// Split out of [`build_card`] for the function-size limit (Rule 18) — task 1.7C
/// added four fields to this struct literal and `build_card` was already over.
///
/// It earns the split beyond arithmetic: this is the one place the CONTEXT LAW's
/// output is shaped for the wire, and the pairing it establishes is not obvious.
/// Each flank contributes THREE things — the text, a boolean saying whether that
/// text ended at a real sentence boundary, and (only when it did not) a
/// server-composed notice naming the page edge. The boolean is the state a client
/// branches on; the notice is the words, because the language law puts every display
/// decision on this side of the wire. Keeping them adjacent here makes a future
/// change that sets one and forgets the other visible.
pub(super) fn build_quote(
    text: String,
    context: QuoteContext,
    question: Option<String>,
    override_row: Option<&EvidenceSummaryOverrideRecord>,
    machine_authorship_label: &str,
) -> CardQuote {
    let (question, question_authorship) =
        resolve_question(question, override_row, machine_authorship_label);
    CardQuote {
        text,
        context_before: context.before.text,
        context_after: context.after.text,
        context_before_complete: context.before.complete,
        context_after_complete: context.after.complete,
        context_before_notice: context.before.notice,
        context_after_notice: context.after.notice,
        question,
        question_authorship,
    }
}
