//! Review action handlers: approve, reject, edit, bulk approve,
//! unapprove, unreject, revert-ingest, and item history.

use std::collections::HashMap;
use std::path::Path;

use axum::{extract::Path as AxumPath, extract::State, Json};
use colossus_extract::EntityCategory;
use serde::{Deserialize, Serialize};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository::{self, review as review_repo, steps};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cascade_warning: Option<CascadeWarning>,
}

#[derive(Debug, Serialize)]
pub struct CascadeWarning {
    pub affected_relationships: i64,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct BulkApproveResponse {
    pub document_id: String,
    pub approved_count: u64,
    pub skipped_ungrounded: i64,
    pub remaining_pending: i64,
}

#[derive(Debug, Serialize)]
pub struct RevertIngestResponse {
    pub document_id: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ReprocessResponse {
    pub document_id: String,
    pub status: String,
    pub message: String,
}

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
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound { message: format!("Item {item_id} not found") })?;

    let result = review_repo::approve_item(
        &state.pipeline_pool, item_id, &user.username, body.notes.as_deref(),
    )
    .await
    .map_err(|e| AppError::Internal { message: format!("Approve failed: {e}") })?
    .ok_or_else(|| AppError::NotFound {
        message: format!("Item {item_id} not found"),
    })?;

    // Record history
    review_repo::insert_edit_history(
        &state.pipeline_pool, item_id, "review_status",
        Some(&current.review_status), Some("approved"), &user.username,
    ).await.ok();

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

    // Guard: check entity category — foundation entities cannot be rejected
    let type_info = review_repo::get_item_type_info(&state.pipeline_pool, item_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound { message: format!("Item {item_id} not found") })?;

    let category_map = load_category_map(&state, &type_info.document_id).await;
    let category = category_map.get(&type_info.entity_type)
        .unwrap_or(&EntityCategory::Evidence);

    if *category == EntityCategory::Foundation {
        return Err(AppError::BadRequest {
            message: "Foundation entities cannot be rejected. Use 'edit' to correct, or fix the extraction.".to_string(),
            details: serde_json::json!({
                "entity_type": type_info.entity_type,
                "category": "foundation",
            }),
        });
    }

    // Fetch current status for history
    let current = review_repo::get_item_by_id(&state.pipeline_pool, item_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound { message: format!("Item {item_id} not found") })?;

    let result = review_repo::reject_item(
        &state.pipeline_pool, item_id, &user.username, &body.reason,
    )
    .await
    .map_err(|e| AppError::Internal { message: format!("Reject failed: {e}") })?
    .ok_or_else(|| AppError::NotFound {
        message: format!("Item {item_id} not found"),
    })?;

    // Record history
    review_repo::insert_edit_history(
        &state.pipeline_pool, item_id, "review_status",
        Some(&current.review_status), Some("rejected"), &user.username,
    ).await.ok();

