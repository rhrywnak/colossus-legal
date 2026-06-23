//! Per-item revisions and status reversals: edit, unapprove, unreject.

use axum::{extract::Path as AxumPath, extract::State, Json};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::models::document_status::{
    REVIEW_STATUS_APPROVED, REVIEW_STATUS_EDITED, REVIEW_STATUS_PENDING, REVIEW_STATUS_REJECTED,
};
use crate::repositories::pipeline_repository::review as review_repo;
use crate::state::AppState;

use super::{check_not_post_ingest, EditRequest, ReviewResponse};

// ── Edit ────────────────────────────────────────────────────────

/// PUT /items/:id
pub async fn edit_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(item_id): AxumPath<i32>,
    Json(body): Json<EditRequest>,
) -> Result<Json<ReviewResponse>, AppError> {
    require_admin(&user)?;

    // Fetch current values for history
    let current = review_repo::get_item_by_id(&state.pipeline_pool, item_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to fetch item {item_id} for edit: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Item {item_id} not found"),
        })?;

    let result = review_repo::edit_item(
        &state.pipeline_pool,
        item_id,
        &user.username,
        body.grounded_page,
        body.verbatim_quote.as_deref(),
        body.notes.as_deref(),
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Edit failed: {e}"),
    })?
    .ok_or_else(|| AppError::NotFound {
        message: format!("Item {item_id} not found"),
    })?;

    // Record history for each changed field
    if current.review_status != result.review_status {
        if let Err(e) = review_repo::insert_edit_history(
            &state.pipeline_pool,
            item_id,
            "review_status",
            Some(&current.review_status),
            Some(&result.review_status),
            &user.username,
        )
        .await
        {
            tracing::error!(
                item_id = item_id,
                error = %e,
                "Failed to write edit history (edit/review_status) — audit trail gap"
            );
        }
    }
    if body.grounded_page.is_some() {
        if let Err(e) = review_repo::insert_edit_history(
            &state.pipeline_pool,
            item_id,
            "grounded_page",
            current.grounded_page.map(|p| p.to_string()).as_deref(),
            body.grounded_page.map(|p| p.to_string()).as_deref(),
            &user.username,
        )
        .await
        {
            tracing::error!(
                item_id = item_id,
                error = %e,
                "Failed to write edit history (edit/grounded_page) — audit trail gap"
            );
        }
    }
    if let Some(ref quote) = body.verbatim_quote {
        if let Err(e) = review_repo::insert_edit_history(
            &state.pipeline_pool,
            item_id,
            "verbatim_quote",
            Some("(previous)"),
            Some(quote),
            &user.username,
        )
        .await
        {
            tracing::error!(
                item_id = item_id,
                error = %e,
                "Failed to write edit history (edit/verbatim_quote) — audit trail gap"
            );
        }
    }

    Ok(Json(ReviewResponse {
        id: result.id,
        review_status: result.review_status,
        reviewed_by: user.username,
        grounded_page: result.grounded_page,
        grounding_status: result.grounding_status,
        cascade_warning: None,
    }))
}

// ── Unapprove ──────────────────────────────────────────────────

/// POST /items/:id/unapprove — revert approved/edited item to pending.
pub async fn unapprove_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(item_id): AxumPath<i32>,
) -> Result<Json<ReviewResponse>, AppError> {
    require_admin(&user)?;

    // Check document is not post-ingest
    let type_info = review_repo::get_item_type_info(&state.pipeline_pool, item_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to fetch type info for item {item_id} (unapprove): {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Item {item_id} not found"),
        })?;

    check_not_post_ingest(&state, &type_info.document_id, "unapprove").await?;

    let current = review_repo::get_item_by_id(&state.pipeline_pool, item_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to fetch item {item_id} for unapprove: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Item {item_id} not found"),
        })?;

    if current.review_status != REVIEW_STATUS_APPROVED
        && current.review_status != REVIEW_STATUS_EDITED
    {
        return Err(AppError::BadRequest {
            message: format!(
                "Cannot unapprove: item status is '{}', expected '{REVIEW_STATUS_APPROVED}' or '{REVIEW_STATUS_EDITED}'",
                current.review_status
            ),
            details: serde_json::json!({"review_status": current.review_status}),
        });
    }

    let result = review_repo::unapprove_item(&state.pipeline_pool, item_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Unapprove failed: {e}"),
        })?
        .ok_or_else(|| AppError::Internal {
            message: "Unapprove returned no rows — race condition?".to_string(),
        })?;

    if let Err(e) = review_repo::insert_edit_history(
        &state.pipeline_pool,
        item_id,
        "review_status",
        Some(&current.review_status),
        Some(REVIEW_STATUS_PENDING),
        &user.username,
    )
    .await
    {
        tracing::error!(
            item_id = item_id,
            error = %e,
            "Failed to write edit history (unapprove) — audit trail gap"
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

// ── Unreject ───────────────────────────────────────────────────

/// POST /items/:id/unreject — revert rejected item to pending.
pub async fn unreject_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(item_id): AxumPath<i32>,
) -> Result<Json<ReviewResponse>, AppError> {
    require_admin(&user)?;

    let type_info = review_repo::get_item_type_info(&state.pipeline_pool, item_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to fetch type info for item {item_id} (unreject): {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Item {item_id} not found"),
        })?;

    check_not_post_ingest(&state, &type_info.document_id, "unreject").await?;

    let current = review_repo::get_item_by_id(&state.pipeline_pool, item_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to fetch item {item_id} for unreject: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Item {item_id} not found"),
        })?;

    if current.review_status != REVIEW_STATUS_REJECTED {
        return Err(AppError::BadRequest {
            message: format!(
                "Cannot unreject: item status is '{}', expected '{REVIEW_STATUS_REJECTED}'",
                current.review_status
            ),
            details: serde_json::json!({"review_status": current.review_status}),
        });
    }

    let result = review_repo::unreject_item(&state.pipeline_pool, item_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Unreject failed: {e}"),
        })?
        .ok_or_else(|| AppError::Internal {
            message: "Unreject returned no rows — race condition?".to_string(),
        })?;

    if let Err(e) = review_repo::insert_edit_history(
        &state.pipeline_pool,
        item_id,
        "review_status",
        Some(REVIEW_STATUS_REJECTED),
        Some(REVIEW_STATUS_PENDING),
        &user.username,
    )
    .await
    {
        tracing::error!(
            item_id = item_id,
            error = %e,
            "Failed to write edit history (unreject) — audit trail gap"
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
