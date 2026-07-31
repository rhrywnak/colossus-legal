//! Pure shaping for `GET /api/cases/:slug/case-health/inventory`: turns the four
//! raw row sets from [`super::case_health_repository`] into the wire
//! [`GraphInventory`].
//!
//! ## Why a builder, and why every rate is computed here
//!
//! This module is the ONLY place a percentage is computed for Pane 1. Not in
//! Cypher (Neo4j would round differently and give the corpus and the per-document
//! rows two rounding behaviours), and emphatically not in the frontend — "zero
//! business logic in the frontend" means the browser divides nothing. One
//! function, [`percent`], produces every rate on the screen, so the headline and
//! a table cell can never disagree about what "19.6%" means.
//!
//! Keeping the shaping pure (no `Graph`, no I/O, no clock) is what lets all of
//! it be unit-tested without a live database — the split
//! `services::scenario_dashboard` established and `proof_review_builder`
//! follows. `computed_at` is passed IN rather than read from `Utc::now()` here,
//! precisely so these tests stay deterministic.

use crate::domain::connection_tier::TOPICAL_EDGE_TYPES;
use crate::dto::case_health::{
    CorpusConnection, DocumentConnection, EdgeClassCount, EdgeTripleCount, GraphInventory,
    LabelCount,
};

use super::case_health_repository::{CorpusRow, DocumentRow, EdgeTripleRow, LabelCountRow};

/// Multiply-round-divide factor fixing every published rate at one decimal place.
///
/// `10.0` = 10^1 = one decimal. See the `// CONST:` note at the use site in
/// [`percent`] for why this is a compile-time contract rather than configuration.
const PERCENT_ROUNDING_FACTOR: f64 = 10.0;

/// A count as a percentage of a total, rounded to one decimal place.
///
/// Returns `None` — never `0.0` — when `total` is zero. Standing Rule 1: "0% of
/// nothing" and "0% of 525 items" are operationally different states, and a bare
/// `0.0` collapses them into one. A document that produced no Evidence must read
/// "—", not "0.0% connected", which would imply a measurement that was never
/// taken. `total < 0` is likewise `None`: it cannot occur (every total is a
/// Cypher `count`), and inventing a percentage from it would be worse than
/// admitting we have nothing to report.
///
/// ## Rust Learning: integer → float, and why the rounding is explicit
///
/// Rust will not implicitly widen `i64` to `f64`; the `as` casts are required
/// and are the reason the arithmetic reads a little heavier than it would in a
/// dynamic language. The `* 10.0).round() / 10.0` idiom is the standard way to
/// fix a decimal place — there is no `round_to(n)` in std. Rounding here (rather
/// than in the frontend's formatter) means the number the API returns is exactly
/// the number on screen, which is what the provenance rule requires: a human
/// pastes the documented query, divides, and gets the same digits.
pub(crate) fn percent(count: i64, total: i64) -> Option<f64> {
    if total <= 0 {
        return None;
    }
    let raw = (count as f64) * 100.0 / (total as f64);
    // CONST: 1 decimal place is the PUBLISHED wire format, not a display
    // preference — the TypeScript type, the rendered cell and the documented
    // query in `docs/CASE_HEALTH_QUERIES.md` all assume these digits, so changing
    // the precision is a versioned contract change, never a per-deployment knob.
    Some((raw * PERCENT_ROUNDING_FACTOR).round() / PERCENT_ROUNDING_FACTOR)
}

/// The edge-class column list the payload ships so the frontend holds no
/// vocabulary of its own. Owned `String`s because the DTO crosses the wire.
pub(crate) fn edge_class_names() -> Vec<String> {
    TOPICAL_EDGE_TYPES.iter().map(|r| r.to_string()).collect()
}

