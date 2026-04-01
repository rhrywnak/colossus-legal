//! GET /api/admin/pipeline/documents/:id/items — list extraction items.
//! Supports pagination and optional filtering by review_status, entity_type,
//! and grounding_status. Returns items from the latest completed run.

use axum::{extract::Path, extract::Query, extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::pipeline_repository::{self, review as review_repo};
use crate::state::AppState;

use review_repo::ReviewItemRow;

#[derive(Debug, Deserialize)]
pub struct ListItemsParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub review_status: Option<String>,
    pub entity_type: Option<String>,
    pub grounding_status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListItemsResponse {
    pub document_id: String,
    pub items: Vec<ReviewItemRow>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

/// GET /api/admin/pipeline/documents/:id/items
pub async fn list_items_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
    Query(params): Query<ListItemsParams>,
) -> Result<Json<ListItemsResponse>, AppError> {
    require_admin(&user)?;

    // Find latest completed run for this document
    let run_id = pipeline_repository::get_latest_completed_run(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("No completed extraction run for document '{doc_id}'"),
        })?;

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).min(200);
    let offset = ((page - 1) * per_page) as i64;
    let limit = per_page as i64;

    let review_status = params.review_status.as_deref();
    let entity_type = params.entity_type.as_deref();
    let grounding_status = params.grounding_status.as_deref();

    let total = review_repo::count_items(
        &state.pipeline_pool, run_id, review_status, entity_type, grounding_status,
    )
    .await
    .map_err(|e| AppError::Internal { message: format!("Count query failed: {e}") })?;

    let items = review_repo::list_items(
        &state.pipeline_pool, run_id, review_status, entity_type, grounding_status, limit, offset,
    )
    .await
    .map_err(|e| AppError::Internal { message: format!("List query failed: {e}") })?;

    let total_pages = if total == 0 { 1 } else { (total as u32).div_ceil(per_page) };

    Ok(Json(ListItemsResponse {
        document_id: doc_id,
        items,
        total,
        page,
        per_page,
        total_pages,
    }))
}