    // Cascade warning for structural entities
    let cascade_warning = if *category == EntityCategory::Structural {
        let affected = review_repo::count_relationships_for_item(&state.pipeline_pool, item_id)
            .await
            .unwrap_or(0);
        if affected > 0 {
            Some(CascadeWarning {
                affected_relationships: affected,
                message: format!("This rejection affects {affected} relationships that reference this entity."),
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
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound { message: format!("Item {item_id} not found") })?;

    let result = review_repo::edit_item(
        &state.pipeline_pool, item_id, &user.username,
        body.grounded_page, body.verbatim_quote.as_deref(), body.notes.as_deref(),
    )
    .await
    .map_err(|e| AppError::Internal { message: format!("Edit failed: {e}") })?
    .ok_or_else(|| AppError::NotFound {
        message: format!("Item {item_id} not found"),
    })?;

    // Record history for each changed field
    if current.review_status != result.review_status {
        review_repo::insert_edit_history(
            &state.pipeline_pool, item_id, "review_status",
            Some(&current.review_status), Some(&result.review_status), &user.username,
        ).await.ok();
    }
    if body.grounded_page.is_some() {
        review_repo::insert_edit_history(
            &state.pipeline_pool, item_id, "grounded_page",
            current.grounded_page.map(|p| p.to_string()).as_deref(),
            body.grounded_page.map(|p| p.to_string()).as_deref(),
            &user.username,
        ).await.ok();
    }
    if let Some(ref quote) = body.verbatim_quote {
        review_repo::insert_edit_history(
            &state.pipeline_pool, item_id, "verbatim_quote",
            Some("(previous)"), Some(quote), &user.username,
        ).await.ok();
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
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound { message: format!("Item {item_id} not found") })?;

    check_not_post_ingest(&state, &type_info.document_id, "unapprove").await?;

    let current = review_repo::get_item_by_id(&state.pipeline_pool, item_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound { message: format!("Item {item_id} not found") })?;

    if current.review_status != "approved" && current.review_status != "edited" {
        return Err(AppError::BadRequest {
            message: format!("Cannot unapprove: item status is '{}', expected 'approved' or 'edited'", current.review_status),
            details: serde_json::json!({"review_status": current.review_status}),
        });
    }

    let result = review_repo::unapprove_item(&state.pipeline_pool, item_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("Unapprove failed: {e}") })?
        .ok_or_else(|| AppError::Internal {
            message: "Unapprove returned no rows — race condition?".to_string(),
        })?;

    review_repo::insert_edit_history(
        &state.pipeline_pool, item_id, "review_status",
        Some(&current.review_status), Some("pending"), &user.username,
    ).await.ok();

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
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound { message: format!("Item {item_id} not found") })?;

    check_not_post_ingest(&state, &type_info.document_id, "unreject").await?;

    let current = review_repo::get_item_by_id(&state.pipeline_pool, item_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound { message: format!("Item {item_id} not found") })?;

    if current.review_status != "rejected" {
        return Err(AppError::BadRequest {
            message: format!("Cannot unreject: item status is '{}', expected 'rejected'", current.review_status),
            details: serde_json::json!({"review_status": current.review_status}),
        });
    }

    let result = review_repo::unreject_item(&state.pipeline_pool, item_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("Unreject failed: {e}") })?
        .ok_or_else(|| AppError::Internal {
            message: "Unreject returned no rows — race condition?".to_string(),
        })?;

    review_repo::insert_edit_history(
        &state.pipeline_pool, item_id, "review_status",
        Some("rejected"), Some("pending"), &user.username,
    ).await.ok();

    Ok(Json(ReviewResponse {
        id: result.id,
        review_status: result.review_status,
        reviewed_by: user.username,
        grounded_page: None,
        grounding_status: None,
        cascade_warning: None,
    }))
}

// ── Revert Ingest ──────────────────────────────────────────────

/// POST /documents/:id/revert-ingest — remove Neo4j data and reset to VERIFIED.
pub async fn revert_ingest_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
) -> Result<Json<RevertIngestResponse>, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, doc_id = %doc_id, "POST revert-ingest");

    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound { message: format!("Document '{doc_id}' not found") })?;

    if !matches!(document.status.as_str(), "INGESTED" | "INDEXED" | "PUBLISHED") {
        return Err(AppError::Conflict {
            message: format!("Cannot revert ingest: status is '{}', expected INGESTED, INDEXED, or PUBLISHED", document.status),
            details: serde_json::json!({"status": document.status}),
        });
    }

    // Remove Neo4j data (reuse delete module's cleanup logic)
    super::delete::cleanup_neo4j(&state, &doc_id).await;

    // Reset status to VERIFIED
    pipeline_repository::update_document_status(&state.pipeline_pool, &doc_id, "VERIFIED")
        .await
        .map_err(|e| AppError::Internal { message: format!("Failed to update status: {e}") })?;

    log_admin_action(
        &state.audit_repo, &user.username, "pipeline.document.revert_ingest",
        Some("document"), Some(&doc_id),
        Some(serde_json::json!({"previous_status": document.status})),
    ).await;

    tracing::info!(doc_id = %doc_id, previous = %document.status, "Ingest reverted — status → VERIFIED");

    Ok(Json(RevertIngestResponse {
        document_id: doc_id,
        status: "VERIFIED".to_string(),
        message: "Ingest reverted. Items unlocked for re-review.".to_string(),
    }))
}

// ── Reprocess ──────────────────────────────────────────────────

