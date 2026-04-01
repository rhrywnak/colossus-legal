//! GET /api/admin/pipeline/documents/:id/history — execution history.

use axum::{extract::Path, extract::State, Json};
use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::pipeline_repository::steps::{self, PipelineStepRecord};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub document_id: String,
    pub steps: Vec<PipelineStepRecord>,
}

/// GET /api/admin/pipeline/documents/:id/history
pub async fn history_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(document_id): Path<String>,
) -> Result<Json<HistoryResponse>, AppError> {
    require_admin(&user)?;

    let steps = steps::get_steps_for_document(&state.pipeline_pool, &document_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("History query failed: {e}"),
        })?;

    Ok(Json(HistoryResponse {
        document_id,
        steps,
    }))
}
