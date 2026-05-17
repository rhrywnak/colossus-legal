//! Restate workflow step: bulk-approve grounded extraction items.
//!
//! Wraps the clean
//! [`run_auto_approve`](crate::pipeline::steps::auto_approve::run_auto_approve)
//! orchestrator with the Restate error classification. Per the P2-2c
//! design decision (option b), this step does NOT write
//! `documents.status` — the lifecycle column stays at `"VERIFIED"`
//! until `step_ingest` writes `"INGESTED"`. The auto-approve outcome
//! is observable via review/UI counters, not via the document's
//! primary lifecycle status.
//!
//! ## Idempotency
//!
//! `bulk_approve` filters by PENDING items only, so re-running on
//! an already-auto-approved document yields `approved_count = 0`.
//! Restate replay is safe; no explicit short-circuit guard needed.

use std::sync::Arc;

use restate_sdk::errors::{HandlerError, TerminalError};

use super::{record_step_lifecycle, StepOutcome, STEP_AUTO_APPROVE};
use crate::pipeline::context::AppContext;
use crate::pipeline::steps::auto_approve::{run_auto_approve, AutoApproveError};

/// Restate workflow step: bulk-approve grounded extraction items.
///
/// Delegates to the clean
/// [`run_auto_approve`](crate::pipeline::steps::auto_approve::run_auto_approve),
/// then formats a summary string for the Restate journal.
///
/// ## No `documents.status` write (design decision)
///
/// Per the P2-2c option-b decision (matching the pass-2 precedent
/// from Refactor 3/3), this handler does NOT write
/// `documents.status`. The lifecycle column stays at `"VERIFIED"`
/// (written by the prior `step_verify`) until `step_ingest` writes
/// `"INGESTED"` in P2-2c Part 2. The auto-approve outcome is
/// observable via review/UI counters
/// (`extraction_items.review_status`), not via the document's
/// primary lifecycle status — no canonical `STATUS_APPROVED`
/// arm exists in the frontend's `compute_status_group` routing
/// table, and adding one solely for this brief AutoApprove → Ingest
/// window would be churn for no operator benefit.
///
/// ## Error classification
///
/// All [`AutoApproveError`] variants route through
/// [`classify_auto_approve_error`]:
/// - `DocumentNotFound` is terminal (the row is gone; retrying
///   won't bring it back).
/// - `BulkApproveFailed`, `CountPendingFailed`, `Helper` are
///   retryable transient DB failures — Restate's backoff likely
///   resolves them.
#[tracing::instrument(skip(app), fields(doc_id = %doc_id, step = STEP_AUTO_APPROVE))]
pub async fn step_auto_approve(
    app: &Arc<AppContext>,
    doc_id: &str,
) -> Result<String, HandlerError> {
    record_step_lifecycle(
        &app.pipeline_pool,
        doc_id,
        STEP_AUTO_APPROVE,
        step_auto_approve_body(app, doc_id),
    )
    .await
}

/// Body of [`step_auto_approve`]. Returns the 2-key audit JSON
/// (`approved` ← `approved_count`, `pending_review` ←
/// `pending_review_count`) matching the legacy
/// `progress.set_step_result(...)` shape at
/// `pipeline/steps/auto_approve.rs:143`.
#[tracing::instrument(skip(app), fields(doc_id = %doc_id))]
async fn step_auto_approve_body(
    app: &Arc<AppContext>,
    doc_id: &str,
) -> Result<StepOutcome, HandlerError> {
    let result = run_auto_approve(doc_id, &app.pipeline_pool, app.as_ref())
        .await
        .map_err(|e| classify_auto_approve_error(doc_id, &e))?;

    let summary = format!(
        "auto_approve_complete approved={} pending_review={}",
        result.approved_count, result.pending_review_count,
    );
    tracing::info!(
        doc_id = %doc_id,
        approved_count = result.approved_count,
        pending_review_count = result.pending_review_count,
        "step_auto_approve: complete"
    );
    // Audit JSON shape matches `pipeline/steps/auto_approve.rs:143`.
    // See [`build_result_summary`] for the rename contract.
    Ok(StepOutcome {
        summary,
        result_summary: build_result_summary(&result),
        skipped_early: false,
    })
}

