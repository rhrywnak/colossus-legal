// Tests for `rekey::execute`.
//
// The write path itself needs two live stores and is exercised from the runbook's
// dry run against DEV. What IS unit-testable is the part that decides whether a
// write may happen at all — the column list every count and update walks, and the
// pre-flight refusal — and those are the two places a defect would be silent.

use super::*;
use crate::oneshot::refs::{ReferencingColumn, REKEY_UPDATES_EVERYTHING};

/// The re-key walks the SHARED registry, not a list of its own.
///
/// Ruled 2026-08-16. It previously carried a private `REFERENCING_COLUMNS` of the
/// eight columns Phase A had measured as populated, and missed three — one of
/// which held 483 rows it was actively invalidating. The list moved to
/// `oneshot::refs::EVIDENCE_REFERENCES`, which every tool in the family reads, so
/// "the re-key's list" and "the registry" are now the same object and cannot
/// drift apart.
///
/// The registry's own membership is pinned in `oneshot::refs_tests` against the
/// dated `information_schema` sweep. What is asserted HERE is the property this
/// tool depends on: that it is walking all of it.
#[test]
fn the_rekey_walks_every_column_in_the_shared_registry() {
    assert_eq!(
        EVIDENCE_REFERENCES.len(),
        11,
        "the re-key updates every column in the registry; if the registry \
         changed size, re-measure and update the runbook's expected row count"
    );
    assert!(REKEY_UPDATES_EVERYTHING);
}

/// The three columns the eight-column version missed, named individually.
#[test]
fn the_columns_added_on_the_sixteenth_are_still_there() {
    for (table, column) in [
        ("extraction_items", "neo4j_node_id"),
        ("evidence_summary_overrides", "graph_node_id"),
        ("response_item_fact_refs", "graph_node_id"),
    ] {
        assert!(
            EVIDENCE_REFERENCES.contains(&ReferencingColumn { table, column }),
            "{table}.{column} was added to the re-key on 2026-08-16 and is gone again"
        );
    }
}

/// `scenario_human_facts` contributes TWO columns — an anchor and an answer are
/// different references and both have to move.
#[test]
fn scenario_human_facts_contributes_both_of_its_columns() {
    let cols: Vec<&str> = EVIDENCE_REFERENCES
        .iter()
        .filter(|c| c.table == "scenario_human_facts")
        .map(|c| c.column)
        .collect();
    assert_eq!(cols.len(), 2);
    assert!(cols.contains(&"anchor_graph_node_id"));
    assert!(cols.contains(&"answers_graph_node_id"));
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