/// POST /documents/:id/reprocess — full reset to TEXT_EXTRACTED for re-extraction.
///
/// Cleans Neo4j + Qdrant (best-effort), deletes extraction data in FK-safe
/// order inside a PG transaction, then resets document status to
/// TEXT_EXTRACTED so "Analyze Content" becomes available again.
pub async fn reprocess_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
) -> Result<Json<ReprocessResponse>, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, doc_id = %doc_id, "POST reprocess");

    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    if !matches!(document.status.as_str(), "INGESTED" | "INDEXED" | "PUBLISHED") {
        return Err(AppError::Conflict {
            message: format!(
                "Cannot reprocess: status is '{}', expected INGESTED, INDEXED, or PUBLISHED",
                document.status
            ),
            details: serde_json::json!({"status": document.status}),
        });
    }

    super::delete::cleanup_neo4j(&state, &doc_id).await;
    super::delete::cleanup_qdrant(&state, &doc_id).await;

    let mut txn = state.pipeline_pool.begin().await
        .map_err(|e| AppError::Internal { message: format!("Transaction begin: {e}") })?;

    sqlx::query(
        "DELETE FROM review_edit_history WHERE item_id IN \
         (SELECT id FROM extraction_items WHERE document_id = $1)"
    )
    .bind(&doc_id)
    .execute(&mut *txn)
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Delete review_edit_history: {e}"),
    })?;

    sqlx::query("DELETE FROM extraction_relationships WHERE document_id = $1")
        .bind(&doc_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Delete extraction_relationships: {e}"),
        })?;

    sqlx::query("DELETE FROM extraction_items WHERE document_id = $1")
        .bind(&doc_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Delete extraction_items: {e}"),
        })?;

    sqlx::query(
        "DELETE FROM extraction_chunks WHERE extraction_run_id IN \
         (SELECT id FROM extraction_runs WHERE document_id = $1)"
    )
    .bind(&doc_id)
    .execute(&mut *txn)
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Delete extraction_chunks: {e}"),
    })?;

    sqlx::query("DELETE FROM extraction_runs WHERE document_id = $1")
        .bind(&doc_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Delete extraction_runs: {e}"),
        })?;

    sqlx::query(
        "DELETE FROM pipeline_steps \
         WHERE document_id = $1 AND step_name NOT IN ('upload', 'extract_text')"
    )
    .bind(&doc_id)
    .execute(&mut *txn)
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Delete pipeline_steps: {e}"),
    })?;

    sqlx::query("UPDATE documents SET status = 'TEXT_EXTRACTED', updated_at = NOW() WHERE id = $1")
        .bind(&doc_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Update status: {e}"),
        })?;

    txn.commit().await
        .map_err(|e| AppError::Internal { message: format!("Transaction commit: {e}") })?;

    log_admin_action(
        &state.audit_repo, &user.username, "pipeline.document.reprocess",
        Some("document"), Some(&doc_id),
        Some(serde_json::json!({"previous_status": document.status})),
    ).await;

    tracing::info!(
        doc_id = %doc_id, previous = %document.status,
        "Document reprocessed — status → TEXT_EXTRACTED"
    );

    Ok(Json(ReprocessResponse {
        document_id: doc_id,
        status: "TEXT_EXTRACTED".to_string(),
        message: "Document reset for re-extraction. Select schema and run Analyze Content.".to_string(),
    }))
}

// ── Bulk Approve ────────────────────────────────────────────────

/// POST /documents/:id/approve-all
pub async fn bulk_approve_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
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

    let remaining_pending = review_repo::count_pending(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("Count pending failed: {e}") })?;

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
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?;

    Ok(Json(history))
}

// ── Helpers ────────────────────────────────────────────────────

/// Check that the document is not post-ingest. Returns Conflict error if it is.
async fn check_not_post_ingest(
    state: &AppState,
    document_id: &str,
    action: &str,
) -> Result<(), AppError> {
    let document = pipeline_repository::get_document(&state.pipeline_pool, document_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{document_id}' not found"),
        })?;

    if matches!(document.status.as_str(), "INGESTED" | "INDEXED" | "PUBLISHED" | "COMPLETED") {
        return Err(AppError::Conflict {
            message: format!("Cannot {action}: document is post-ingest (status: {}). Revert ingest first.", document.status),
            details: serde_json::json!({"status": document.status}),
        });
    }
    Ok(())
}

/// Load entity category map from schema. Returns empty map on failure.
async fn load_category_map(state: &AppState, doc_id: &str) -> HashMap<String, EntityCategory> {
    let pipe_config = match pipeline_repository::get_pipeline_config(&state.pipeline_pool, doc_id).await {
        Ok(Some(cfg)) => cfg,
        _ => return HashMap::new(),
    };

    let schema_path = format!("{}/{}", state.config.extraction_schema_dir, pipe_config.schema_file);
    match colossus_extract::ExtractionSchema::from_file(Path::new(&schema_path)) {
        Ok(schema) => schema.entity_types.iter()
            .map(|et| (et.name.clone(), et.category.clone()))
            .collect(),
        Err(_) => HashMap::new(),
    }
}
