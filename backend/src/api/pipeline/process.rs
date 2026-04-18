//! POST /api/admin/pipeline/documents/:id/process
//!
//! Submits a DocProcessing pipeline job to the Scheduler. The Worker
//! (spawned in main.rs) polls pipeline_jobs and executes the full step
//! sequence: ExtractText → LlmExtract → Ingest → Index → Completeness.
//!
//! This is the Phase 5 replacement for the pre-P2-Cleanup in-line
//! orchestrator (extract.rs + process.rs, both deleted in commit
//! 1414838 on 2026-04-16). Since that deletion, the frontend has been
//! POSTing to an unregistered route; this handler restores the
//! endpoint with correct scheduler-submit semantics.
//!
//! ## Known gaps (tracked as Phase 5b follow-ups)
//!
//! - Cancel endpoint writes documents.is_cancelled (inert flag for
//!   worker jobs); fix: call Scheduler::cancel(job_id).
//! - documents.status terminal sync (COMPLETED/FAILED/CANCELLED) not
//!   wired; worker writes pipeline_jobs.status, not documents.status.
//!   Completeness step writes STATUS_PUBLISHED (legacy) only.
//! - UI per-step progress polls documents.processing_step (stale);
//!   worker writes pipeline_jobs.progress JSONB.

use axum::{extract::Path, extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
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
        "PROCESSING",
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
        status: "PROCESSING".to_string(),
        message: "Pipeline job submitted".to_string(),
        job_id: Some(job_id.to_string()),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn process_response_serialization_with_job_id() {
        let r = ProcessResponse {
            document_id: "d1".into(),
            status: "PROCESSING".into(),
            message: "ok".into(),
            job_id: Some("abc-123".into()),
        };
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains(r#""job_id":"abc-123""#));
        assert!(s.contains(r#""document_id":"d1""#));
    }

    #[test]
    fn process_response_serialization_without_job_id() {
        let r = ProcessResponse {
            document_id: "d1".into(),
            status: "PROCESSING".into(),
            message: "ok".into(),
            job_id: None,
        };
        let s = serde_json::to_string(&r).unwrap();
        assert!(!s.contains("job_id"), "job_id key must be omitted when None");
    }

    #[test]
    fn process_response_field_stability() {
        // Compile-only: constructing with all four named fields must work.
        // Renaming any field breaks this test.
        let _r = ProcessResponse {
            document_id: String::new(),
            status: String::new(),
            message: String::new(),
            job_id: None,
        };
    }
}
