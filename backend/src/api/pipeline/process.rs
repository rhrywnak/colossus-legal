//! POST /api/admin/pipeline/documents/:id/process
//!
//! Invokes the Restate `DocumentPipeline` workflow for the given
//! document. The workflow is keyed by document id and runs the 8-step
//! pipeline (extract_text → llm_extract_pass1 → llm_extract_pass2 →
//! verify → auto_approve → ingest → index → completeness) inside
//! Restate; each step writes its own `pipeline_steps` audit row via
//! the Restate handlers' lifecycle wrapping (P2-PRE-4).
//!
//! The companion `/cancel` endpoint lives in
//! [`super::cancel`](crate::api::pipeline::cancel) — split out for
//! module-size compliance and to keep the dual-cancel decision matrix
//! focused on its own concerns.
//!
//! ## documents.status transitions on this path
//!
//! Restate-driven processing does NOT create a `pipeline_jobs` row, so
//! the `pipeline_jobs_sync_document_status` trigger (migration
//! 20260422112238) — which projects `pipeline_jobs.status` onto
//! `documents.status` — never fires for new invocations. The
//! transitions are driven directly:
//!
//! 1. **process_handler** writes `STATUS_PROCESSING` here, BEFORE
//!    invoking Restate. If this write fails, the handler returns 500
//!    and Restate is NOT invoked — the frontend polls for
//!    `status_group == "processing"`, and a missed write would strand
//!    the UI at `"new"` while the workflow ran invisibly.
//! 2. **Each Restate step handler** writes the post-step status
//!    (`STATUS_TEXT_EXTRACTED`, `STATUS_EXTRACTED`, …,
//!    `STATUS_PUBLISHED`) directly to `documents` from inside its
//!    `ctx.run` closure.
//! 3. **The workflow failure path** in `pipeline::workflow::run`
//!    writes `STATUS_FAILED` on terminal step errors.
//!
//! The trigger still fires for legacy `pipeline_jobs` rows — both
//! historical rows from the pre-P2 worker path and any future rows
//! that the cancel handler's dual-path support continues to operate
//! on. The failed-job cleanup below removes `'failed'` legacy rows so
//! their trigger fire (`status='failed'` → `documents.status='FAILED'`)
//! does not regress a fresh `STATUS_PROCESSING` write.
//!
//! ## Why not dual-submit?
//!
//! The legacy `colossus_pipeline::Scheduler::submit(...)` call was
//! removed in P2-3 (this commit). The Worker still polls
//! `pipeline_jobs` for legacy-path documents already in flight, but
//! new submissions go exclusively through Restate. Dual-submit was
//! considered and rejected: two parallel runs of the same document
//! would double-spend LLM budget and race on Neo4j/Qdrant writes for
//! no operator benefit during the transition.

use axum::{extract::Path, extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::models::document_status::STATUS_PROCESSING;
use crate::pipeline::workflow_admin::{invoke_restate_workflow, InvokeOutcome};
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository;
use crate::state::AppState;

// ── Response DTO ────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessResponse {
    pub document_id: String,
    pub status: String,
    pub message: String,
    /// Under Restate-driven processing this carries the Restate
    /// invocation id (`inv_…`). Field name preserved from the legacy
    /// Worker era when it held a `pipeline_jobs.id` UUID; the JSON
    /// wire shape stays the same so the frontend's
    /// `ProcessResponse` typing (`pipelineApi.ts`) does not need
    /// updating.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
}

// ── Handler ─────────────────────────────────────────────────────

