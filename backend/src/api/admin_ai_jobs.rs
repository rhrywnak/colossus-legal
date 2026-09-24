//! `GET /api/admin/ai-jobs`, `PUT /api/admin/ai-jobs/:job` and `POST
//! /api/admin/ai-jobs/:job/switch-back` — Admin → Overview's "Which model and which
//! instructions each AI job uses" panel (CC_TASK_MODEL_JOBS_PANEL_v1, Stage A).
//!
//! Administrator-only. Every value arrives ready to show; the page composes
//! nothing and decides nothing (Standing Rule 12).

use axum::{
    extract::{Path, State},
    Json,
};

use crate::api::settings::settings_error_to_app_error;
use crate::auth::{require_admin, AuthUser};
use crate::dto::ai_jobs::{AiJobSaveRequest, AiJobSavedDto, AiJobsPanelDto};
use crate::error::AppError;
use crate::services::ai_jobs::panel;
use crate::services::ai_jobs_write::{save, switch_back, AiJobsWriteError};
use crate::state::AppState;

/// GET /api/admin/ai-jobs
pub async fn get_ai_jobs(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<AiJobsPanelDto>, AppError> {
    require_admin(&user)?;
    panel(&state).await.map(Json).map_err(|e| {
        let message = format!("could not read the AI jobs panel: {e}");
        tracing::error!(by = %user.username, %message, "jobs panel: read failed");
        AppError::Internal { message }
    })
}

/// `PUT /api/admin/ai-jobs/:job` — Save (board 3).
///
/// Every value goes through the Settings page's own guarded write
/// (`settings_write::set_setting`); see `services::ai_jobs_write`.
pub async fn put_ai_job(
    user: AuthUser,
    State(state): State<AppState>,
    Path(job): Path<String>,
    Json(request): Json<AiJobSaveRequest>,
) -> Result<Json<AiJobSavedDto>, AppError> {
    require_admin(&user)?;
    save(&state, &job, &request.changes, &user.username)
        .await
        .map(Json)
        .map_err(|e| write_error(e, &job, &user.username))
}

/// `POST /api/admin/ai-jobs/:job/switch-back` — restore what the job used
/// before its last change, in one click.
pub async fn post_switch_back(
    user: AuthUser,
    State(state): State<AppState>,
    Path(job): Path<String>,
) -> Result<Json<AiJobSavedDto>, AppError> {
    require_admin(&user)?;
    switch_back(&state, &job, &user.username)
        .await
        .map(Json)
        .map_err(|e| write_error(e, &job, &user.username))
}

/// One status per failure, each logged with the job and who asked.
///
/// A refusal a person caused is a 409 whose message the page shows as it is; the
/// settings store's own refusals keep the statuses the Settings page gives them;
/// an unreadable store is a 500.
fn write_error(e: AiJobsWriteError, job: &str, username: &str) -> AppError {
    tracing::warn!(job, by = username, error = %e, "jobs panel: save refused or failed");
    match e {
        AiJobsWriteError::UnknownJob(_) => AppError::NotFound {
            message: e.to_string(),
        },
        AiJobsWriteError::Refused(message) | AiJobsWriteError::Partial(message) => {
            AppError::Conflict {
                message,
                details: serde_json::json!({ "reason": "ai_job_refused" }),
            }
        }
        AiJobsWriteError::Settings(inner) => settings_error_to_app_error(inner),
        AiJobsWriteError::Read(_) | AiJobsWriteError::Panel(_) => {
            tracing::error!(job, by = username, error = %e, "jobs panel: save could not read the store");
            AppError::Internal {
                message: e.to_string(),
            }
        }
    }
}
