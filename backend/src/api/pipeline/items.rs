//! GET /api/admin/pipeline/documents/:id/items — list extraction items.
//! Supports pagination and optional filtering by review_status, entity_type,
//! and grounding_status. Returns items from the latest completed run.
//!
//! Each item includes `can_approve`, `can_reject`, `can_edit` flags so the
//! frontend never needs to check review_status to decide which buttons to show.
//! The response also includes a `summary` with counts by review_status.

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

/// An extraction item with computed action permission flags.
#[derive(Debug, Serialize)]
pub struct ReviewItemResponse {
    #[serde(flatten)]
    pub item: ReviewItemRow,
    /// Whether this item can be approved (only pending items).
    pub can_approve: bool,
    /// Whether this item can be rejected (only pending items).
    pub can_reject: bool,
    /// Whether this item can be edited (only pending items).
    pub can_edit: bool,
}

/// Summary counts for the review panel header.
#[derive(Debug, Serialize)]
pub struct ReviewSummary {
    pub pending: i64,
    pub approved: i64,
    pub rejected: i64,
    pub edited: i64,
    pub total: i64,
}

#[derive(Debug, Serialize)]
pub struct ListItemsResponse {
    pub document_id: String,
    pub items: Vec<ReviewItemResponse>,
    pub summary: ReviewSummary,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
    pub total_pages: u32,
}

/// Compute action permissions from review_status.
/// Only pending items can be approved, rejected, or edited.
fn compute_item_actions(review_status: &str) -> (bool, bool, bool) {
    let is_pending = review_status.eq_ignore_ascii_case("pending")
        || review_status.is_empty();
    (is_pending, is_pending, is_pending)
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

    // Compute summary counts (unfiltered — always show total picture)
    let total_all = review_repo::count_items(&state.pipeline_pool, run_id, None, None, None)
        .await
        .unwrap_or(0);
    let pending_count = review_repo::count_items(
        &state.pipeline_pool, run_id, Some("pending"), None, None,
    ).await.unwrap_or(0);
    let approved_count = review_repo::count_items(
        &state.pipeline_pool, run_id, Some("approved"), None, None,
    ).await.unwrap_or(0);
    let rejected_count = review_repo::count_items(
        &state.pipeline_pool, run_id, Some("rejected"), None, None,
    ).await.unwrap_or(0);
    let edited_count = review_repo::count_items(
        &state.pipeline_pool, run_id, Some("edited"), None, None,
    ).await.unwrap_or(0);

    // Enrich items with action permissions
    let enriched_items: Vec<ReviewItemResponse> = items.into_iter()
        .map(|item| {
            let (can_approve, can_reject, can_edit) = compute_item_actions(&item.review_status);
            ReviewItemResponse { item, can_approve, can_reject, can_edit }
        })
        .collect();

    let total_pages = if total == 0 { 1 } else { (total as u32).div_ceil(per_page) };

    Ok(Json(ListItemsResponse {
        document_id: doc_id,
        items: enriched_items,
        summary: ReviewSummary {
            pending: pending_count,
            approved: approved_count,
            rejected: rejected_count,
            edited: edited_count,
            total: total_all,
        },
        total,
        page,
        per_page,
        total_pages,
    }))
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_item_actions_pending() {
        let (approve, reject, edit) = compute_item_actions("pending");
        assert!(approve);
        assert!(reject);
        assert!(edit);
    }

    #[test]
    fn compute_item_actions_empty() {
        let (approve, reject, edit) = compute_item_actions("");
        assert!(approve);
        assert!(reject);
        assert!(edit);
    }

    #[test]
    fn compute_item_actions_approved() {
        let (approve, reject, edit) = compute_item_actions("approved");
        assert!(!approve);
        assert!(!reject);
        assert!(!edit);
    }

    #[test]
    fn compute_item_actions_rejected() {
        let (approve, reject, edit) = compute_item_actions("rejected");
        assert!(!approve);
        assert!(!reject);
        assert!(!edit);
    }

    #[test]
    fn compute_item_actions_edited() {
        let (approve, reject, edit) = compute_item_actions("edited");
        assert!(!approve);
        assert!(!reject);
        assert!(!edit);
    }

    #[test]
    fn compute_item_actions_case_insensitive() {
        let (approve, _, _) = compute_item_actions("Pending");
        assert!(approve);
        let (approve, _, _) = compute_item_actions("PENDING");
        assert!(approve);
    }
}
