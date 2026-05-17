//! POST /api/admin/pipeline/documents/:id/process
//!
//! Submits a DocProcessing pipeline job to the Scheduler. The Worker
//! (spawned in main.rs) polls pipeline_jobs and executes the full step
//! sequence: ExtractText → LlmExtract → Ingest → Index → Completeness.
//!
//! The companion `/cancel` endpoint lives in
//! [`super::cancel`](crate::api::pipeline::cancel) — split out for
//! module-size compliance and to keep the dual-cancel decision
//! matrix focused on its own concerns.
//!
//! This is the Phase 5 replacement for the pre-P2-Cleanup in-line
//! orchestrator (extract.rs + process.rs, both deleted in commit
//! 1414838 on 2026-04-16). Since that deletion, the frontend has been
//! POSTing to an unregistered route; this handler restores the
//! endpoint with correct scheduler-submit semantics.
//!
//! ## Terminal-state self-heal
//!
//! `documents.status` is projected from `pipeline_jobs.status` by the
//! `pipeline_jobs_sync_document_status` trigger (migration
//! 20260422112238). So by the time the user clicks Re-process, the
//! document row reads `'FAILED'` / `'CANCELLED'` / `'PUBLISHED'` and
//! `compute_status_group` routes it out of the `processing` branch.
//!
//! Before re-submitting, this handler also deletes the prior `'failed'`
//! row from `pipeline_jobs` — the framework's partial unique index
//! `idx_pipeline_jobs_unique_active` excludes only `'completed'` and
//! `'cancelled'`, so a stale `'failed'` row would otherwise cause the
//! next `Scheduler::submit` to hit `DuplicateJob`. `pipeline_steps` rows
//! preserve the failure history for the Execution History panel.
//!
//! ## Known gaps (tracked as Phase 5b follow-ups)
//!
//! - UI per-step progress polls documents.processing_step (stale);
//!   worker writes pipeline_jobs.progress JSONB.

use axum::{extract::Path, extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::models::document_status::STATUS_PROCESSING;
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository;
use crate::state::AppState;

// ── Response DTO ────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessResponse {
    pub document_id: String,
    pub status: String,
    pub message: String,
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

    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

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

    // Free the partial unique-index slot before re-submitting. A stale
    // `'failed'` row survives the pipeline_jobs unique index and would
    // otherwise cause `Scheduler::submit` to return `DuplicateJob`. Only
    // `'failed'` needs deletion — `'completed'` and `'cancelled'` are
    // already excluded by the framework's partial WHERE clause.
    if let Err(e) = sqlx::query(
        "DELETE FROM pipeline_jobs WHERE job_type = $1 AND job_key = $2 AND status = 'failed'",
    )
    .bind(crate::pipeline::constants::JOB_TYPE_DOCUMENT_PROCESSING)
    .bind(&doc_id)
    .execute(&state.pipeline_pool)
    .await
    {
        return Err(AppError::Internal {
            message: format!("Failed to clean up prior failed pipeline_jobs row: {e}"),
        });
    }

    let scheduler = colossus_pipeline::Scheduler::new(&state.pipeline_pool);
    let initial_task = crate::pipeline::task::DocProcessing::ExtractText(
        crate::pipeline::steps::extract_text::ExtractText {
            document_id: doc_id.clone(),
        },
    );

    let job_id = match scheduler
        .submit(
            crate::pipeline::constants::JOB_TYPE_DOCUMENT_PROCESSING,
            &doc_id,
            initial_task,
            crate::pipeline::constants::PRIORITY_DEFAULT,
            Some(&user.username),
        )
        .await
    {
        Ok(id) => id,
        Err(colossus_pipeline::PipelineError::DuplicateJob { .. }) => {
            return Err(AppError::Conflict {
                message: format!("An active pipeline job already exists for '{doc_id}'"),
                details: serde_json::json!({ "document_id": doc_id }),
            });
        }
        Err(e) => {
            return Err(AppError::Internal {
                message: format!("Failed to submit pipeline job: {e}"),
            });
        }
    };

    if let Err(e) = pipeline_repository::update_document_status(
        &state.pipeline_pool,
        &doc_id,
        STATUS_PROCESSING,
    )
    .await
    {
        tracing::error!(
            doc_id = %doc_id,
            job_id = %job_id,
            error = %e,
            "Failed to update documents.status after successful job submit; worker will correct on terminal state"
        );
    }

    log_admin_action(
        &state.audit_repo,
        &user.username,
        "pipeline.document.process_submitted",
        Some("document"),
        Some(&doc_id),
        Some(serde_json::json!({ "job_id": job_id })),
    )
    .await;

    tracing::info!(
        doc_id = %doc_id,
        job_id = %job_id,
        user = %user.username,
        "Pipeline job submitted"
    );

    Ok(Json(ProcessResponse {
        document_id: doc_id,
        status: STATUS_PROCESSING.to_string(),
        message: "Pipeline job submitted".to_string(),
        job_id: Some(job_id.to_string()),
    }))
}
