// Tests for `rekey::execute`.
//
// The write path itself needs two live stores and is exercised from the runbook's
// dry run against DEV. What IS unit-testable is the part that decides whether a
// write may happen at all — the column list every count and update walks, and the
// pre-flight refusal — and those are the two places a defect would be silent.

use super::*;

/// The eight columns are exactly the eight measured on 2026-08-14.
///
/// A column added to the schema and forgotten here would leave rows pointing at
/// ids that no longer exist, and the count proof would not notice, because the
/// proof walks this same list. Pinning membership is the only place that gap can
/// be caught.
#[test]
fn the_referencing_column_list_is_the_measured_eight() {
    assert_eq!(REFERENCING_COLUMNS.len(), 8);
    for expected in [
        ("scenario_fact_refs", "graph_node_id"),
        ("scenario_human_facts", "anchor_graph_node_id"),
        ("scenario_human_facts", "answers_graph_node_id"),
        ("evidence_allegation_links", "graph_node_id"),
        ("evidence_allegation_link_events", "graph_node_id"),
        ("scenario_ruling_anchors", "graph_node_id"),
        ("scenario_candidate_ordinals", "graph_node_id"),
        ("scan_run_verdicts", "graph_node_id"),
    ] {
        assert!(
            REFERENCING_COLUMNS.contains(&expected),
            "{expected:?} is missing from REFERENCING_COLUMNS",
        );
    }
}

/// `scenario_human_facts` contributes TWO columns — an anchor and an answer are
/// different references and both have to move.
#[test]
fn scenario_human_facts_contributes_both_of_its_columns() {
    let cols: Vec<&str> = REFERENCING_COLUMNS
        .iter()
        .filter(|(t, _)| *t == "scenario_human_facts")
        .map(|(_, c)| *c)
        .collect();
    assert_eq!(cols.len(), 2);
    assert!(cols.contains(&"anchor_graph_node_id"));
    assert!(cols.contains(&"answers_graph_node_id"));
}

/// No column appears twice — a duplicate would double-count the proof and make a
/// sound run look like a mismatch.
#[test]
fn no_reference_is_listed_twice() {
    let mut seen: Vec<String> = REFERENCING_COLUMNS
        .iter()
        .map(|(t, c)| format!("{t}.{c}"))
        .collect();
    let total = seen.len();
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), total, "a referencing column is listed twice");
}

fn evidence(current_id: &str, page: i64, quote: &str) -> EvidenceRow {
    EvidenceRow {
        current_id: current_id.to_string(),
        doc_slug: "doc-a".to_string(),
        page: Some(page),
        verbatim_quote: quote.to_string(),
        question: None,
    }
}

/// An unsafe plan is refused BEFORE any document is touched.
///
/// Built by parking a refused twin on the id another node is planned to move to
/// — the only way to construct the case without brute-forcing an 8-hex digest
/// collision.
#[test]
fn an_unsafe_plan_is_refused_rather_than_executed() {
    let probe = RekeyPlan::build(vec![evidence("stale", 4, "Unique.")]);
    let landing = probe
        .nodes()
        .next()
        .and_then(PlannedNode::rekey_target)
        .expect("planned")
        .to_string();

    let err = plan_or_refuse(vec![
        evidence(&landing, 9, "Shared."),
        evidence("twin-2", 9, "Shared."),
        evidence("stale", 4, "Unique."),
    ])
    .expect_err("an unsafe plan must refuse");

    let message = err.to_string();
    assert!(message.contains("Nothing was written"), "{message}");
    assert!(
        message.contains(&landing),
        "the refusal must name the contested id: {message}",
    );
}

/// A safe plan passes the gate and comes back intact.
#[test]
fn a_safe_plan_is_returned_unchanged() {
    let plan = plan_or_refuse(vec![evidence("stale", 1, "A.")]).expect("a safe plan is accepted");
    assert_eq!(plan.totals().to_rekey, 1);
}

/// Twins alone are not "unsafe" — they are the expected refusal, and a corpus
/// that is nothing but twins must still plan cleanly (zero work, no error).
#[test]
fn twins_alone_do_not_make_a_plan_unsafe() {
    let plan = plan_or_refuse(vec![
        evidence("twin-1", 9, "Shared."),
        evidence("twin-2", 9, "Shared."),
    ])
    .expect("refused twins are a normal outcome, not an unsafe plan");
    assert_eq!(plan.totals().to_rekey, 0);
    assert_eq!(plan.totals().refused_shared_key, 2);
}
