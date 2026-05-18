//! POST /api/admin/pipeline/documents/:id/cancel — dual-cancel handler.
//!
//! Lives alongside [`super::process`] but in its own file so each
//! module stays focused: `process.rs` owns the `/process` submission
//! path and the shared `ProcessResponse` DTO, while this module owns
//! the dual-cancel decision matrix and its internal helpers.
//!
//! ## Dual-cancel design
//!
//! Documents on the transition path may be processing on either the
//! legacy `colossus-pipeline` Worker OR the Restate workflow. The
//! Cancel button in the UI does not (and should not) know which
//! backend owns a given document, so this handler tries both and
//! treats success on either as a successful cancel for the caller.
//!
//! - Legacy cancel — `colossus_pipeline::Scheduler::cancel(job_id)`.
//!   Three possible outcomes ([`LegacyCancelOutcome`]):
//!   `Cancelled(job_id)`, `AlreadyTerminal`, `NoJob`.
//! - Restate cancel —
//!   [`crate::pipeline::workflow_admin::cancel_restate_workflow`].
//!   Three possible outcomes ([`RestateCancelOutcome`]):
//!   `Cancelled`, `NoInvocation`, `NotConfigured`.
//!
//! See the doc comment on [`cancel_handler`] for the outcome matrix.

use axum::{extract::Path, extract::State, Json};

use super::process::ProcessResponse;
use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::models::document_status::STATUS_CANCELLED;
use crate::pipeline::workflow_admin::cancel_restate_workflow;
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository;
use crate::state::AppState;

/// POST /api/admin/pipeline/documents/:id/cancel
///
/// Dual-cancel: tries BOTH the legacy `Scheduler::cancel` and the
/// Restate admin-API cancel, returning success if either path
/// reports a cancellation.
///
/// ## Outcome matrix
///
/// |  | legacy: cancelled | legacy: terminal | legacy: no job |
/// |--|------------------|------------------|----------------|
/// | **restate: cancelled (202)** | 200 (both) | 200 (restate only) | 200 (restate only) |
/// | **restate: 404** | 200 (legacy only) | **409** (already terminal everywhere) | **404** (nothing to cancel) |
/// | **restate: not configured** | 200 (legacy only) | **409** | **404** |
///
/// 5xx / network errors from Restate are propagated as
/// `AppError::Internal` regardless of the legacy outcome — operators
/// need to know when Restate is reachable but misbehaving, and
/// silently swallowing the error would hide a real outage.
///
/// On a successful Restate-side cancel the handler also writes
/// `STATUS_CANCELLED` to `documents.status` directly: the
/// `pipeline_jobs_sync_document_status` trigger that handles the
/// legacy path only fires on `pipeline_jobs` row changes, and a
/// Restate-only document has no such row.
pub async fn cancel_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Json<ProcessResponse>, AppError> {
    require_admin(&user)?;

    // [A] 404-if-no-document guard — disambiguates "typo'd document id"
    //     from "document exists but has no cancellable work."
    if pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!(
                "Failed to look up document '{doc_id}' in the documents table: {e}. \
                 Check PostgreSQL connectivity and the pipeline_pool configuration \
                 (PIPELINE_DATABASE_URL); a transient pool exhaustion will resolve \
                 on retry, a persistent failure needs operator intervention."
            ),
        })?
        .is_none()
    {
        return Err(AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        });
    }

    let legacy_outcome = try_legacy_cancel(&state, &doc_id).await?;
    let restate_outcome = try_restate_cancel(&state, &doc_id).await?;

    let legacy_cancelled = matches!(legacy_outcome, LegacyCancelOutcome::Cancelled(_));
    let restate_cancelled = matches!(restate_outcome, RestateCancelOutcome::Cancelled);

    // On Restate-side success the documents row must be updated
    // manually: the legacy trigger only projects pipeline_jobs
    // changes onto documents.status. Best-effort — a write failure
    // here leaves the document at "PROCESSING" until the next
    // status reconciliation, which is preferable to failing the
    // cancel itself.
    if restate_cancelled {
        if let Err(e) = pipeline_repository::update_document_status(
            &state.pipeline_pool,
            &doc_id,
            STATUS_CANCELLED,
        )
        .await
        {
            tracing::error!(
                doc_id = %doc_id, error = %e,
                "Restate cancel succeeded but writing STATUS_CANCELLED to documents failed \
                 (non-fatal — operator may need to refresh document status manually)"
            );
        }
    }

    // Flip `documents.is_cancelled = true` on either-path success.
    // This is the cooperative-cancellation signal read by the LLM
    // extraction chunk loop (`extract_chunks_loop` in
    // `pipeline/steps/llm_extract.rs`) and at the entry of
    // `run_pass2_extraction`. Restate's cancel signal does NOT
    // interrupt an in-flight `ctx.run()` closure (the SDK only
    // checks for cancellation at journal boundaries, not while a
    // closure is mid-flight on a multi-minute Opus call), so the
    // closure must poll for itself. The legacy `Scheduler::cancel`
    // path also benefits — it triggers `CancellationToken` aborts
    // for steps not currently inside the chunk loop, but the chunk
    // loop runs inside a single `tokio::select!` branch that the
    // token doesn't interrupt mid-iteration either.
    //
    // Best-effort: a failure to flip the flag is logged and
    // swallowed. The cancel itself has already succeeded at the
    // workflow / scheduler layer; the worst outcome is the
    // in-flight extractor keeps running until the abort timeout,
    // which is no worse than the pre-fix behaviour.
    if legacy_cancelled || restate_cancelled {
        if let Err(e) =
            pipeline_repository::documents::mark_document_cancelled(&state.pipeline_pool, &doc_id)
                .await
        {
            tracing::error!(
                doc_id = %doc_id, error = %e,
                legacy_cancelled, restate_cancelled,
                "Cancel succeeded at workflow layer but writing documents.is_cancelled=true failed \
                 (non-fatal — in-flight chunk loop will not short-circuit but the abort timeout \
                 will still terminate the invocation)"
            );
        }
    }

    // Decision matrix → either an OK response or an error variant.
    let job_id_string = match &legacy_outcome {
        LegacyCancelOutcome::Cancelled(id) => Some(id.to_string()),
        _ => None,
    };

    let audit_details = serde_json::json!({
        "legacy_cancelled": legacy_cancelled,
        "restate_cancelled": restate_cancelled,
        "job_id": job_id_string,
    });

    log_admin_action(
        &state.audit_repo,
        &user.username,
        "pipeline.document.cancel_requested",
        Some("document"),
        Some(&doc_id),
        Some(audit_details.clone()),
    )
    .await;

    tracing::info!(
        doc_id = %doc_id,
        legacy_cancelled,
        restate_cancelled,
        job_id = ?job_id_string,
        user = %user.username,
        "Dual-cancel attempted"
    );

    if legacy_cancelled || restate_cancelled {
        return Ok(Json(ProcessResponse {
            document_id: doc_id,
            status: "CANCELLING".to_string(),
            message: cancel_success_message(legacy_cancelled, restate_cancelled),
            job_id: job_id_string,
        }));
    }

    // Neither path cancelled. Distinguish "nothing to cancel" (404)
    // from "everything already terminal" (409). `NotConfigured` is
    // grouped with `NoInvocation` for this decision because there's
    // nothing to cancel on a backend we can't talk to.
    let restate_terminal_like = matches!(
        restate_outcome,
        RestateCancelOutcome::NoInvocation | RestateCancelOutcome::NotConfigured
    );
    match legacy_outcome {
        LegacyCancelOutcome::AlreadyTerminal if restate_terminal_like => Err(AppError::Conflict {
            message: format!(
                "Document '{doc_id}' is already in a terminal state on both pipeline backends"
            ),
            details: audit_details,
        }),
        _ => Err(AppError::NotFound {
            message: format!(
                "No active pipeline job or Restate invocation found to cancel for '{doc_id}'"
            ),
        }),
    }
}

