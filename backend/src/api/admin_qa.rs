//! Admin endpoints for QA entry management.
//!
//! Lists all entries across users and supports bulk deletion.
//! The existing `DELETE /api/qa/:id` handles single deletes.

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::qa_repository::{self, QAEntrySummary};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ListParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    /// Optional filter by username.
    pub user: Option<String>,
}

fn default_limit() -> i64 {
    50
}

#[derive(Debug, Serialize)]
pub struct ListEntriesResponse {
    pub entries: Vec<QAEntrySummary>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Deserialize)]
pub struct BulkDeleteRequest {
    /// Delete ALL entries (nuclear option).
    #[serde(default)]
    pub all: bool,
    /// Specific entry IDs to delete.
    #[serde(default)]
    pub ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BulkDeleteResponse {
    pub deleted: u64,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// GET /api/admin/qa-entries?limit=50&offset=0&user=roman
///
/// Lists all QA entries across all users. Optional user filter.
pub async fn list_all_entries(
    user: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<ListEntriesResponse>, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, "GET /api/admin/qa-entries");

    let limit = params.limit.min(200);
    let (entries, total) = qa_repository::get_all_qa_entries(
        &state.pg_pool,
        limit,
        params.offset,
        params.user.as_deref(),
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to list QA entries: {e}"),
    })?;

    Ok(Json(ListEntriesResponse {
        entries,
        total,
        limit,
        offset: params.offset,
    }))
}

/// DELETE /api/admin/qa-entries
///
/// Body: `{ "ids": ["uuid-1", "uuid-2"] }` or `{ "all": true }`
pub async fn bulk_delete_entries(
    user: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<BulkDeleteRequest>,
) -> Result<Json<BulkDeleteResponse>, AppError> {
    require_admin(&user)?;

    if req.all {
        let deleted = qa_repository::delete_all_qa_entries(&state.pg_pool)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to delete all QA entries: {e}"),
            })?;
        tracing::warn!(user = %user.username, deleted, "Admin deleted ALL QA entries");
        log_admin_action(
            &state.audit_repo,
            &user.username,
            "qa.delete_all",
            Some("qa_entry"),
            None,
            Some(json!({ "count": deleted })),
        )
        .await;
        return Ok(Json(BulkDeleteResponse { deleted }));
    }

    if req.ids.is_empty() {
        return Err(AppError::BadRequest {
            message: "Provide 'ids' array or 'all: true'".to_string(),
            details: json!({}),
        });
    }

    // Parse string UUIDs
    let uuids: Vec<uuid::Uuid> = req
        .ids
        .iter()
        .map(|s| {
            uuid::Uuid::parse_str(s).map_err(|e| AppError::BadRequest {
                message: format!("Invalid UUID '{s}': {e}"),
                details: json!({ "id": s }),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let deleted = qa_repository::bulk_delete_qa_entries(&state.pg_pool, &uuids)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to bulk delete QA entries: {e}"),
        })?;

    tracing::warn!(
        user = %user.username,
        deleted,
        requested = req.ids.len(),
        "Admin bulk deleted QA entries"
    );

    log_admin_action(
        &state.audit_repo,
        &user.username,
        "qa.bulk_delete",
        Some("qa_entry"),
        None,
        Some(json!({ "count": req.ids.len(), "ids": &req.ids })),
    )
    .await;

    Ok(Json(BulkDeleteResponse { deleted }))
}
