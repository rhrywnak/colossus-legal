//! The proof is the only record of what a `--apply` run did to the People page.

use super::*;
use crate::partymerge::census::PartyNode;
use crate::partymerge::plan::MergePlan;
use crate::partymerge::rulings::parse;

fn party(id: &str, label: &str, name: &str, statements: u64) -> PartyNode {
    PartyNode {
        id: id.to_string(),
        label: label.to_string(),
        display_name: name.to_string(),
        statement_count: statements,
        source_documents: Vec::new(),
        aliases: Vec::new(),
    }
}

fn tighe_plan() -> MergePlan {
    let rulings = parse("CLUSTER Tighe\nSURVIVOR person-karen-a-tighe\nMERGE person-tighe\nEND\n")
        .expect("valid rulings");
    let census = vec![
        party("person-karen-a-tighe", "Person", "Karen A. Tighe", 39),
        party("person-tighe", "Person", "Tighe", 62),
    ];
    MergePlan::build(&rulings, &census).expect("a buildable plan")
}

fn conserved_proof() -> ClusterProof {
    ClusterProof {
        label: "Tighe".to_string(),
        survivor: "person-karen-a-tighe".to_string(),
        merged_in: vec!["person-tighe".to_string()],
        statements_expected: 101,
        statements_after: 101,
        nodes_deleted: 1,
        edges_repointed: 62,
        aliases_added: vec!["Tighe".to_string()],
        tables: vec![TableProof {
            reference: "extraction_items.neo4j_node_id".to_string(),
            expected: 4,
            updated: 4,
        }],
        aborted: None,
    }
}

#[test]
fn a_dry_run_says_so_and_shows_no_execution_section() {
    let rendered = RunReport::from_plan(&tighe_plan(), false).render();
    assert!(rendered.contains("DRY RUN — nothing was written"));
    assert!(!rendered.contains("EXECUTION"));
    assert!(rendered.contains("Nodes merging in         : 1"));
    assert!(rendered.contains("Statements moving        : 62"));
}

#[test]
fn the_plan_section_names_every_ruling_including_the_skips() {
    // A merge session's record is as much what was left alone as what was
    // merged; a report that only listed merges would lose half the session.
    let rulings = parse(
        "CLUSTER Tighe\nSURVIVOR person-karen-a-tighe\nMERGE person-tighe\nEND\n\
         CLUSTER Humphrey\nSKIP Jeff is ambiguous\nEND\n",
    )
    .expect("valid rulings");
    let census = vec![
        party("person-karen-a-tighe", "Person", "Karen A. Tighe", 39),
        party("person-tighe", "Person", "Tighe", 62),
    ];
    let plan = MergePlan::build(&rulings, &census).expect("a buildable plan");
    let rendered = RunReport::from_plan(&plan, false).render();

    assert!(rendered.contains("Tighe — merge 1 node(s) into person-karen-a-tighe"));
    assert!(rendered.contains("101 statement(s) expected after"));
    assert!(rendered.contains("Humphrey — SKIPPED — Jeff is ambiguous"));
}

#[test]
fn a_conserved_merge_reports_its_hundred_and_one_and_exits_zero() {
    let mut report = RunReport::from_plan(&tighe_plan(), true);
    report.clusters.push(conserved_proof());
    let rendered = report.render();

    assert!(rendered.contains("APPLIED — the graph was written"));
    assert!(rendered.contains("statements: expected 101 · after 101"));
    assert!(!rendered.contains("STATEMENTS LOST"));
    assert!(rendered.contains("Clusters conserving statements : 1/1"));
    assert!(rendered.contains("aliases recorded on the survivor: Tighe"));
    assert_eq!(report.exit_code(), crate::oneshot::exit::EXIT_OK);
}