// ── Dual-cancel internals ───────────────────────────────────────

/// Outcome of the legacy `Scheduler::cancel` attempt.
///
/// Used by [`cancel_handler`] to feed the dual-cancel decision matrix
/// without re-discriminating on a `Result<Option<_>>` shape. The
/// three variants map 1:1 onto the three columns of the matrix in
/// the handler's doc comment.
#[derive(Debug)]
enum LegacyCancelOutcome {
    /// `Scheduler::cancel` returned Ok — the worker has been asked to
    /// stop or the Ready job's row has been flipped. Carries the
    /// `pipeline_jobs.id` for audit-log purposes.
    Cancelled(uuid::Uuid),
    /// `Scheduler::cancel` returned `JobNotCancellable` — the job is
    /// already in a terminal state (`completed` / `failed` /
    /// `cancelled`). Distinct from `NoJob` so the dual-cancel matrix
    /// can distinguish "already done" from "never existed."
    AlreadyTerminal,
    /// No `pipeline_jobs` row exists for `(doc_id, document_processing)`.
    /// The legacy path has nothing to cancel.
    NoJob,
}

#[tracing::instrument(skip(state), fields(doc_id = %doc_id))]
async fn try_legacy_cancel(
    state: &AppState,
    doc_id: &str,
) -> Result<LegacyCancelOutcome, AppError> {
    let scheduler = colossus_pipeline::Scheduler::new(&state.pipeline_pool);

    let job_opt = scheduler
        .status_by_key(
            crate::pipeline::constants::JOB_TYPE_DOCUMENT_PROCESSING,
            doc_id,
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to look up active legacy job for '{doc_id}': {e}"),
        })?;

    let Some(job) = job_opt else {
        tracing::info!(doc_id = %doc_id, "Legacy cancel: no pipeline_jobs row");
        return Ok(LegacyCancelOutcome::NoJob);
    };

    match scheduler.cancel(job.id).await {
        Ok(()) => {
            tracing::info!(
                doc_id = %doc_id, job_id = %job.id,
                "Legacy cancel: pipeline_jobs row cancelled"
            );
            Ok(LegacyCancelOutcome::Cancelled(job.id))
        }
        Err(colossus_pipeline::PipelineError::JobNotCancellable(_)) => {
            // No longer a hard 409 — under dual-cancel this is one of
            // two signals the matrix combines with the Restate outcome.
            tracing::info!(
                doc_id = %doc_id, job_id = %job.id,
                "Legacy cancel: job already terminal — falling through to Restate"
            );
            Ok(LegacyCancelOutcome::AlreadyTerminal)
        }
        Err(other) => Err(AppError::Internal {
            message: format!("Legacy cancel for '{doc_id}' failed: {other}"),
        }),
    }
}

