//! Unit tests for [`super`] — the pure Case Health shaping layer.
//!
//! Everything here runs without a database: the builder takes rows in and gives
//! DTOs out, which is the whole reason the shaping was split from the repository.
//! `computed_at` is a fixed literal, so no test depends on the clock.

use super::*;
use crate::neo4j::schema;

/// A Document row with the four per-class counts supplied positionally, in
/// `TOPICAL_EDGE_TYPES` order (CORROBORATES, REBUTS, CHARACTERIZES, ABOUT).
fn doc_row(
    id: &str,
    evidence_total: i64,
    probative: i64,
    topical: i64,
    classes: [i64; 4],
) -> DocumentRow {
    DocumentRow {
        document_id: id.to_string(),
        title: Some(format!("Title of {id}")),
        doc_type: Some("court_ruling".to_string()),
        evidence_total,
        probative_connected: probative,
        topical_connected: topical,
        by_edge_class: classes.to_vec(),
    }
}

fn corpus_row(total: i64, probative: i64, topical: i64, orphans: i64) -> CorpusRow {
    CorpusRow {
        evidence_total: total,
        probative_connected: probative,
        topical_connected: topical,
        evidence_without_document: orphans,
    }
}

// ── percent ───────────────────────────────────────────────────────────────────

/// A zero denominator yields `None`, never `Some(0.0)`. This is the single most
/// important assertion in the file: it is what keeps "this document produced no
/// Evidence" distinguishable from "this document produced Evidence and none of
/// it connected" (Standing Rule 1).
#[test]
fn percent_of_zero_total_is_none_not_zero() {
    assert_eq!(percent(0, 0), None);
    assert_eq!(percent(5, 0), None);
}

/// A genuine zero — some Evidence, none connected — IS reported as 0.0, and must
/// not be confused with the `None` case above.
#[test]
fn percent_of_zero_count_over_positive_total_is_zero() {
    assert_eq!(percent(0, 525), Some(0.0));
}

/// A negative total cannot occur (every total is a Cypher `count`), but if one
/// ever arrived we report nothing rather than inventing a percentage.
#[test]
fn percent_of_negative_total_is_none() {
    assert_eq!(percent(3, -1), None);
}

/// Rounding is to exactly one decimal place, half-away-from-zero (`f64::round`).
/// The provenance rule depends on this: a human running the documented query and
/// dividing must get the digits the screen shows.
#[test]
fn percent_rounds_to_one_decimal_place() {
    // 103/525 = 19.6190…  → 19.6
    assert_eq!(percent(103, 525), Some(19.6));
    // 126/525 = 24.0 exactly
    assert_eq!(percent(126, 525), Some(24.0));
    // 1/3 = 33.333… → 33.3
    assert_eq!(percent(1, 3), Some(33.3));
    // 2/3 = 66.666… → 66.7 (rounds up)
    assert_eq!(percent(2, 3), Some(66.7));
    // 1/8 = 12.5 exactly — no drift from the multiply/divide idiom
    assert_eq!(percent(1, 8), Some(12.5));
}

#[test]
fn percent_of_everything_is_one_hundred() {
    assert_eq!(percent(9, 9), Some(100.0));
}

// ── edge_class_names ──────────────────────────────────────────────────────────

/// The shipped column list is the tier constant, in order — not a hand-typed
/// list that could drift from the definition the queries use.
#[test]
fn edge_class_names_mirror_the_tier_constant() {
    assert_eq!(
        edge_class_names(),
        vec![
            schema::CORROBORATES.to_string(),
            schema::REBUTS.to_string(),
            schema::CHARACTERIZES.to_string(),
            schema::ABOUT.to_string(),
        ]
    );
    assert_eq!(edge_class_names().len(), TOPICAL_EDGE_TYPES.len());
}

// ── split_label_rows ──────────────────────────────────────────────────────────

/// Unlabeled nodes are counted into their own tally, never dropped. Without this
/// the label counts would silently fail to account for every node in the graph.
#[test]
fn unlabeled_nodes_are_tallied_separately_not_dropped() {
    let rows = vec![
        LabelCountRow {
            label: Some("Evidence".to_string()),
            node_count: 525,
        },
        LabelCountRow {
            label: None,
            node_count: 4,
        },
        LabelCountRow {
            label: Some("Document".to_string()),
            node_count: 9,
        },
    ];
    let (labels, unlabeled) = split_label_rows(rows);

    assert_eq!(unlabeled, 4);
    assert_eq!(
        labels.len(),
        2,
        "only the labeled rows become label entries"
    );
    assert_eq!(labels[0].label, "Evidence");
    assert_eq!(labels[0].node_count, 525);
    assert_eq!(labels[1].label, "Document");

    // Every node is accounted for: the labeled counts plus the unlabeled tally
    // equal the graph total. This is the invariant dropping the None row would
    // have broken.
    let labeled_total: i64 = labels.iter().map(|l| l.node_count).sum();
    assert_eq!(labeled_total + unlabeled, 525 + 9 + 4);
}