#[test]
fn a_cluster_that_lost_statements_is_flagged_and_earns_exit_three() {
    let mut report = RunReport::from_plan(&tighe_plan(), true);
    report.clusters.push(ClusterProof {
        statements_after: 62,
        aborted: Some("statements: expected 101, found 62".to_string()),
        ..conserved_proof()
    });
    let rendered = report.render();

    assert!(rendered.contains("<-- STATEMENTS LOST"));
    assert!(rendered.contains("[ABORTED — rolled back]"));
    assert!(rendered.contains("Clusters conserving statements : 0/1"));
    assert_eq!(
        report.exit_code(),
        crate::oneshot::exit::EXIT_UNIT_ABORTED,
        "a merge that dropped a judge's testimony must not hide under exit 0"
    );
}

#[test]
fn a_referencing_table_mismatch_is_flagged() {
    let mut report = RunReport::from_plan(&tighe_plan(), true);
    report.clusters.push(ClusterProof {
        tables: vec![TableProof {
            reference: "extraction_items.neo4j_node_id".to_string(),
            expected: 4,
            updated: 3,
        }],
        aborted: Some("extraction_items.neo4j_node_id: expected 4, updated 3".to_string()),
        ..conserved_proof()
    });
    assert!(report.render().contains("<-- MISMATCH"));
}

#[test]
fn an_already_merged_cluster_reads_as_a_no_op_not_a_failure() {
    let rulings = parse("CLUSTER Tighe\nSURVIVOR person-karen-a-tighe\nMERGE person-gone\nEND\n")
        .expect("valid rulings");
    let census = vec![party(
        "person-karen-a-tighe",
        "Person",
        "Karen A. Tighe",
        101,
    )];
    let plan = MergePlan::build(&rulings, &census).expect("a buildable plan");
    let rendered = RunReport::from_plan(&plan, false).render();

    assert!(rendered.contains("Already merged (no-op)   : 1"));
    assert!(rendered.contains("already merged into person-karen-a-tighe on an earlier run"));
}

#[test]
fn two_renders_of_one_plan_are_byte_identical() {
    let plan = tighe_plan();
    assert_eq!(
        RunReport::from_plan(&plan, false).render(),
        RunReport::from_plan(&plan, false).render()
    );
}

// ── failure_reason: the three ways a cluster aborts ──────────────────────────
//
// Moved here from `execute` because it is a DECISION, and decisions live where a
// test can reach them. The three conditions are not interchangeable and an
// operator reads exactly these strings.

#[test]
fn a_conserved_cluster_with_matching_counts_has_no_failure() {
    assert_eq!(conserved_proof().failure_reason(), None);
}

#[test]
fn a_postgres_count_mismatch_is_reported_first_and_names_the_column() {
    // Checked before the statement count on purpose: it is the cheapest to
    // interpret and the most likely to be a real data problem.
    let proof = ClusterProof {
        tables: vec![TableProof {
            reference: "extraction_items.neo4j_node_id".to_string(),
            expected: 4,
            updated: 3,
        }],
        statements_after: 62,
        ..conserved_proof()
    };
    let reason = proof.failure_reason().expect("must abort");
    assert_eq!(
        reason,
        "extraction_items.neo4j_node_id: expected 4, updated 3"
    );
}

#[test]
fn a_lost_statement_aborts_the_cluster_and_names_both_numbers() {
    // The one that matters most: a merge that silently dropped 39 of a judge's
    // sworn statements would leave one tidy node and nothing saying so.
    let proof = ClusterProof {
        statements_after: 62,
        ..conserved_proof()
    };
    let reason = proof.failure_reason().expect("must abort");
    assert!(reason.contains("expected 101"), "got: {reason}");
    assert!(reason.contains("found 62"), "got: {reason}");
}

#[test]
fn a_delete_that_removed_the_wrong_number_of_nodes_aborts() {
    let proof = ClusterProof {
        nodes_deleted: 0,
        ..conserved_proof()
    };
    let reason = proof.failure_reason().expect("must abort");
    assert!(reason.contains("nodes deleted"), "got: {reason}");
    assert!(reason.contains("expected 1"), "got: {reason}");
}

#[test]
fn deleting_more_nodes_than_named_also_aborts() {
    // Not just "too few". A delete that took a node nobody named is worse.
    let proof = ClusterProof {
        nodes_deleted: 2,
        ..conserved_proof()
    };
    assert!(proof.failure_reason().is_some());
}
