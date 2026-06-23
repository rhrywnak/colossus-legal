//! Document-level review operations: revert-ingest, reprocess, bulk-approve.

use axum::{extract::Path as AxumPath, extract::State, Json};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::models::document_status::{
    STATUS_INDEXED, STATUS_INGESTED, STATUS_PUBLISHED, STATUS_TEXT_EXTRACTED, STATUS_VERIFIED,
};
use crate::pipeline::workflow_steps::{STEP_EXTRACT_TEXT, STEP_UPLOAD};
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository::{self, review as review_repo, steps};
use crate::state::AppState;

use crate::api::pipeline::delete::{cleanup_neo4j, cleanup_qdrant};

use super::{BulkApproveRequest, BulkApproveResponse, ReprocessResponse, RevertIngestResponse};

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
        .map_err(|e| AppError::Internal {
            message: format!("Failed to fetch document '{doc_id}' for revert-ingest: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    if !matches!(
        document.status.as_str(),
        STATUS_INGESTED | STATUS_INDEXED | STATUS_PUBLISHED
    ) {
        return Err(AppError::Conflict {
            message: format!(
                "Cannot revert ingest: status is '{}', expected {STATUS_INGESTED}, {STATUS_INDEXED}, or {STATUS_PUBLISHED}",
                document.status
            ),
            details: serde_json::json!({"status": document.status}),
        });
    }

    // Remove Neo4j data (reuse delete module's cleanup logic)
    cleanup_neo4j(&state, &doc_id).await;

    // Reset status to VERIFIED
    pipeline_repository::update_document_status(&state.pipeline_pool, &doc_id, STATUS_VERIFIED)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update status: {e}"),
        })?;

    log_admin_action(
        &state.audit_repo,
        &user.username,
        "pipeline.document.revert_ingest",
        Some("document"),
        Some(&doc_id),
        Some(serde_json::json!({"previous_status": document.status})),
    )
    .await;

    tracing::info!(doc_id = %doc_id, previous = %document.status, "Ingest reverted — status → VERIFIED");

    Ok(Json(RevertIngestResponse {
        document_id: doc_id,
        status: STATUS_VERIFIED.to_string(),
        message: "Ingest reverted. Items unlocked for re-review.".to_string(),
    }))
}

// ── Reprocess ──────────────────────────────────────────────────