/// Several unlabeled rows (possible if the driver splits them) accumulate rather
/// than the last one overwriting the tally.
#[test]
fn multiple_unlabeled_rows_accumulate() {
    let rows = vec![
        LabelCountRow {
            label: None,
            node_count: 2,
        },
        LabelCountRow {
            label: None,
            node_count: 3,
        },
    ];
    let (labels, unlabeled) = split_label_rows(rows);
    assert!(labels.is_empty());
    assert_eq!(unlabeled, 5);
}

/// A graph with no unlabeled nodes reports 0 — a real measurement, present in
/// every payload, not an absent field the reader has to interpret.
#[test]
fn no_unlabeled_nodes_reports_zero() {
    let rows = vec![LabelCountRow {
        label: Some("Evidence".to_string()),
        node_count: 1,
    }];
    let (_, unlabeled) = split_label_rows(rows);
    assert_eq!(unlabeled, 0);
}

// ── build_corpus ──────────────────────────────────────────────────────────────

/// The headline block: inert is the topical complement, and the two rates are
/// computed independently against the same denominator.
#[test]
fn corpus_computes_inert_and_both_rates() {
    let corpus = build_corpus(corpus_row(525, 103, 126, 0));

    assert_eq!(corpus.evidence_total, 525);
    assert_eq!(corpus.probative_connected, 103);
    assert_eq!(corpus.topical_connected, 126);
    // Inert is measured against TOPICAL, not probative: an item that is merely
    // ABOUT an allegation is connected-but-not-probative, and calling it inert
    // would double-punish it.
    assert_eq!(corpus.inert, 399);
    assert_eq!(corpus.probative_rate_pct, Some(19.6));
    assert_eq!(corpus.topical_rate_pct, Some(24.0));
    assert_eq!(corpus.inert_rate_pct, Some(76.0));
}

/// The probative rate is always ≤ the topical rate, because the probative edge
/// set is a strict subset. If this ever inverts, the tiers have been swapped
/// somewhere and the headline is overstating the case.
#[test]
fn probative_rate_never_exceeds_topical_rate() {
    let corpus = build_corpus(corpus_row(525, 103, 126, 0));
    let probative = corpus.probative_rate_pct.expect("rate present");
    let topical = corpus.topical_rate_pct.expect("rate present");
    assert!(
        probative <= topical,
        "{probative} must not exceed {topical}"
    );
}

/// An empty graph reports zeros and `None` rates — never a fabricated 0.0%.
#[test]
fn empty_corpus_reports_none_rates() {
    let corpus = build_corpus(corpus_row(0, 0, 0, 0));
    assert_eq!(corpus.evidence_total, 0);
    assert_eq!(corpus.inert, 0);
    assert_eq!(corpus.probative_rate_pct, None);
    assert_eq!(corpus.topical_rate_pct, None);
    assert_eq!(corpus.inert_rate_pct, None);
}

/// Orphaned Evidence (no `CONTAINED_IN` Document) is carried through verbatim so
/// a gap between the headline and the per-document table has a number attached.
#[test]
fn orphaned_evidence_count_is_carried_through() {
    let corpus = build_corpus(corpus_row(525, 103, 126, 7));
    assert_eq!(corpus.evidence_without_document, 7);
}

// ── build_document ────────────────────────────────────────────────────────────

/// Per-class counts zip back onto the class names in tier order.
#[test]
fn document_class_counts_zip_onto_the_tier_names() {
    let doc = build_document(doc_row("doc-a", 138, 8, 12, [3, 4, 2, 6]));

    assert_eq!(doc.by_edge_class.len(), 4);
    assert_eq!(doc.by_edge_class[0].rel_type, schema::CORROBORATES);
    assert_eq!(doc.by_edge_class[0].item_count, 3);
    assert_eq!(doc.by_edge_class[1].rel_type, schema::REBUTS);
    assert_eq!(doc.by_edge_class[1].item_count, 4);
    assert_eq!(doc.by_edge_class[2].rel_type, schema::CHARACTERIZES);
    assert_eq!(doc.by_edge_class[2].item_count, 2);
    assert_eq!(doc.by_edge_class[3].rel_type, schema::ABOUT);
    assert_eq!(doc.by_edge_class[3].item_count, 6);
}

/// The per-class counts a Document row carries must be positionally parallel to
/// `TOPICAL_EDGE_TYPES`, because `zip` would otherwise truncate silently. The
/// repository builds both from that same constant; this pins the contract on the
/// builder's side so a future hand-built row cannot get away with a short list.
#[test]
fn document_class_counts_are_parallel_to_the_tier_constant() {
    let row = doc_row("doc-a", 1, 0, 0, [0, 0, 0, 0]);
    assert_eq!(
        row.by_edge_class.len(),
        TOPICAL_EDGE_TYPES.len(),
        "a DocumentRow must carry exactly one count per topical edge class"
    );
}