pub async fn process_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Json<ProcessResponse>, AppError> {
    require_admin(&user)?;

    // [1] Document existence guard.
    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!(
                "Failed to look up document '{doc_id}' in the documents table: {e}. \
                 Check PostgreSQL connectivity and the pipeline_pool configuration \
                 (PIPELINE_DATABASE_URL); a transient pool exhaustion will resolve \
                 on retry, a persistent failure needs operator intervention."
            ),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    // Capture the prior status BEFORE the processing guard so it's
    // available later for the best-effort revert if Restate invocation
    // fails after we've already written STATUS_PROCESSING.
    let previous_status = document.status.clone();

    // [2] Processing-state guard. A second click while processing is
    // a no-op for safety; the operator must Cancel first.
    let status_group =
        crate::api::pipeline::document_response::compute_status_group(&document.status);
    if status_group == "processing" {
        return Err(AppError::Conflict {
            message: format!(
                "Document '{doc_id}' is already processing. Cancel it first if you want to re-process."
            ),
            details: serde_json::json!({ "status_group": status_group }),
        });
    }

    // [3] Free the partial unique-index slot before any new run. The
    // pipeline_jobs unique index excludes only 'completed' and
    // 'cancelled' rows, so a stale 'failed' row from a prior legacy
    // attempt would trigger a sync-down to documents.status='FAILED'
    // on the trigger's next fire AND, more importantly, would
    // resurface as "active" in any /history queries that still scan
    // the legacy table. We keep this cleanup even under Restate-only
    // because legacy rows DO still exist from pre-P2 documents.
    if let Err(e) = sqlx::query(
        "DELETE FROM pipeline_jobs WHERE job_type = $1 AND job_key = $2 AND status = 'failed'",
    )
    .bind(crate::pipeline::constants::JOB_TYPE_DOCUMENT_PROCESSING)
    .bind(&doc_id)
    .execute(&state.pipeline_pool)
    .await
    {
        return Err(AppError::Internal {
            message: format!(
                "Failed to clean up prior failed pipeline_jobs row for '{doc_id}': {e}. \
                 The legacy partial unique index would block a fresh run; check \
                 PostgreSQL connectivity before retrying."
            ),
        });
    }

    // [4] Restate ingress must be configured. Unlike the cancel path
    // (which silently skips when RESTATE_ADMIN_URL is unset), the
    // process path REQUIRES the ingress URL — Restate-driven
    // processing is the only supported path under P2-3. A missing
    // env var is an operator-actionable misconfiguration, surfaced as
    // 503 Service Unavailable so the operator distinguishes "we
    // forgot to set RESTATE_INGRESS_URL" from "the request itself
    // was malformed."
    let ingress_url = state.config.restate_ingress_url.as_deref().ok_or_else(|| {
        AppError::ServiceUnavailable {
            message: "Restate ingress not configured: set RESTATE_INGRESS_URL in the backend \
                 environment. Document processing is unavailable until this is set."
                .to_string(),
        }
    })?;

    // [5] STATUS_PROCESSING write. MUST succeed before Restate
    // invocation: the frontend polls for status_group == "processing"
    // and would otherwise be stranded at "new" while Restate ran the
    // workflow invisibly. Hard error (500), not fire-and-log — failing
    // this write means the operator can't see progress, which is
    // worse than refusing the request.
    pipeline_repository::update_document_status(&state.pipeline_pool, &doc_id, STATUS_PROCESSING)
        .await
        .map_err(|e| AppError::Internal {
            message: format!(
                "Failed to set document '{doc_id}' status to PROCESSING: {e}. \
                 Check PostgreSQL connectivity; Restate workflow was not invoked."
            ),
        })?;

    // [6] Invoke Restate. On any failure here, best-effort revert
    // documents.status to the value we observed at [1] — leaving a
    // failed-to-invoke document stuck at PROCESSING would mislead the
    // frontend into polling forever.
    let outcome = match invoke_restate_workflow(&state.http_client, ingress_url, &doc_id).await {
        Ok(outcome) => outcome,
        Err(invoke_err) => {
            revert_status_best_effort(&state, &doc_id, &previous_status).await;
            return Err(AppError::Internal {
                message: format!(
                    "Restate invocation failed for '{doc_id}': {invoke_err}. \
                     Document status reverted to '{previous_status}'."
                ),
            });
        }
    };

    let (invocation_id, restate_status) = match outcome {
        InvokeOutcome::Accepted { invocation_id } => (invocation_id, "Accepted"),
        InvokeOutcome::PreviouslyAccepted { invocation_id } => {
            // 409 Conflict. The keyed Restate invocation for this
            // doc_id already exists — operator must delete the
            // document and re-upload, or purge the existing Restate
            // invocation via the admin UI, before retrying. Revert
            // the STATUS_PROCESSING write since we are NOT taking
            // ownership of the document for a fresh run.
            revert_status_best_effort(&state, &doc_id, &previous_status).await;

            // Audit the rejected attempt to the DB (not just logs) so
            // an operator querying `admin_audit_log` can see that a
            // second process attempt was made and rejected for this
            // document. Without this row, the only trail of the
            // PreviouslyAccepted rejection is in container logs.
            log_admin_action(
                &state.audit_repo,
                &user.username,
                "pipeline.document.process_conflict",
                Some("document"),
                Some(&doc_id),
                Some(serde_json::json!({
                    "invocation_id": invocation_id,
                    "restate_status": "PreviouslyAccepted",
                    "previous_status": previous_status,
                })),
            )
            .await;

            return Err(AppError::Conflict {
                message: format!(
                    "Document '{doc_id}' already has a Restate workflow invocation \
                     ('{invocation_id}'). Delete the document and re-upload, or \
                     purge the existing Restate invocation, before re-processing."
                ),
                details: serde_json::json!({
                    "invocation_id": invocation_id,
                    "previous_status": previous_status,
                }),
            });
        }
    };

    // [7] Audit + tracing. Both records carry the invocation id under
    // the same field names the legacy path used (`job_id` in the
    // audit details JSON) so log-scraping tooling still finds the
    // value at a stable key.
    log_admin_action(
        &state.audit_repo,
        &user.username,
        "pipeline.document.process_submitted",
        Some("document"),
        Some(&doc_id),
        Some(serde_json::json!({
            "invocation_id": invocation_id,
            "restate_status": restate_status,
        })),
    )
    .await;

    tracing::info!(
        doc_id = %doc_id,
        invocation_id = %invocation_id,
        user = %user.username,
        "Restate workflow invoked"
    );

    Ok(Json(ProcessResponse {
        document_id: doc_id,
        status: STATUS_PROCESSING.to_string(),
        message: "Restate workflow invoked".to_string(),
        job_id: Some(invocation_id),
    }))
}

/// Best-effort revert of `documents.status` after a failed Restate
/// invocation.
///
/// Used by [`process_handler`] on the two failure paths after
/// `STATUS_PROCESSING` has already been written: a network/server
/// error talking to Restate, and the `PreviouslyAccepted` 409 case.
/// In both cases we did not take ownership of the document for a
/// fresh run, so leaving `documents.status = 'PROCESSING'` would
/// mislead the frontend into polling forever.
///
/// On its own DB failure this helper logs a `tracing::error!` and
/// returns silently — the caller is already returning an error to
/// the user, so failing the revert too would only add noise. The
/// operator sees both lines in the log and can manually correct the
/// stranded status from the admin UI.
async fn revert_status_best_effort(state: &AppState, doc_id: &str, previous_status: &str) {
    if let Err(e) =
        pipeline_repository::update_document_status(&state.pipeline_pool, doc_id, previous_status)
            .await
    {
        tracing::error!(
            doc_id = %doc_id,
            previous_status = %previous_status,
            error = %e,
            "Failed to revert documents.status after Restate invocation failure \
             (document is stranded at PROCESSING — operator must correct manually)"
        );
    }
}
