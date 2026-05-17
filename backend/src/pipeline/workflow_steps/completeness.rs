//! Restate workflow step: terminal completeness verification.
//!
//! Wraps the clean
//! [`run_completeness`](crate::pipeline::steps::completeness::run_completeness)
//! orchestrator with the Restate error classification. The
//! `documents.status = "PUBLISHED"` write happens inside the
//! orchestrator (not here) — both legacy and Restate paths see the
//! canonical terminal-status surface via the core function.
//!
//! ## Idempotency
//!
//! Completeness is naturally idempotent — it reads Neo4j + Qdrant +
//! Postgres and writes one row (the status update, which converges
//! on the same value). Restate replay re-runs the verification and
//! reaches the same end state. No explicit guard needed.

use std::sync::Arc;

use restate_sdk::errors::{HandlerError, TerminalError};

use super::{record_step_lifecycle, StepOutcome, STEP_COMPLETENESS};
use crate::pipeline::context::AppContext;
use crate::pipeline::steps::completeness::{run_completeness, CompletenessError};

/// Restate workflow step: terminal completeness verification.
///
/// Delegates to the clean
/// [`run_completeness`](crate::pipeline::steps::completeness::run_completeness)
/// and returns a short summary string suitable for journaling. The
/// status transition to `"PUBLISHED"` is written inside the
/// orchestrator — this handler does not duplicate it.
#[tracing::instrument(skip(app), fields(doc_id = %doc_id, step = STEP_COMPLETENESS))]
pub async fn step_completeness(
    app: &Arc<AppContext>,
    doc_id: &str,
) -> Result<String, HandlerError> {
    record_step_lifecycle(
        &app.pipeline_pool,
        doc_id,
        STEP_COMPLETENESS,
        step_completeness_body(app, doc_id),
    )
    .await
}

/// Body of [`step_completeness`]. Returns the 4-key audit JSON
/// (`total_items`, `nodes_verified`, `points_verified`,
/// `points_missing`) matching `pipeline/steps/completeness.rs:195`.
#[tracing::instrument(skip(app), fields(doc_id = %doc_id))]
async fn step_completeness_body(
    app: &Arc<AppContext>,
    doc_id: &str,
) -> Result<StepOutcome, HandlerError> {
    let result = run_completeness(doc_id, &app.pipeline_pool, app.as_ref())
        .await
        .map_err(|e| classify_completeness_error(doc_id, &e))?;

    let summary = format!(
        "completeness_complete total_items={} nodes_verified={} \
         points_verified={} points_missing={}",
        result.total_items, result.nodes_verified, result.points_verified, result.points_missing,
    );
    tracing::info!(
        doc_id = %doc_id,
        total_items = result.total_items,
        nodes_verified = result.nodes_verified,
        points_verified = result.points_verified,
        points_missing = result.points_missing,
        "step_completeness: complete — document PUBLISHED"
    );
    // Audit JSON shape matches `pipeline/steps/completeness.rs:195`
    // — see [`build_result_summary`] for the 4-key mapping.
    Ok(StepOutcome {
        summary,
        result_summary: build_result_summary(&result),
        skipped_early: false,
    })
}

/// Build the 4-key `result_summary` JSON for completeness, matching
/// `pipeline/steps/completeness.rs:195` byte-for-byte. Same field
/// names as `CompletenessResult` (no rename) — extracted for
/// testability so a future struct-field rename without an audit
/// migration is caught at test time.
fn build_result_summary(
    result: &crate::pipeline::steps::completeness::CompletenessResult,
) -> serde_json::Value {
    serde_json::json!({
        "total_items": result.total_items,
        "nodes_verified": result.nodes_verified,
        "points_verified": result.points_verified,
        "points_missing": result.points_missing,
    })
}