/// Rates and inert are computed per document against that document's own
/// Evidence total, not against the corpus.
#[test]
fn document_rates_use_the_documents_own_denominator() {
    let doc = build_document(doc_row("doc-a", 138, 8, 12, [3, 4, 2, 6]));
    assert_eq!(doc.inert, 126);
    // 8/138 = 5.797… → 5.8
    assert_eq!(doc.probative_rate_pct, Some(5.8));
    // 12/138 = 8.695… → 8.7
    assert_eq!(doc.topical_rate_pct, Some(8.7));
    // 126/138 = 91.30… → 91.3
    assert_eq!(doc.inert_rate_pct, Some(91.3));
}

/// A Document that produced NO Evidence still becomes a row — that is a finding,
/// not a reason to disappear — and its rates are `None`, not 0.0.
#[test]
fn document_with_no_evidence_reports_zeros_and_none_rates() {
    let doc = build_document(doc_row("doc-empty", 0, 0, 0, [0, 0, 0, 0]));
    assert_eq!(doc.evidence_total, 0);
    assert_eq!(doc.inert, 0);
    assert_eq!(doc.probative_rate_pct, None);
    assert_eq!(doc.topical_rate_pct, None);
    assert_eq!(doc.inert_rate_pct, None);
    assert_eq!(doc.by_edge_class.len(), 4);
}

/// Absent graph properties stay absent — the builder never invents a title or a
/// doc_type placeholder (property-name / no-fabrication discipline).
#[test]
fn document_absent_title_and_doc_type_stay_none() {
    let row = DocumentRow {
        document_id: "doc-bare".to_string(),
        title: None,
        doc_type: None,
        evidence_total: 0,
        probative_connected: 0,
        topical_connected: 0,
        by_edge_class: vec![0, 0, 0, 0],
    };
    let doc = build_document(row);
    assert_eq!(doc.title, None);
    assert_eq!(doc.doc_type, None);
    assert_eq!(doc.document_id, "doc-bare");
}

// ── build_inventory ───────────────────────────────────────────────────────────

/// End-to-end shaping: every row set lands in its field, order is preserved, and
/// the clock is the caller's.
#[test]
fn build_inventory_assembles_every_section_in_order() {
    let inventory = build_inventory(
        "2026-07-27T12:00:00+00:00".to_string(),
        vec![
            LabelCountRow {
                label: Some("Evidence".to_string()),
                node_count: 525,
            },
            LabelCountRow {
                label: None,
                node_count: 1,
            },
        ],
        vec![
            EdgeTripleRow {
                from_label: Some("Evidence".to_string()),
                rel_type: schema::ABOUT.to_string(),
                to_label: Some("Person".to_string()),
                edge_count: 614,
            },
            EdgeTripleRow {
                from_label: None,
                rel_type: schema::CONTAINED_IN.to_string(),
                to_label: Some("Document".to_string()),
                edge_count: 2,
            },
        ],
        corpus_row(525, 103, 126, 0),
        vec![
            doc_row("doc-b", 138, 8, 12, [3, 4, 2, 6]),
            doc_row("doc-a", 40, 20, 22, [10, 5, 5, 2]),
        ],
    );

    assert_eq!(inventory.computed_at, "2026-07-27T12:00:00+00:00");
    assert_eq!(inventory.corpus.evidence_total, 525);
    assert_eq!(inventory.node_labels.len(), 1);
    assert_eq!(inventory.unlabeled_node_count, 1);

    // Edge triples pass through verbatim, including a null endpoint label.
    assert_eq!(inventory.edge_triples.len(), 2);
    assert_eq!(inventory.edge_triples[0].edge_count, 614);
    assert_eq!(inventory.edge_triples[1].from_label, None);

    // Document order is the query's order (Evidence desc), preserved — the
    // frontend sorts nothing.
    assert_eq!(inventory.documents.len(), 2);
    assert_eq!(inventory.documents[0].document_id, "doc-b");
    assert_eq!(inventory.documents[1].document_id, "doc-a");
}

/// An entirely empty graph still produces a well-formed, honest payload: empty
/// lists, zeroed counts, `None` rates. No panic, no fabricated figures.
#[test]
fn build_inventory_handles_an_empty_graph() {
    let inventory = build_inventory(
        "2026-07-27T12:00:00+00:00".to_string(),
        vec![],
        vec![],
        corpus_row(0, 0, 0, 0),
        vec![],
    );

    assert!(inventory.node_labels.is_empty());
    assert!(inventory.edge_triples.is_empty());
    assert!(inventory.documents.is_empty());
    assert_eq!(inventory.unlabeled_node_count, 0);
    assert_eq!(inventory.corpus.probative_rate_pct, None);
}