/// Split label rows into the populated-label list and the unlabeled-node tally.
///
/// `labels(n)[0]` is null for a node carrying no label, so such nodes arrive as
/// a row with `label: None`. They are counted into their own field rather than
/// dropped: dropping them would make `node_labels` silently fail to account for
/// every node in the graph, and an unlabeled node is a real data-integrity
/// finding (Standing Rule 1 — every distinct state gets a distinct observable).
fn split_label_rows(rows: Vec<LabelCountRow>) -> (Vec<LabelCount>, i64) {
    let mut labels = Vec::with_capacity(rows.len());
    let mut unlabeled = 0_i64;
    for row in rows {
        match row.label {
            Some(label) => labels.push(LabelCount {
                label,
                node_count: row.node_count,
            }),
            None => unlabeled += row.node_count,
        }
    }
    (labels, unlabeled)
}

/// Shape the corpus aggregation into its wire block, computing the three rates.
///
/// `inert` is a plain subtraction, not a saturating one. `topical_connected` is
/// tallied over the very rows `evidence_total` counts, so it can never exceed
/// it; clamping would only serve to hide a future query bug that broke that
/// invariant, and hiding it is exactly what Standing Rule 1 forbids.
fn build_corpus(row: CorpusRow) -> CorpusConnection {
    let inert = row.evidence_total - row.topical_connected;
    CorpusConnection {
        evidence_total: row.evidence_total,
        probative_connected: row.probative_connected,
        topical_connected: row.topical_connected,
        inert,
        probative_rate_pct: percent(row.probative_connected, row.evidence_total),
        topical_rate_pct: percent(row.topical_connected, row.evidence_total),
        inert_rate_pct: percent(inert, row.evidence_total),
        evidence_without_document: row.evidence_without_document,
    }
}

/// Shape one Document row, zipping its parallel per-class counts back onto the
/// class names.
///
/// The repository builds one query column per entry of `TOPICAL_EDGE_TYPES`, in
/// that order, so `row.by_edge_class` is positionally parallel to it. `zip`
/// stops at the shorter of the two, which means a length mismatch would silently
/// truncate — so the caller asserts the lengths agree
/// (`document_class_counts_are_parallel_to_the_tier_constant` in the tests) and
/// the repository builds both from the same constant, leaving no way for them to
/// drift at runtime.
fn build_document(row: DocumentRow) -> DocumentConnection {
    let inert = row.evidence_total - row.topical_connected;
    let by_edge_class = TOPICAL_EDGE_TYPES
        .iter()
        .zip(row.by_edge_class.iter())
        .map(|(rel_type, item_count)| EdgeClassCount {
            rel_type: rel_type.to_string(),
            item_count: *item_count,
        })
        .collect();

    DocumentConnection {
        document_id: row.document_id,
        title: row.title,
        doc_type: row.doc_type,
        evidence_total: row.evidence_total,
        probative_connected: row.probative_connected,
        topical_connected: row.topical_connected,
        inert,
        probative_rate_pct: percent(row.probative_connected, row.evidence_total),
        topical_rate_pct: percent(row.topical_connected, row.evidence_total),
        inert_rate_pct: percent(inert, row.evidence_total),
        by_edge_class,
    }
}

/// Assemble one complete measurement from the four row sets.
///
/// `computed_at` is supplied by the caller (RFC 3339, UTC) rather than read from
/// the clock here — see the module note on determinism.
pub(crate) fn build_inventory(
    computed_at: String,
    label_rows: Vec<LabelCountRow>,
    edge_rows: Vec<EdgeTripleRow>,
    corpus_row: CorpusRow,
    document_rows: Vec<DocumentRow>,
) -> GraphInventory {
    let (node_labels, unlabeled_node_count) = split_label_rows(label_rows);

    GraphInventory {
        computed_at,
        corpus: build_corpus(corpus_row),
        node_labels,
        unlabeled_node_count,
        edge_triples: edge_rows
            .into_iter()
            .map(|row| EdgeTripleCount {
                from_label: row.from_label,
                rel_type: row.rel_type,
                to_label: row.to_label,
                edge_count: row.edge_count,
            })
            .collect(),
        documents: document_rows.into_iter().map(build_document).collect(),
    }
}

#[cfg(test)]
#[path = "case_health_builder_tests.rs"]
mod tests;
