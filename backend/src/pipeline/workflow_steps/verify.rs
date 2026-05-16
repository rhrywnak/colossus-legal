//! Restate workflow step: canonical-text verification.
//!
//! Wraps the clean [`run_verify`](crate::pipeline::steps::verify::run_verify)
//! orchestrator with the Restate error classification and the
//! `documents.status = "VERIFIED"` Postgres write.
//!
//! ## Idempotency
//!
//! The orchestrator has no explicit short-circuit guard, but its
//! per-item `update_item_grounding` writes are idempotent at the DB
//! level (UPDATEs that converge to the same value). Restate replay
//! re-runs the verification linearly and reaches the same end state;
//! the cost is the redundant work, not correctness. Adding a guard
//! would require checking every item's `grounding_status` upstream —
//! more code and DB I/O than the redundant verify it would save.

use std::sync::Arc;

use restate_sdk::errors::{HandlerError, TerminalError};

use crate::models::document_status::STATUS_VERIFIED;
use crate::pipeline::context::AppContext;
use crate::pipeline::steps::verify::{run_verify, VerifyError};
use crate::repositories::pipeline_repository;

/// Restate workflow step: canonical-text verification.
///
/// Delegates to the clean
/// [`run_verify`](crate::pipeline::steps::verify::run_verify) and
/// then writes `documents.status = "VERIFIED"` on success. Returns a
/// short summary string suitable for journaling.
///
/// ## Error classification
///
/// All [`VerifyError`] variants route through
/// [`classify_verify_error`]:
///
/// - Configuration / data-state issues → terminal (won't fix on
///   retry without operator intervention).
/// - Transient DB failures → retryable (Restate's exponential backoff
///   likely resolves these).
#[tracing::instrument(skip(app), fields(doc_id = %doc_id, step = "verify"))]
pub async fn step_verify(app: &Arc<AppContext>, doc_id: &str) -> Result<String, HandlerError> {
    let result = run_verify(doc_id, &app.pipeline_pool, app.as_ref())
        .await
        .map_err(|e| classify_verify_error(doc_id, &e))?;

    // Postgres status write — mirrors the Restate state write the
    // workflow performs after this step. `STATUS_VERIFIED` is in
    // `compute_status_group`'s "processing" arm, so the frontend's
    // 3s poll loop keeps running.
    pipeline_repository::update_document_status(&app.pipeline_pool, doc_id, STATUS_VERIFIED)
        .await
        .map_err(|e| match e {
            pipeline_repository::PipelineRepoError::NotFound(_) => TerminalError::new(format!(
                "step_verify: documents row for '{doc_id}' disappeared while \
                 updating status. Cannot proceed; confirm the document still \
                 exists in the documents table."
            ))
            .into(),
            other => HandlerError::from(format!(
                "step_verify: failed to update status for '{doc_id}': {other}. \
                 Will retry."
            )),
        })?;

    let summary = format!(
        "verify_complete total={} exact={} normalized={} not_found={} \
         derived={} derived_invalid={} unverified={} missing_quote={} \
         grounding_pct={:.0}",
        result.total_items,
        result.exact,
        result.normalized,
        result.not_found,
        result.derived,
        result.derived_invalid,
        result.unverified,
        result.missing_quote,
        result.grounding_pct,
    );
    tracing::info!(
        doc_id = %doc_id,
        total_items = result.total_items,
        exact = result.exact,
        normalized = result.normalized,
        not_found = result.not_found,
        derived = result.derived,
        derived_invalid = result.derived_invalid,
        grounding_pct = result.grounding_pct,
        "step_verify: complete"
    );
    Ok(summary)
}

/// Classify a [`VerifyError`] as terminal or retryable for Restate.
///
/// Rule of thumb: anything the *next* retry can't change is
/// terminal. Missing document, missing canonical text, malformed
/// schema config — none resolve on the next attempt. Transient DB
/// errors are retryable.
fn classify_verify_error(doc_id: &str, e: &VerifyError) -> HandlerError {
    use VerifyError as E;
    match e {
        // ── Terminal: data-state / config issues ──────────────────
        E::DocumentNotFound { .. } => TerminalError::new(format!(
            "step_verify: document '{doc_id}' not found in database. \
             Confirm the upload completed before invoking the workflow."
        ))
        .into(),
        E::PdfNotFound { path, .. } => TerminalError::new(format!(
            "step_verify: PDF for document '{doc_id}' not present at '{path}'. \
             Check DOCUMENT_STORAGE_PATH and documents.file_path."
        ))
        .into(),
        E::NoCanonicalText { .. } => TerminalError::new(format!(
            "step_verify: no canonical text for document '{doc_id}'. \
             Re-run extract_text first — verify needs the document_text rows."
        ))
        .into(),
        E::GroundingModes { message, .. } => TerminalError::new(format!(
            "step_verify: grounding-config load failed for '{doc_id}': \
             {message}. Fix the schema YAML's grounding fields and redeploy."
        ))
        .into(),

        // ── Retryable: transient infrastructure ───────────────────
        E::Db { message, .. } => HandlerError::from(format!(
            "step_verify: transient DB failure for '{doc_id}': {message}. \
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

    #[test]
    fn classify_document_not_found_is_terminal() {
        let err = VerifyError::DocumentNotFound {
            doc_id: "doc-x".into(),
        };
        let c = classify_verify_error("doc-x", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(msg.contains("doc-x"));
        assert!(msg.contains("upload completed"));
    }

    #[test]
    fn classify_pdf_not_found_is_terminal() {
        let err = VerifyError::PdfNotFound {
            doc_id: "doc-x".into(),
            path: "/tmp/missing.pdf".into(),
        };
        let c = classify_verify_error("doc-x", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(msg.contains("/tmp/missing.pdf"));
        assert!(msg.contains("DOCUMENT_STORAGE_PATH"));
    }

    #[test]
    fn classify_no_canonical_text_is_terminal() {
        let err = VerifyError::NoCanonicalText {
            doc_id: "doc-x".into(),
        };
        let c = classify_verify_error("doc-x", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(msg.contains("extract_text"), "msg must point at fix: {msg}");
    }

    #[test]
    fn classify_grounding_modes_is_terminal() {
        let err = VerifyError::GroundingModes {
            doc_id: "doc-x".into(),
            message: "yaml parse error at line 12".into(),
        };
        let c = classify_verify_error("doc-x", &err);
        assert!(is_terminal(&c));
        let msg = display_message(&c);
        assert!(msg.contains("schema YAML"), "msg must point at fix: {msg}");
    }

    #[test]
    fn classify_db_is_retryable() {
        let err = VerifyError::Db {
            doc_id: "doc-x".into(),
            message: "connection refused".into(),
        };
        let c = classify_verify_error("doc-x", &err);
        assert!(!is_terminal(&c), "Db must be retryable: {c:?}");
        let msg = display_message(&c);
        assert!(msg.contains("Will retry"));
    }
}