/// Classify a [`CompletenessError`] as terminal or retryable.
///
/// Missing data-state (no run, no document node, missing nodes) is
/// terminal — operator must investigate the upstream Ingest step
/// before completeness can succeed. Transient helper failures
/// (Postgres timeouts, Neo4j connectivity blips) are retryable.
fn classify_completeness_error(doc_id: &str, e: &CompletenessError) -> HandlerError {
    use CompletenessError as E;
    match e {
        E::DocumentNotFound { .. } => TerminalError::new(format!(
            "step_completeness: document '{doc_id}' not found in database. \
             Confirm the upload completed before invoking the workflow."
        ))
        .into(),
        E::NoCompletedRun { .. } => TerminalError::new(format!(
            "step_completeness: no COMPLETED extraction_run for '{doc_id}'. \
             Pass-1 (and pass-2 if configured) must succeed before \
             completeness can verify entity nodes."
        ))
        .into(),
        E::MissingDocumentNode { .. } => TerminalError::new(format!(
            "step_completeness: Document node missing in Neo4j for '{doc_id}'. \
             Ingest did not produce the Document node — investigate the \
             ingest step's log output before retrying."
        ))
        .into(),
        E::MissingNodes { .. } => TerminalError::new(format!(
            "step_completeness: {e}. Missing entity nodes indicate an Ingest \
             gap — re-run Ingest after investigating which expected ids did \
             not land."
        ))
        .into(),
        E::Helper { .. } => HandlerError::from(format!(
            "step_completeness: transient helper failure for '{doc_id}': {e}. \
             Will retry."
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn display_message(e: &HandlerError) -> String {
        let inner: &dyn std::error::Error = e.as_ref();
        format!("{inner}")
    }

    fn is_terminal(e: &HandlerError) -> bool {
        display_message(e).starts_with("Terminal error")
    }

    // ── `build_result_summary` shape contract ──────────────────

    #[test]
    fn build_result_summary_emits_4_keys_direct_mapping() {
        let result = crate::pipeline::steps::completeness::CompletenessResult {
            total_items: 50,
            nodes_verified: 47,
            points_verified: 47,
            points_missing: 3,
        };
        let summary = super::build_result_summary(&result);
        assert_eq!(summary["total_items"], serde_json::json!(50));
        assert_eq!(summary["nodes_verified"], serde_json::json!(47));
        assert_eq!(summary["points_verified"], serde_json::json!(47));
        assert_eq!(summary["points_missing"], serde_json::json!(3));
        let obj = summary
            .as_object()
            .expect("result_summary must be a JSON object");
        assert_eq!(
            obj.len(),
            4,
            "result_summary must contain exactly 4 keys, got {obj:?}"
        );
    }

    #[test]
    fn classify_document_not_found_is_terminal() {
        let err = CompletenessError::DocumentNotFound {
            doc_id: "doc-x".into(),
        };
        let c = classify_completeness_error("doc-x", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(msg.contains("doc-x"));
        assert!(msg.contains("upload completed"));
    }

    #[test]
    fn classify_no_completed_run_is_terminal() {
        let err = CompletenessError::NoCompletedRun {
            doc_id: "doc-x".into(),
        };
        let c = classify_completeness_error("doc-x", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(msg.contains("Pass-1"));
    }

    #[test]
    fn classify_missing_document_node_is_terminal() {
        let err = CompletenessError::MissingDocumentNode {
            doc_id: "doc-x".into(),
        };
        let c = classify_completeness_error("doc-x", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(msg.contains("ingest"));
    }

    #[test]
    fn classify_missing_nodes_is_terminal() {
        let err = CompletenessError::MissingNodes {
            doc_id: "doc-x".into(),
            missing_count: 3,
            total: 41,
            ids: "person-a, person-b, organization-c".into(),
        };
        let c = classify_completeness_error("doc-x", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        // The Display impl of MissingNodes is included via `{e}` —
        // confirm the missing-id list survives into the operator
        // message.
        assert!(
            msg.contains("person-a"),
            "msg must surface missing ids: {msg}"
        );
        assert!(msg.contains("re-run Ingest"));
    }

    #[test]
    fn classify_helper_is_retryable() {
        let err = CompletenessError::Helper {
            doc_id: "doc-x".into(),
            message: "neo4j connection refused".into(),
        };
        let c = classify_completeness_error("doc-x", &err);
        assert!(!is_terminal(&c));
        let msg = display_message(&c);
        assert!(msg.contains("Will retry"));
    }
}
