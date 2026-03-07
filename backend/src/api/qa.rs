//! Q&A history endpoints — browse, view, and rate persisted QAEntries.
//!
//! These endpoints expose the shared research notebook. Any authenticated
//! user can read all entries (by design — it's a collaborative tool).

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::embed::ErrorResponse;
use crate::auth::AuthUser;
use crate::repositories::qa_repository::{self, QAEntry, QAEntrySummary, QAError};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Query / request types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct QAHistoryParams {
    pub scope_type: String,
    pub scope_id: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct RateRequest {
    pub rating: String,
}

#[derive(Debug, Serialize)]
pub struct SuccessResponse {
    pub success: bool,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

type ApiError = (StatusCode, Json<ErrorResponse>);

/// GET /api/qa-history?scope_type=case&scope_id=awad-v-cfs-2011&limit=50
///
/// Returns all QAEntry summaries for a scope, newest first.
/// Any authenticated user can access.
pub async fn get_qa_history(
    user: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<QAHistoryParams>,
) -> Result<Json<Vec<QAEntrySummary>>, ApiError> {
    tracing::info!("{} GET /api/qa-history", user.username);

    let limit = params.limit.unwrap_or(50).min(200);

    let entries = qa_repository::get_qa_history(
        &state.graph,
        &params.scope_type,
        &params.scope_id,
        limit,
    )
    .await
    .map_err(map_qa_error)?;

    Ok(Json(entries))
}

/// GET /api/qa/:id
///
/// Returns a single QAEntry with full answer and metadata.
pub async fn get_qa_entry(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<QAEntry>, ApiError> {
    tracing::info!("{} GET /api/qa/{}", user.username, id);

    let entry = qa_repository::get_qa_entry(&state.graph, &id)
        .await
        .map_err(map_qa_error)?
        .ok_or_else(|| error_response(StatusCode::NOT_FOUND, "QA entry not found"))?;

    Ok(Json(entry))
}

/// PATCH /api/qa/:id/rate
///
/// Rate a QA entry as "helpful" or "not_helpful".
/// Uses the authenticated user's username as rating_by.
pub async fn rate_qa_entry(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<RateRequest>,
) -> Result<Json<SuccessResponse>, ApiError> {
    tracing::info!("{} PATCH /api/qa/{}/rate", user.username, id);

    qa_repository::rate_qa_entry(&state.graph, &id, &body.rating, &user.username)
        .await
        .map_err(map_qa_error)?;

    Ok(Json(SuccessResponse { success: true }))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn error_response(status: StatusCode, message: &str) -> ApiError {
    (status, Json(ErrorResponse { error: message.to_string() }))
}

fn map_qa_error(e: QAError) -> ApiError {
    match &e {
        QAError::NotFound(_) => error_response(StatusCode::NOT_FOUND, &e.to_string()),
        QAError::InvalidRating(_) => error_response(StatusCode::BAD_REQUEST, &e.to_string()),
        QAError::Neo4j(_) => {
            tracing::error!("QA repository error: {e}");
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "database error")
        }
    }
}