/// Widened relationships-DELETE for the reprocess path (DELETE-FK-FIX).
///
/// Identical in intent to
/// `documents_delete::DELETE_RELATIONSHIPS_TOUCHING_DOCUMENT` (kept as a
/// separate copy by design — the three delete paths are fixed in place, not
/// refactored into one shared helper). Matches every relationship that touches
/// this document: rows it owns (`document_id`) AND rows another document owns
/// that point at this document's items via either RESTRICT FK (`from_item_id` /
/// `to_item_id`). Without the endpoint predicates a foreign relationship
/// targeting this document's items survives and trips the FK on the
/// `extraction_items` delete, rolling the reprocess back.
const REPROCESS_DELETE_RELATIONSHIPS_SQL: &str = "DELETE FROM extraction_relationships \
     WHERE document_id = $1 \
        OR from_item_id IN (SELECT id FROM extraction_items WHERE document_id = $1) \
        OR to_item_id IN (SELECT id FROM extraction_items WHERE document_id = $1)";

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
        .map_err(|e| AppError::Internal {
            message: format!("Failed to fetch document '{doc_id}' for reprocess: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    if !matches!(
        document.status.as_str(),
        STATUS_INGESTED | STATUS_INDEXED | STATUS_PUBLISHED
    ) {
        return Err(AppError::Conflict {
            message: format!(
                "Cannot reprocess: status is '{}', expected {STATUS_INGESTED}, {STATUS_INDEXED}, or {STATUS_PUBLISHED}",
                document.status
            ),
            details: serde_json::json!({"status": document.status}),
        });
    }

    cleanup_neo4j(&state, &doc_id).await;
    cleanup_qdrant(&state, &doc_id).await;

    let mut txn = state
        .pipeline_pool
        .begin()
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Transaction begin: {e}"),
        })?;

    sqlx::query(
        "DELETE FROM review_edit_history WHERE item_id IN \
         (SELECT id FROM extraction_items WHERE document_id = $1)",
    )
    .bind(&doc_id)
    .execute(&mut *txn)
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Delete review_edit_history: {e}"),
    })?;

    sqlx::query(REPROCESS_DELETE_RELATIONSHIPS_SQL)
        .bind(&doc_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Delete extraction_relationships for reprocess of '{doc_id}': {e}"),
        })?;

    sqlx::query("DELETE FROM extraction_items WHERE document_id = $1")
        .bind(&doc_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Delete extraction_items: {e}"),
        })?;

    sqlx::query("DELETE FROM extraction_runs WHERE document_id = $1")
        .bind(&doc_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Delete extraction_runs: {e}"),
        })?;

    // Keep the upload and text-extraction step rows; clear everything later in
    // the pipeline so re-extraction starts from a clean TEXT_EXTRACTED state.
    // The two preserved step names are bound as parameters ($2, $3) rather than
    // inlined as string literals so they stay tied to the canonical
    // `STEP_*` constants (Rule 2 — no magic values) and cannot drift from the
    // names `record_step_start` actually writes.
    sqlx::query(
        "DELETE FROM pipeline_steps \
         WHERE document_id = $1 AND step_name NOT IN ($2, $3)",
    )
    .bind(&doc_id)
    .bind(STEP_UPLOAD)
    .bind(STEP_EXTRACT_TEXT)
    .execute(&mut *txn)
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Delete pipeline_steps for reprocess of '{doc_id}': {e}"),
    })?;

    sqlx::query("UPDATE documents SET status = $1, updated_at = NOW() WHERE id = $2")
        .bind(STATUS_TEXT_EXTRACTED)
        .bind(&doc_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Update status: {e}"),
        })?;

    txn.commit().await.map_err(|e| AppError::Internal {
        message: format!("Transaction commit: {e}"),
    })?;

    log_admin_action(
        &state.audit_repo,
        &user.username,
        "pipeline.document.reprocess",
        Some("document"),
        Some(&doc_id),
        Some(serde_json::json!({"previous_status": document.status})),
    )
    .await;

    tracing::info!(
        doc_id = %doc_id, previous = %document.status,
        "Document reprocessed — status → TEXT_EXTRACTED"
    );

    Ok(Json(ReprocessResponse {
        document_id: doc_id,
        status: STATUS_TEXT_EXTRACTED.to_string(),
        message: "Document reset for re-extraction. Select schema and run Analyze Content."
            .to_string(),
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
            message: format!(
                "Invalid filter '{}' — must be 'grounded' or 'all'",
                body.filter
            ),
            details: serde_json::json!({"field": "filter", "valid": ["grounded", "all"]}),
        });
    }

    let approved_count =
        review_repo::bulk_approve(&state.pipeline_pool, &doc_id, &user.username, &body.filter)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Bulk approve failed: {e}"),
            })?;

    let remaining_pending = review_repo::count_pending(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Count pending failed: {e}"),
        })?;

    let skipped_ungrounded = review_repo::count_ungrounded_pending(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Count ungrounded failed: {e}"),
        })?;

    if let Ok(sid) = steps::record_step_start(
        &state.pipeline_pool,
        &doc_id,
        "bulk_approve",
        &user.username,
        &serde_json::json!({"filter": body.filter}),
    )
    .await
    {
        if let Err(e) = steps::record_step_complete(
            &state.pipeline_pool,
            sid,
            0.0,
            &serde_json::json!({
                "approved_count": approved_count,
                "skipped_ungrounded": skipped_ungrounded,
                "remaining_pending": remaining_pending,
            }),
        )
        .await
        {
            tracing::error!(
                document_id = %doc_id,
                step_id = sid,
                error = %e,
                "Failed to record bulk_approve step completion — audit trail gap"
            );
        }
    }

    Ok(Json(BulkApproveResponse {
        document_id: doc_id,
        approved_count,
        skipped_ungrounded,
        remaining_pending,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DELETE-FK-FIX guard for the reprocess path: the relationships clear must
    /// match BOTH item-endpoint FKs, not just the owning `document_id`. Without
    /// this, reprocessing a document whose items are targeted by another
    /// document's relationship rolls back on the RESTRICT FK. There is no
    /// `#[sqlx::test]` / live-DB harness in this repo, so the widening is
    /// verified by asserting the SQL covers both endpoints; the end-to-end
    /// behaviour is verified manually on DEV.
    #[test]
    fn reprocess_delete_relationships_sql_covers_both_fk_endpoints() {
        let sql = REPROCESS_DELETE_RELATIONSHIPS_SQL;
        assert!(
            sql.contains("document_id = $1"),
            "must still clear rows this document owns"
        );
        assert!(
            sql.contains(
                "from_item_id IN (SELECT id FROM extraction_items WHERE document_id = $1)"
            ),
            "must clear rows pointing FROM this document's items"
        );
        assert!(
            sql.contains("to_item_id IN (SELECT id FROM extraction_items WHERE document_id = $1)"),
            "must clear rows pointing TO this document's items (the RESTRICT endpoint)"
        );
    }
}
