//! User listing and reviewer assignment endpoints.
//!
//! - `list_users_handler` is mounted on the main API router at `/api/users`
//!   because user listing is app-wide (not pipeline-specific).
//! - `assign_reviewer_handler` is mounted on the pipeline router at
//!   `/documents/:id/assign` because it modifies pipeline documents.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::pipeline_repository::users::{self, KnownUser};
use crate::state::AppState;

// ── Request / Response types ─────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AssignReviewerRequest {
    /// Username of the reviewer to assign. `None` (or JSON null) to unassign.
    pub reviewer: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AssignReviewerResponse {
    pub document_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_reviewer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_at: Option<chrono::DateTime<chrono::Utc>>,
}

// ── Handlers ─────────────────────────────────────────────────────

/// GET /api/users — list all known users for dropdown population.
///
/// Any authenticated user can call this (no admin check).
/// Returns users sorted by display_name.
pub async fn list_users_handler(
    _user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<KnownUser>>, AppError> {
    let known = users::list_known_users(&state.pipeline_pool)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to list users: {e}"),
        })?;
    Ok(Json(known))
}

/// PUT /documents/:id/assign — assign or unassign a reviewer.
///
/// Admin-only. Send `{"reviewer": "username"}` to assign,
/// or `{"reviewer": null}` to unassign.
pub async fn assign_reviewer_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    Json(body): Json<AssignReviewerRequest>,
) -> Result<Json<AssignReviewerResponse>, AppError> {
    require_admin(&user)?;

    users::assign_reviewer(&state.pipeline_pool, &document_id, body.reviewer.as_deref())
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to assign reviewer: {e}"),
        })?;

    // Read back the document to get the actual assigned_at timestamp.
    let doc =
        crate::repositories::pipeline_repository::get_document(&state.pipeline_pool, &document_id)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to read document: {e}"),
            })?
            .ok_or_else(|| AppError::Internal {
                message: format!("Document not found after update: {document_id}"),
            })?;

    Ok(Json(AssignReviewerResponse {
        document_id: doc.id,
        assigned_reviewer: doc.assigned_reviewer,
        assigned_at: doc.assigned_at,
    }))
}