/// Outcome of the Restate admin-API cancel attempt.
#[derive(Debug)]
enum RestateCancelOutcome {
    /// Restate returned 202 — the workflow invocation has been signalled
    /// to cancel.
    Cancelled,
    /// Restate returned 404 — no invocation exists for this document
    /// (either never ran on Restate or already terminal).
    NoInvocation,
    /// `restate_admin_url` is unset, so the Restate cancel call was
    /// skipped without contacting Restate. Treated like `NoInvocation`
    /// for the purposes of the 404/409 decision matrix but logged
    /// distinctly so deployments mid-rollout can audit the silent-skip
    /// branch via the audit log.
    NotConfigured,
}

#[tracing::instrument(skip(state), fields(doc_id = %doc_id))]
async fn try_restate_cancel(
    state: &AppState,
    doc_id: &str,
) -> Result<RestateCancelOutcome, AppError> {
    let Some(admin_url) = state.config.restate_admin_url.as_deref() else {
        tracing::info!(
            doc_id = %doc_id,
            "Restate cancel: RESTATE_ADMIN_URL not configured, skipping"
        );
        return Ok(RestateCancelOutcome::NotConfigured);
    };

    match cancel_restate_workflow(&state.http_client, admin_url, doc_id).await {
        Ok(true) => Ok(RestateCancelOutcome::Cancelled),
        Ok(false) => Ok(RestateCancelOutcome::NoInvocation),
        Err(e) => {
            // Network errors and unexpected status codes propagate as
            // Internal so an operator notices Restate is sick. The dual-
            // cancel outcome on the legacy side is preserved in the
            // tracing::error! below so the post-mortem can correlate.
            tracing::error!(
                doc_id = %doc_id, error = %e,
                "Restate cancel call failed unexpectedly"
            );
            Err(AppError::Internal {
                message: format!("Restate cancel for '{doc_id}' failed: {e}"),
            })
        }
    }
}

/// Build the user-facing message for a successful dual-cancel response.
///
/// Three success shapes: both backends cancelled, legacy only, or
/// Restate only. The message names which path(s) acted so an operator
/// reading the response (and the audit-log row whose
/// `legacy_cancelled` / `restate_cancelled` flags it mirrors) can
/// cross-reference without parsing logs.
fn cancel_success_message(legacy_cancelled: bool, restate_cancelled: bool) -> String {
    match (legacy_cancelled, restate_cancelled) {
        (true, true) => "Cancel requested on both legacy pipeline and Restate workflow. \
             Document will transition to CANCELLED when both workers acknowledge."
            .to_string(),
        (true, false) => "Cancel requested on legacy pipeline. \
             Document will transition to CANCELLED when the worker acknowledges."
            .to_string(),
        (false, true) => "Restate workflow cancelled. Document marked CANCELLED.".to_string(),
        // The caller never reaches this branch — the success path is
        // gated on `legacy_cancelled || restate_cancelled`. Kept here
        // so the match is exhaustive and a future refactor can't reach
        // this function with both flags false without a compiler error.
        (false, false) => "Cancel requested (no cancellable work found).".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_success_message_names_legacy_only_path() {
        let msg = cancel_success_message(true, false);
        assert!(
            msg.contains("legacy pipeline"),
            "legacy-only message must name the legacy pipeline: {msg}"
        );
        assert!(
            !msg.contains("Restate"),
            "legacy-only message must NOT claim Restate cancelled: {msg}"
        );
    }

    #[test]
    fn cancel_success_message_names_restate_only_path() {
        let msg = cancel_success_message(false, true);
        assert!(
            msg.contains("Restate"),
            "restate-only message must name Restate: {msg}"
        );
        assert!(
            !msg.contains("legacy"),
            "restate-only message must NOT claim legacy cancelled: {msg}"
        );
    }

    #[test]
    fn cancel_success_message_names_both_paths() {
        let msg = cancel_success_message(true, true);
        assert!(
            msg.contains("legacy"),
            "both-cancelled message must name legacy: {msg}"
        );
        assert!(
            msg.contains("Restate"),
            "both-cancelled message must name Restate: {msg}"
        );
    }
}
