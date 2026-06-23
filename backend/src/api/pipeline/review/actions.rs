//! Per-item review decisions: approve, reject, and item edit history.

use axum::{extract::Path as AxumPath, extract::State, Json};
use colossus_extract::EntityCategory;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::models::document_status::{REVIEW_STATUS_APPROVED, REVIEW_STATUS_REJECTED};
use crate::repositories::pipeline_repository::review as review_repo;
use crate::state::AppState;

use crate::api::pipeline::items::load_category_map;

use super::{is_rejection_allowed, ApproveRequest, CascadeWarning, RejectRequest, ReviewResponse};

// ── Approve ─────────────────────────────────────────────────────

/// POST /items/:id/approve
pub async fn approve_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(item_id): AxumPath<i32>,
    Json(body): Json<ApproveRequest>,
) -> Result<Json<ReviewResponse>, AppError> {
    require_admin(&user)?;

    // Fetch current status for history
    let current = review_repo::get_item_by_id(&state.pipeline_pool, item_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to fetch item {item_id} for approve: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Item {item_id} not found"),
        })?;

    let result = review_repo::approve_item(
        &state.pipeline_pool,
        item_id,
        &user.username,
        body.notes.as_deref(),
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Approve failed: {e}"),
    })?
    .ok_or_else(|| AppError::NotFound {
        message: format!("Item {item_id} not found"),
    })?;

    // Record history
    if let Err(e) = review_repo::insert_edit_history(
        &state.pipeline_pool,
        item_id,
        "review_status",
        Some(&current.review_status),
        Some(REVIEW_STATUS_APPROVED),
        &user.username,
    )
    .await
    {
        tracing::error!(
            item_id = item_id,
            error = %e,
            "Failed to write edit history (approve) — audit trail gap"
        );
    }

    Ok(Json(ReviewResponse {
        id: result.id,
        review_status: result.review_status,
        reviewed_by: user.username,
        grounded_page: None,
        grounding_status: None,
        cascade_warning: None,
    }))
}

// ── Reject ──────────────────────────────────────────────────────

/// POST /items/:id/reject
pub async fn reject_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(item_id): AxumPath<i32>,
    Json(body): Json<RejectRequest>,
) -> Result<Json<ReviewResponse>, AppError> {
    require_admin(&user)?;

    if body.reason.trim().is_empty() {
        return Err(AppError::BadRequest {
            message: "Reject reason must not be empty".to_string(),
            details: serde_json::json!({"field": "reason"}),
        });
    }

    // Look up entity category for the cascade warning on Structural entities.
    let type_info = review_repo::get_item_type_info(&state.pipeline_pool, item_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to fetch type info for item {item_id} (reject): {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Item {item_id} not found"),
        })?;

    let category_map = load_category_map(&state, &type_info.document_id)
        .await
        .map_err(|e| {
            tracing::error!(
                document_id = %type_info.document_id, error = %e,
                "reject_handler: failed to load category map for cascade warning"
            );
            AppError::Internal { message: e }
        })?;
    let category = category_map
        .get(&type_info.entity_type)
        .unwrap_or(&EntityCategory::Evidence);

    if !is_rejection_allowed(category) {
        return Err(AppError::BadRequest {
            message: format!("Rejection is not allowed for {:?} entities", category),
            details: serde_json::json!({
                "entity_type": type_info.entity_type,
                "category": format!("{:?}", category),
            }),
        });
    }

    // Fetch current status for history
    let current = review_repo::get_item_by_id(&state.pipeline_pool, item_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to fetch item {item_id} for reject: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Item {item_id} not found"),
        })?;

    let result =
        review_repo::reject_item(&state.pipeline_pool, item_id, &user.username, &body.reason)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Reject failed: {e}"),
            })?
            .ok_or_else(|| AppError::NotFound {
                message: format!("Item {item_id} not found"),
            })?;

    // Record history
    if let Err(e) = review_repo::insert_edit_history(
        &state.pipeline_pool,
        item_id,
        "review_status",
        Some(&current.review_status),
        Some(REVIEW_STATUS_REJECTED),
        &user.username,
    )
    .await
    {
        tracing::error!(
            item_id = item_id,
            error = %e,
            "Failed to write edit history (reject) — audit trail gap"
        );
    }

    // Cascade warning for structural entities
    let cascade_warning = if *category == EntityCategory::Structural {
        let affected = review_repo::count_relationships_for_item(&state.pipeline_pool, item_id)
            .await
            .unwrap_or(0);
        if affected > 0 {
            Some(CascadeWarning {
                affected_relationships: affected,
                message: format!(
                    "This rejection affects {affected} relationships that reference this entity."
                ),
            })
        } else {
            None
        }
    } else {
        None
    };

    Ok(Json(ReviewResponse {
        id: result.id,
        review_status: result.review_status,
        reviewed_by: user.username,
        grounded_page: None,
        grounding_status: None,
        cascade_warning,
    }))
}

// ── Item History ───────────────────────────────────────────────

/// GET /items/:id/history — get edit history for an item.
pub async fn item_history_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(item_id): AxumPath<i32>,
) -> Result<Json<Vec<review_repo::EditHistoryRecord>>, AppError> {
    require_admin(&user)?;

    let history = review_repo::get_edit_history(&state.pipeline_pool, item_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to fetch edit history for item {item_id}: {e}"),
        })?;

    Ok(Json(history))
}
