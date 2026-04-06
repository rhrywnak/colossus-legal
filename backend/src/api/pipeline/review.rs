//! Review action handlers: approve, reject, edit, bulk approve.

use axum::{extract::Path, extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::pipeline_repository::{review as review_repo, steps};
use crate::state::AppState;

// ── Request DTOs ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ApproveRequest {
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RejectRequest {
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct EditRequest {
    pub grounded_page: Option<i32>,
    pub verbatim_quote: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BulkApproveRequest {
    pub filter: String,
}

// ── Response DTOs ───────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ReviewResponse {
    pub id: i32,
    pub review_status: String,
    pub reviewed_by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounded_page: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grounding_status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BulkApproveResponse {
    pub document_id: String,
    pub approved_count: u64,
    pub skipped_ungrounded: i64,
    pub remaining_pending: i64,
}

// ── Approve ─────────────────────────────────────────────────────

/// POST /items/:id/approve
pub async fn approve_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(item_id): Path<i32>,
    Json(body): Json<ApproveRequest>,
) -> Result<Json<ReviewResponse>, AppError> {
    require_admin(&user)?;

    let result = review_repo::approve_item(
        &state.pipeline_pool, item_id, &user.username, body.notes.as_deref(),
    )
    .await
    .map_err(|e| AppError::Internal { message: format!("Approve failed: {e}") })?
    .ok_or_else(|| AppError::NotFound {
        message: format!("Item {item_id} not found"),
    })?;

    // Best-effort step logging
    if let Ok(sid) = steps::record_step_start(
        &state.pipeline_pool, "", "review", &user.username,
        &serde_json::json!({"action": "approve", "item_id": item_id}),
    ).await {
        steps::record_step_complete(
            &state.pipeline_pool, sid, 0.0,
            &serde_json::json!({"item_id": item_id, "action": "approve"}),
        ).await.ok();
    }

    Ok(Json(ReviewResponse {
        id: result.id,
        review_status: result.review_status,
        reviewed_by: user.username,
        grounded_page: None,
        grounding_status: None,
    }))
}

// ── Reject ──────────────────────────────────────────────────────

/// POST /items/:id/reject
pub async fn reject_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(item_id): Path<i32>,
    Json(body): Json<RejectRequest>,
) -> Result<Json<ReviewResponse>, AppError> {
    require_admin(&user)?;

    if body.reason.trim().is_empty() {
        return Err(AppError::BadRequest {
            message: "Reject reason must not be empty".to_string(),
            details: serde_json::json!({"field": "reason"}),
        });
    }

    let result = review_repo::reject_item(
        &state.pipeline_pool, item_id, &user.username, &body.reason,
    )
    .await
    .map_err(|e| AppError::Internal { message: format!("Reject failed: {e}") })?
    .ok_or_else(|| AppError::NotFound {
        message: format!("Item {item_id} not found"),
    })?;

    if let Ok(sid) = steps::record_step_start(
        &state.pipeline_pool, "", "review", &user.username,
        &serde_json::json!({"action": "reject", "item_id": item_id}),
    ).await {
        steps::record_step_complete(
            &state.pipeline_pool, sid, 0.0,
            &serde_json::json!({"item_id": item_id, "action": "reject"}),
        ).await.ok();
    }

    Ok(Json(ReviewResponse {
        id: result.id,
        review_status: result.review_status,
        reviewed_by: user.username,
        grounded_page: None,
        grounding_status: None,
    }))
}

// ── Edit ────────────────────────────────────────────────────────

/// PUT /items/:id
pub async fn edit_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(item_id): Path<i32>,
    Json(body): Json<EditRequest>,
) -> Result<Json<ReviewResponse>, AppError> {
    require_admin(&user)?;

    let result = review_repo::edit_item(
        &state.pipeline_pool, item_id, &user.username,
        body.grounded_page, body.verbatim_quote.as_deref(), body.notes.as_deref(),
    )
    .await
    .map_err(|e| AppError::Internal { message: format!("Edit failed: {e}") })?
    .ok_or_else(|| AppError::NotFound {
        message: format!("Item {item_id} not found"),
    })?;

    if let Ok(sid) = steps::record_step_start(
        &state.pipeline_pool, "", "review", &user.username,
        &serde_json::json!({"action": "edit", "item_id": item_id}),
    ).await {
        steps::record_step_complete(
            &state.pipeline_pool, sid, 0.0,
            &serde_json::json!({"item_id": item_id, "action": "edit",
                "grounded_page": result.grounded_page, "grounding_status": result.grounding_status}),
        ).await.ok();
    }

    Ok(Json(ReviewResponse {
        id: result.id,
        review_status: result.review_status,
        reviewed_by: user.username,
        grounded_page: result.grounded_page,
        grounding_status: result.grounding_status,
    }))
}

// ── Bulk Approve ────────────────────────────────────────────────

/// POST /documents/:id/approve-all
pub async fn bulk_approve_handler(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
    Json(body): Json<BulkApproveRequest>,
) -> Result<Json<BulkApproveResponse>, AppError> {
    require_admin(&user)?;

    if body.filter != "grounded" && body.filter != "all" {
        return Err(AppError::BadRequest {
            message: format!("Invalid filter '{}' — must be 'grounded' or 'all'", body.filter),
            details: serde_json::json!({"field": "filter", "valid": ["grounded", "all"]}),
        });
    }

    let approved_count = review_repo::bulk_approve(
        &state.pipeline_pool, &doc_id, &user.username, &body.filter,
    )
    .await
    .map_err(|e| AppError::Internal { message: format!("Bulk approve failed: {e}") })?;

    // count_pending only counts grounded pending items (pipeline-actionable).
    let remaining_pending = review_repo::count_pending(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("Count pending failed: {e}") })?;

    // Count ungrounded pending items (skipped by approve-grounded).
    let skipped_ungrounded = review_repo::count_ungrounded_pending(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("Count ungrounded failed: {e}") })?;

    if let Ok(sid) = steps::record_step_start(
        &state.pipeline_pool, &doc_id, "bulk_approve", &user.username,
        &serde_json::json!({"filter": body.filter}),
    ).await {
        steps::record_step_complete(
            &state.pipeline_pool, sid, 0.0,
            &serde_json::json!({
                "approved_count": approved_count,
                "skipped_ungrounded": skipped_ungrounded,
                "remaining_pending": remaining_pending,
            }),
        ).await.ok();
    }

    Ok(Json(BulkApproveResponse {
        document_id: doc_id,
        approved_count,
        skipped_ungrounded,
        remaining_pending,
    }))
}
