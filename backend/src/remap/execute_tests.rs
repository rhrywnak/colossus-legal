//! The remap's count proof — the only record of what an approved `apply` did.
//!
//! `is_sound` is the sole gate between a committed transaction and a rollback,
//! and `render` is what an operator reads afterwards. Neither touches a store,
//! so both are tested here.

use super::*;
use crate::oneshot::refs::TableProof;

fn proof(reference: &str, expected: u64, updated: u64) -> TableProof {
    TableProof {
        reference: reference.to_string(),
        expected,
        updated,
    }
}

fn report(tables: Vec<TableProof>, aborted: Option<String>) -> ApplyReport {
    ApplyReport {
        document_id: "doc-sabrina-morris-affidavit".to_string(),
        approved_by: "Roman".to_string(),
        moves: 2,
        tables,
        aborted,
    }
}

#[test]
fn matching_counts_are_sound() {
    assert!(proof("scenario_fact_refs.graph_node_id", 3, 3).is_sound());
    assert!(proof("scenario_fact_refs.graph_node_id", 0, 0).is_sound());
}

#[test]
fn a_short_update_is_not_sound() {
    assert!(!proof("scenario_fact_refs.graph_node_id", 3, 2).is_sound());
}

#[test]
fn an_update_that_moved_more_rows_than_expected_is_also_not_sound() {
    // Equality, not "at least". A higher count means the UPDATE matched rows the
    // plan did not know about, which is as much a failure as missing them — and
    // it is the reason the abort is on `!=` rather than `<`.
    assert!(!proof("scenario_fact_refs.graph_node_id", 3, 4).is_sound());
}

#[test]
fn rows_updated_totals_every_column() {
    let r = report(
        vec![
            proof("scenario_fact_refs.graph_node_id", 3, 3),
            proof("scan_run_verdicts.graph_node_id", 7, 7),
        ],
        None,
    );
    assert_eq!(r.rows_updated(), 10);
}

#[test]
fn an_applied_report_names_the_document_and_who_approved_it() {
    // The approver's name is in the proof because "who authorised this" is a
    // question that gets asked about a write, days later, by someone reading
    // only the file.
    let rendered = report(vec![proof("scenario_fact_refs.graph_node_id", 3, 3)], None).render();

    assert!(rendered.contains("EVIDENCE REMAP — APPLIED"));
    assert!(rendered.contains("doc-sabrina-morris-affidavit"));
    assert!(rendered.contains("approved by   : Roman"));
    assert!(rendered.contains("moves applied : 2"));
    assert!(!rendered.contains("MISMATCH"));
}

#[test]
fn an_aborted_report_says_so_in_its_header_and_gives_the_reason() {
    let rendered = report(
        vec![proof("scenario_fact_refs.graph_node_id", 3, 2)],
        Some("scenario_fact_refs.graph_node_id: expected 3, updated 2".to_string()),
    )
    .render();

    assert!(rendered.contains("ABORTED — rolled back"));
    assert!(rendered.contains("expected 3, updated 2"));
    assert!(
        rendered.contains("<-- MISMATCH"),
        "the offending line must be findable without reading the header twice"
    );
}

#[test]
fn a_column_that_moved_nothing_is_still_proved_rather_than_omitted() {
    // A zero is a claim; an absent line is a silence, and the two must not look
    // the same to someone checking the run.
    let rendered = report(
        vec![
            proof("scenario_fact_refs.graph_node_id", 0, 0),
            proof("scan_run_verdicts.graph_node_id", 4, 4),
        ],
        None,
    )
    .render();

    assert!(rendered.contains("scenario_fact_refs.graph_node_id"));
    assert!(rendered.contains("expected    0  updated    0"));
}
