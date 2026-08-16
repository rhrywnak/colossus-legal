//! The parts of `execute` that touch no store: the plan gate, and the error
//! messages an operator reads when it refuses.
//!
//! `plan_or_refuse` is the whole safety gate — it is what turns a dangerous plan
//! into exit code 4 before a single row moves — and it is pure, so it is tested
//! here rather than trusted.

use super::*;
use crate::rekey::plan::EvidenceRow;
use crate::twinmerge::plan::TwinNode;

fn twin(id: &str, curated_rows: u64) -> TwinNode {
    TwinNode {
        row: EvidenceRow {
            current_id: id.to_string(),
            doc_slug: "doc-cfs-interrogatory-response-08-08-16".to_string(),
            page: Some(6),
            verbatim_quote: "Catholic Family Service is unaware of who prepared it.".to_string(),
            question: Some("Who prepared the document?".to_string()),
        },
        curated_rows,
        relationships: vec!["CONTAINED_IN->doc-cfs".to_string()],
    }
}

#[test]
fn a_safe_plan_is_returned_intact() {
    let plan = plan_or_refuse(vec![twin("aaa", 0), twin("bbb", 0)]).expect("a safe plan");
    assert_eq!(plan.clusters.len(), 1);
    assert_eq!(plan.totals().clusters_to_merge, 1);
}

#[test]
fn a_plan_whose_target_is_already_held_is_refused_before_anything_moves() {
    // The birthday case, and the half-finished-earlier-run case. A node outside
    // the cluster already carrying the target id would be welded to this
    // statement — detectable before writing, so it is detected.
    let target = crate::api::pipeline::evidence_key::evidence_id(
        "doc-cfs-interrogatory-response-08-08-16",
        Some(6),
        "Catholic Family Service is unaware of who prepared it.",
        Some("Who prepared the document?"),
    );
    // A third node that is NOT a twin (different quote → different key) but
    // happens to already carry the id the merge would assign.
    let mut outsider = twin(&target, 0);
    outsider.row.verbatim_quote = "An unrelated statement entirely.".to_string();

    let error =
        plan_or_refuse(vec![twin("aaa", 0), twin("bbb", 0), outsider]).expect_err("must refuse");

    match &error {
        TwinMergeError::UnsafePlan { count, first } => {
            assert_eq!(*count, 1);
            assert!(first.contains(&target), "got: {first}");
        }
        other => panic!("expected UnsafePlan, got {other:?}"),
    }
    let message = error.to_string();
    assert!(
        message.contains("Nothing was written"),
        "the operator reads this on exit 4 and needs to know the corpus is \
         untouched: {message}"
    );
    assert!(message.contains("unsafe"), "got: {message}");
}

#[test]
fn no_clusters_is_a_safe_plan_and_not_an_error() {
    // After the merge session has run, every key has one holder. That must read
    // as "nothing to do", not as a failure.
    let plan = plan_or_refuse(vec![twin("aaa", 0)]).expect("a safe plan");
    assert_eq!(plan.clusters.len(), 0);
    assert_eq!(plan.totals().clusters_to_merge, 0);
}

#[test]
fn the_invariant_error_says_it_is_a_bug_and_not_a_data_problem() {
    // The defensive arm in `apply_cluster` used to borrow `UnsafePlan`, whose
    // message would have sent an operator hunting for two nodes sharing an id.
    // This one has to point at the code instead.
    let message = TwinMergeError::InvariantViolated {
        what: "apply_cluster reached a non-Merge disposition for cluster k".to_string(),
    }
    .to_string();

    assert!(message.contains("BUG"), "got: {message}");
    assert!(message.contains("Nothing was written"), "got: {message}");
    assert!(
        message.contains("code fix"),
        "the operator must be told this is not theirs to investigate: {message}"
    );
    assert!(message.contains("non-Merge disposition"), "got: {message}");
}

#[test]
fn every_store_error_names_the_operation_that_failed() {
    // Standing Rule 1: a reader of the logs must be able to tell WHAT failed.
    let neo4j = TwinMergeError::Neo4jDecode {
        operation: "load_rows_with_edges",
        source: neo4rs::DeError::PropertyMissingButRequired,
    };
    assert!(neo4j.to_string().contains("load_rows_with_edges"));

    let postgres = TwinMergeError::Postgres {
        operation: "count_rows",
        source: sqlx::Error::RowNotFound,
    };
    assert!(postgres.to_string().contains("count_rows"));
}
