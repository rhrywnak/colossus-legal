//! Error tracking endpoint — surfaces documents with failed pipeline steps.

use axum::{extract::State, Json};
use serde::Serialize;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DocumentError {
    pub document_id: String,
    pub document_title: String,
    pub document_status: String,
    pub failed_step: String,
    pub error_message: Option<String>,
    pub failed_at: String,
    pub triggered_by: Option<String>,
    pub retry_count: i64,
}

#[derive(Debug, Serialize)]
pub struct ErrorsResponse {
    pub documents_with_errors: Vec<DocumentError>,
    pub total_errors: i64,
    pub documents_with_no_errors: i64,
}

/// GET /documents/errors — returns all documents with failed pipeline steps.
pub async fn errors_handler(
    _user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ErrorsResponse>, AppError> {
    let pool = &state.pipeline_pool;

    // Get documents with their most recent failed step
    let errors: Vec<DocumentError> = sqlx::query_as::<_, DocumentError>(
        r#"SELECT
            d.id AS document_id,
            d.title AS document_title,
            d.status AS document_status,
            ps.step_name AS failed_step,
            ps.error_message,
            ps.started_at::text AS failed_at,
            ps.triggered_by,
            (SELECT COUNT(*) FROM pipeline_steps ps2
             WHERE ps2.document_id = d.id AND ps2.step_name = ps.step_name
            ) AS retry_count
        FROM documents d
        JOIN pipeline_steps ps ON ps.document_id = d.id
        WHERE ps.status = 'failed'
          AND ps.id = (
              SELECT ps3.id FROM pipeline_steps ps3
              WHERE ps3.document_id = d.id AND ps3.status = 'failed'
              ORDER BY ps3.started_at DESC
              LIMIT 1
          )
        ORDER BY ps.started_at DESC"#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal { message: format!("Failed to query errors: {e}") })?;

    let total_errors = errors.len() as i64;

    let total_docs: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents")
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::Internal { message: format!("Failed to count documents: {e}") })?;

    Ok(Json(ErrorsResponse {
        documents_with_errors: errors,
        total_errors,
        documents_with_no_errors: total_docs.0 - total_errors,
    }))
}
