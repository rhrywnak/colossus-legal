// Tests for `rekey::report`.
//
// The report IS the count proof — it is the artifact Roman checks the run
// against, so what it must never do is look clean while something was wrong.

use super::*;
use crate::rekey::plan::{EvidenceRow, RekeyPlan};

fn table(reference: &str, expected: u64, updated: u64) -> TableProof {
    TableProof {
        reference: reference.to_string(),
        expected,
        updated,
    }
}

fn row(current_id: &str, doc: &str, page: i64, quote: &str) -> EvidenceRow {
    EvidenceRow {
        current_id: current_id.to_string(),
        doc_slug: doc.to_string(),
        page: Some(page),
        verbatim_quote: quote.to_string(),
        question: None,
    }
}

#[test]
fn a_table_proof_is_sound_only_on_exact_equality() {
    assert!(table("scenario_fact_refs.graph_node_id", 8, 8).is_sound());
    assert!(!table("scenario_fact_refs.graph_node_id", 8, 7).is_sound());
    // HIGHER than expected is a failure too: the UPDATE matched rows the plan
    // did not know about, which is not a happier outcome than missing some.
    assert!(!table("scenario_fact_refs.graph_node_id", 8, 9).is_sound());
}

/// A dry run must say so, unmistakably and near the top.
#[test]
fn a_dry_run_report_says_nothing_was_written() {
    let plan = RekeyPlan::build(vec![row("stale", "doc-a", 1, "A.")]);
    let rendered = RunReport::from_plan(&plan, false).render();
    assert!(rendered.contains("DRY RUN"), "{rendered}");
    assert!(rendered.contains("nothing was written"), "{rendered}");
    assert!(
        !rendered.contains("EXECUTION"),
        "a dry run must not print execution counts: {rendered}",
    );
}

#[test]
fn an_applied_report_says_the_database_was_written() {
    let plan = RekeyPlan::build(vec![row("stale", "doc-a", 1, "A.")]);
    let rendered = RunReport::from_plan(&plan, true).render();
    assert!(rendered.contains("APPLIED"), "{rendered}");
    assert!(rendered.contains("EXECUTION"), "{rendered}");
}

/// A mismatched table is MARKED, not just recorded.
///
/// The failure mode this guards is a report that carries the wrong numbers in a
/// tidy column and reads as fine at a glance.
#[test]
fn a_mismatched_table_is_flagged_in_the_rendered_report() {
    let plan = RekeyPlan::build(vec![row("stale", "doc-a", 1, "A.")]);
    let mut report = RunReport::from_plan(&plan, true);
    report.documents.push(DocumentProof {
        doc_slug: "doc-a".to_string(),
        nodes_rekeyed: 0,
        nodes_already_current: 0,
        nodes_refused: 0,
        tables: vec![
            table("scenario_fact_refs.graph_node_id", 8, 8),
            table("scan_run_verdicts.graph_node_id", 24, 23),
        ],
        aborted: Some("scan_run_verdicts.graph_node_id: expected 24, updated 23".to_string()),
    });
    let rendered = report.render();
    assert!(rendered.contains("MISMATCH"), "{rendered}");
    assert!(rendered.contains("ABORTED"), "{rendered}");
    assert!(
        rendered.contains("expected 24, updated 23"),
        "the abort reason must name the numbers: {rendered}",
    );
}

/// An aborted document is counted, named, and its reason carried.
#[test]
fn aborted_documents_are_summarised_at_the_top() {
    let plan = RekeyPlan::build(vec![row("stale", "doc-a", 1, "A.")]);
    let mut report = RunReport::from_plan(&plan, true);
    report.documents.push(DocumentProof {
        doc_slug: "doc-b".to_string(),
        nodes_rekeyed: 0,
        nodes_already_current: 0,
        nodes_refused: 0,
        tables: vec![],
        aborted: Some("count mismatch".to_string()),
    });
    assert_eq!(report.aborted_documents().len(), 1);
    let rendered = report.render();
    assert!(
        rendered.contains("Documents aborted        : 1"),
        "{rendered}"
    );
    assert!(rendered.contains("doc-b"), "{rendered}");
}

/// The twin enumeration the completion report owes, with BOTH ids per group and
/// the reason they were left alone.
#[test]
fn refused_twins_are_enumerated_with_the_reason() {
    let plan = RekeyPlan::build(vec![
        row("doc-a:evidence:t1", "doc-a", 9, "Shared words."),
        row("doc-a:evidence:t2", "doc-a", 9, "Shared words."),
    ]);
    let rendered = RunReport::from_plan(&plan, false).render();
    assert!(
        rendered.contains("REFUSED — SHARED KEY (1 groups"),
        "{rendered}"
    );
    assert!(rendered.contains("retains doc-a:evidence:t1"), "{rendered}");
    assert!(rendered.contains("retains doc-a:evidence:t2"), "{rendered}");
    assert!(
        rendered.contains("swap a ruling"),
        "the report must say WHY they were refused: {rendered}",
    );
}

/// Row totals sum across tables and documents, so the headline figure cannot
/// disagree with the breakdown beneath it.
#[test]
fn totals_are_summed_from_the_per_table_proofs() {
    let plan = RekeyPlan::build(vec![row("stale", "doc-a", 1, "A.")]);
    let mut report = RunReport::from_plan(&plan, true);
    report.documents.push(DocumentProof {
        doc_slug: "doc-a".to_string(),
        nodes_rekeyed: 3,
        nodes_already_current: 0,
        nodes_refused: 0,
        tables: vec![table("t1.c", 5, 5), table("t2.c", 7, 7)],
        aborted: None,
    });
    report.documents.push(DocumentProof {
        doc_slug: "doc-b".to_string(),
        nodes_rekeyed: 2,
        nodes_already_current: 1,
        nodes_refused: 0,
        tables: vec![table("t1.c", 4, 4)],
        aborted: None,
    });
    assert_eq!(report.rows_updated(), 16);
    assert_eq!(report.nodes_rekeyed(), 5);
    let rendered = report.render();
    assert!(
        rendered.contains("Referencing rows updated : 16"),
        "{rendered}"
    );
    assert!(
        rendered.contains("Nodes re-keyed           : 5"),
        "{rendered}"
    );
}

/// The plan's own numbers reach the report unchanged — the dry run's whole value.
#[test]
fn the_plan_totals_are_reported_verbatim() {
    let plan = RekeyPlan::build(vec![
        row("stale-1", "doc-a", 1, "Unique one."),
        row("doc-a:evidence:t1", "doc-a", 9, "Shared."),
        row("doc-a:evidence:t2", "doc-a", 9, "Shared."),
    ]);
    let rendered = RunReport::from_plan(&plan, false).render();
    assert!(
        rendered.contains("Evidence nodes seen      : 3"),
        "{rendered}"
    );
    assert!(
        rendered.contains("To re-key                : 1"),
        "{rendered}"
    );
    assert!(
        rendered.contains("Refused, shared key      : 2"),
        "{rendered}"
    );
}