/// Build the 2-key `result_summary` JSON for auto_approve, matching
/// `pipeline/steps/auto_approve.rs:143` byte-for-byte. The legacy
/// code renames the struct fields (`approved_count → approved`,
/// `pending_review_count → pending_review`) so we do the same to
/// keep the column byte-identical. Extracted for testability — a
/// future struct-field rename without an audit-trail migration
/// would silently break tooling that reads
/// `pipeline_steps.result_summary` directly.
fn build_result_summary(
    result: &crate::pipeline::steps::auto_approve::AutoApproveResult,
) -> serde_json::Value {
    serde_json::json!({
        "approved": result.approved_count,
        "pending_review": result.pending_review_count,
    })
}

/// Classify an [`AutoApproveError`] as terminal or retryable.
///
/// Only `DocumentNotFound` is terminal (the document row is gone —
/// retrying can't recover it). All three transient DB-failure
/// variants are retryable.
fn classify_auto_approve_error(doc_id: &str, e: &AutoApproveError) -> HandlerError {
    use AutoApproveError as E;
    match e {
        E::DocumentNotFound { .. } => TerminalError::new(format!(
            "step_auto_approve: document '{doc_id}' not found in database. \
             Confirm the upload completed before invoking the workflow."
        ))
        .into(),
        E::BulkApproveFailed { .. } => HandlerError::from(format!(
            "step_auto_approve: transient bulk_approve failure for '{doc_id}': \
             {e}. Will retry."
        )),
        E::CountPendingFailed { .. } => HandlerError::from(format!(
            "step_auto_approve: transient count_pending failure for '{doc_id}': \
             {e}. Will retry."
        )),
        E::Helper { .. } => HandlerError::from(format!(
            "step_auto_approve: transient helper failure for '{doc_id}': {e}. \
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

    // ── `build_result_summary` rename contract ─────────────────

    #[test]
    fn build_result_summary_renames_struct_fields() {
        let result = crate::pipeline::steps::auto_approve::AutoApproveResult {
            approved_count: 17,
            pending_review_count: 4,
        };
        let summary = super::build_result_summary(&result);
        // Renamed keys.
        assert_eq!(summary["approved"], serde_json::json!(17));
        assert_eq!(summary["pending_review"], serde_json::json!(4));
        // The struct field names must NOT appear in the JSON.
        assert!(
            summary.get("approved_count").is_none(),
            "approved_count must be renamed to approved"
        );
        assert!(
            summary.get("pending_review_count").is_none(),
            "pending_review_count must be renamed to pending_review"
        );
        let obj = summary
            .as_object()
            .expect("result_summary must be a JSON object");
        assert_eq!(obj.len(), 2, "result_summary must have exactly 2 keys");
    }

    #[test]
    fn classify_document_not_found_is_terminal() {
        let err = AutoApproveError::DocumentNotFound {
            doc_id: "doc-x".into(),
        };
        let c = classify_auto_approve_error("doc-x", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(msg.contains("doc-x"));
        assert!(msg.contains("upload completed"));
    }

    #[test]
    fn classify_bulk_approve_failed_is_retryable() {
        let err = AutoApproveError::BulkApproveFailed {
            doc_id: "doc-x".into(),
            source: sqlx::Error::Configuration("connection refused".into()),
        };
        let c = classify_auto_approve_error("doc-x", &err);
        assert!(!is_terminal(&c));
        let msg = display_message(&c);
        assert!(msg.contains("Will retry"));
    }

    #[test]
    fn classify_count_pending_failed_is_retryable() {
        let err = AutoApproveError::CountPendingFailed {
            doc_id: "doc-x".into(),
            source: sqlx::Error::Configuration("connection refused".into()),
        };
        let c = classify_auto_approve_error("doc-x", &err);
        assert!(!is_terminal(&c));
    }

    #[test]
    fn classify_helper_is_retryable() {
        let err = AutoApproveError::Helper {
            doc_id: "doc-x".into(),
            message: "pool exhausted".into(),
        };
        let c = classify_auto_approve_error("doc-x", &err);
        assert!(!is_terminal(&c));
    }
}
