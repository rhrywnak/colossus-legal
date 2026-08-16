//! The proof and the queue are the only records of what a `--apply` run did and
//! what it refused. These assert both say the right thing for the shapes measured
//! on DEV.

use super::*;
use crate::rekey::plan::EvidenceRow;
use crate::twinmerge::plan::{TwinNode, TwinPlan};

fn node(id: &str, curated_rows: u64, edges: &[&str]) -> TwinNode {
    TwinNode {
        row: EvidenceRow {
            current_id: id.to_string(),
            doc_slug: "doc-cfs-interrogatory-response-08-08-16".to_string(),
            page: Some(6),
            verbatim_quote: "Catholic Family Service is unaware of who prepared it.".to_string(),
            question: Some("Who prepared the document?".to_string()),
        },
        curated_rows,
        relationships: edges.iter().map(|s| s.to_string()).collect(),
    }
}

const EDGES: [&str; 1] = ["CONTAINED_IN->doc-cfs"];

#[test]
fn a_dry_run_says_so_and_shows_no_execution_section() {
    let plan = TwinPlan::build(vec![node("aaa", 0, &EDGES), node("bbb", 0, &EDGES)]);
    let rendered = RunReport::from_plan(&plan, false).render();

    assert!(rendered.contains("DRY RUN — nothing was written"));
    assert!(
        !rendered.contains("EXECUTION"),
        "a dry run must not print an execution section; there was no execution"
    );
    assert!(rendered.contains("Clusters to merge        : 1"));
    assert!(rendered.contains("Nodes to delete          : 1"));
}

#[test]
fn an_applied_run_reports_its_totals() {
    let plan = TwinPlan::build(vec![node("aaa", 0, &EDGES), node("bbb", 0, &EDGES)]);
    let mut report = RunReport::from_plan(&plan, true);
    report.clusters.push(ClusterProof {
        key: "doc-cfs:evidence:deadbeef".to_string(),
        survivor: "aaa".to_string(),
        losers: vec!["bbb".to_string()],
        tables: vec![TableProof {
            reference: "extraction_items.neo4j_node_id".to_string(),
            expected: 2,
            updated: 2,
        }],
        nodes_deleted: 1,
        edges_deleted: 1,
        aborted: None,
    });

    let rendered = report.render();
    assert!(rendered.contains("APPLIED — the database was written"));
    assert!(rendered.contains("Nodes deleted            : 1"));
    assert!(rendered.contains("Referencing rows updated : 2"));
    assert!(rendered.contains("survives: aaa"));
    assert!(rendered.contains("merged in: bbb"));
    assert_eq!(report.exit_code(), crate::oneshot::exit::EXIT_OK);
}

#[test]
fn a_mismatched_table_is_flagged_and_earns_exit_three() {
    let plan = TwinPlan::build(vec![node("aaa", 0, &EDGES), node("bbb", 0, &EDGES)]);
    let mut report = RunReport::from_plan(&plan, true);
    report.clusters.push(ClusterProof {
        key: "k".to_string(),
        survivor: "aaa".to_string(),
        losers: vec!["bbb".to_string()],
        tables: vec![TableProof {
            reference: "scenario_fact_refs.graph_node_id".to_string(),
            expected: 3,
            updated: 2,
        }],
        nodes_deleted: 0,
        edges_deleted: 0,
        aborted: Some("scenario_fact_refs.graph_node_id: expected 3, updated 2".to_string()),
    });

    let rendered = report.render();
    assert!(rendered.contains("<-- MISMATCH"));
    assert!(rendered.contains("[ABORTED — rolled back]"));
    assert_eq!(
        report.exit_code(),
        crate::oneshot::exit::EXIT_UNIT_ABORTED,
        "a rolled-back cluster must not hide under exit 0"
    );
}

#[test]
fn refusals_do_not_change_the_exit_code() {
    // Seven pairs are expected to be refused on every run until the merge
    // session happens. A non-zero code for that would train an operator to
    // ignore the code.
    let plan = TwinPlan::build(vec![node("aaa", 5, &EDGES), node("bbb", 7, &EDGES)]);
    let report = RunReport::from_plan(&plan, true);

    assert_eq!(report.queue.len(), 1);
    assert_eq!(report.exit_code(), crate::oneshot::exit::EXIT_OK);
}

#[test]
fn the_queue_names_every_member_and_its_row_count() {
    let plan = TwinPlan::build(vec![
        node("98515eda", 12, &EDGES),
        node("f1439b2c", 9, &EDGES),
    ]);
    let queue = RunReport::from_plan(&plan, false).render_queue();

    assert!(queue.contains("98515eda — 12 curated row(s)"));
    assert!(queue.contains("f1439b2c — 9 curated row(s)"));
    assert!(
        queue.contains("Catholic Family Service is unaware"),
        "the quote must be in the queue — a merge session should not have to go \
         look up what the statement says"
    );
    assert!(queue.contains("page 6"));
    assert!(queue.contains("only Roman can decide"));
}

#[test]
fn the_queue_names_the_edges_a_delete_would_lose() {
    let plan = TwinPlan::build(vec![
        node("aaa", 0, &["CONTAINED_IN->doc-cfs"]),
        node(
            "bbb",
            0,
            &["CONTAINED_IN->doc-cfs", "ABOUT->person-marie-awad"],
        ),
    ]);
    let queue = RunReport::from_plan(&plan, false).render_queue();

    assert!(queue.contains("aaa — proposed survivor"));
    assert!(queue.contains("ABOUT->person-marie-awad"));
}

#[test]
fn an_empty_queue_still_writes_a_file_that_says_it_is_empty() {
    // An absent file cannot distinguish "nothing was refused" from "the tool
    // died before writing it", and those need different responses.
    let plan = TwinPlan::build(vec![node("aaa", 0, &EDGES), node("bbb", 0, &EDGES)]);
    let queue = RunReport::from_plan(&plan, false).render_queue();

    assert!(queue.contains("No cluster was refused"));
    assert!(!queue.is_empty());
}

#[test]
fn two_renders_of_one_plan_are_byte_identical() {
    // Two dry runs of unchanged data must diff cleanly against each other.
    let plan = TwinPlan::build(vec![node("bbb", 0, &EDGES), node("aaa", 0, &EDGES)]);
    let a = RunReport::from_plan(&plan, false);
    let b = RunReport::from_plan(&plan, false);
    assert_eq!(a.render(), b.render());
    assert_eq!(a.render_queue(), b.render_queue());
}
