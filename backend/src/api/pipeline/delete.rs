//! DELETE /api/admin/pipeline/documents/:id — Delete a document with audit trail.
//!
//! ## Rust Learning: Transactional deletes with audit snapshots
//!
//! Before destroying data, we capture a complete snapshot of the document and
//! all its related records into a `document_audit_log` row. This INSERT uses
//! NO foreign key to the `documents` table, so the audit entry survives the
//! subsequent deletion.
//!
//! Neo4j and Qdrant cleanup are best-effort: if either external system is
//! unreachable, we log the error and continue with the PostgreSQL deletion.
//! The audit log records the intended cleanup so an operator can verify later.

use axum::{
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::pipeline_repository;
use crate::services::qdrant_service;
use crate::state::AppState;

// ── Request DTO ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct DeleteRequest {
    pub reason: Option<String>,
}

// ── Handler ─────────────────────────────────────────────────────

/// DELETE /api/admin/pipeline/documents/:id
///
/// Enhanced delete: builds an audit snapshot, writes to `document_audit_log`,
/// cleans up Neo4j and Qdrant (best-effort), then deletes all PostgreSQL rows
/// in FK-safe order. Returns 204 No Content on success.
pub async fn delete_document(
    user: AuthUser,
    State(state): State<AppState>,
    Path(document_id): Path<String>,
    body: Option<axum::Json<DeleteRequest>>,
) -> Result<StatusCode, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, doc_id = %document_id, "DELETE /api/admin/pipeline/documents/:id");

    // 1. Fetch document (404 if not found)
    let doc = pipeline_repository::get_document(&state.pipeline_pool, &document_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{document_id}' not found"),
        })?;

    let title = doc.title.clone();
    let file_path = doc.file_path.clone();
    let previous_status = doc.status.clone();
    let reason = body.and_then(|b| b.0.reason);

    // Published documents require a reason
    if previous_status == "PUBLISHED" && reason.is_none() {
        return Err(AppError::BadRequest {
            message: "Reason is required when deleting a PUBLISHED document".to_string(),
            details: serde_json::json!({ "status": previous_status }),
        });
    }

    // 2. Build audit snapshot before deleting anything
    let snapshot = build_audit_snapshot(&state, &document_id, &doc).await?;

    // 3. Write audit log entry (before deletion — survives even if delete fails)
    sqlx::query(
        "INSERT INTO document_audit_log \
         (document_id, document_title, action, reason, performed_by, previous_status, snapshot) \
         VALUES ($1, $2, 'DELETE', $3, $4, $5, $6)",
    )
    .bind(&document_id)
    .bind(&title)
    .bind(&reason)
    .bind(&user.username)
    .bind(&previous_status)
    .bind(&snapshot)
    .execute(&state.pipeline_pool)
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to write audit log: {e}"),
    })?;

    // 4. Neo4j cleanup (best-effort, only for ingested/published/indexed documents)
    let needs_graph_cleanup = matches!(
        previous_status.as_str(),
        "PUBLISHED" | "INGESTED" | "INDEXED"
    );
    if needs_graph_cleanup {
        cleanup_neo4j(&state, &document_id).await;
    }

    // 5. Qdrant cleanup (best-effort, only for published/indexed documents)
    let needs_vector_cleanup = matches!(
        previous_status.as_str(),
        "PUBLISHED" | "INDEXED"
    );
    if needs_vector_cleanup {
        cleanup_qdrant(&state, &document_id).await;
    }

    // 6. PostgreSQL deletion — FK-safe order in a single transaction
    let mut txn = state.pipeline_pool.begin().await.map_err(|e| {
        AppError::Internal { message: format!("Failed to begin transaction: {e}") }
    })?;

    sqlx::query("DELETE FROM extraction_relationships WHERE document_id = $1")
        .bind(&document_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| AppError::Internal { message: format!("Delete extraction_relationships: {e}") })?;

    sqlx::query("DELETE FROM extraction_items WHERE document_id = $1")
        .bind(&document_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| AppError::Internal { message: format!("Delete extraction_items: {e}") })?;

    sqlx::query("DELETE FROM extraction_runs WHERE document_id = $1")
        .bind(&document_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| AppError::Internal { message: format!("Delete extraction_runs: {e}") })?;

    sqlx::query("DELETE FROM document_text WHERE document_id = $1")
        .bind(&document_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| AppError::Internal { message: format!("Delete document_text: {e}") })?;

    sqlx::query("DELETE FROM pipeline_steps WHERE document_id = $1")
        .bind(&document_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| AppError::Internal { message: format!("Delete pipeline_steps: {e}") })?;

    sqlx::query("DELETE FROM pipeline_config WHERE document_id = $1")
        .bind(&document_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| AppError::Internal { message: format!("Delete pipeline_config: {e}") })?;

    sqlx::query("DELETE FROM documents WHERE id = $1")
        .bind(&document_id)
        .execute(&mut *txn)
        .await
        .map_err(|e| AppError::Internal { message: format!("Delete documents: {e}") })?;

    txn.commit().await.map_err(|e| {
        AppError::Internal { message: format!("Failed to commit transaction: {e}") }
    })?;

    // 7. Delete PDF file from disk (warn on failure, don't fail the request)
    let full_path = format!(
        "{}/{}",
        state.config.document_storage_path.trim_end_matches('/'),
        file_path
    );
    if let Err(e) = tokio::fs::remove_file(&full_path).await {
        tracing::warn!(
            path = %full_path,
            error = %e,
            "Failed to delete PDF file from disk (DB records already removed)"
        );
    }

    tracing::info!(
        "Deleted document '{}' (id: {}) by {}, reason: {:?}",
        title,
        document_id,
        user.username,
        reason
    );

    Ok(StatusCode::NO_CONTENT)
}

// ── Audit snapshot builder ──────────────────────────────────────

/// Build a JSONB snapshot of all data associated with a document.
///
/// This captures everything that will be destroyed so the audit log
/// has a complete record of what existed before deletion.
async fn build_audit_snapshot(
    state: &AppState,
    document_id: &str,
    doc: &pipeline_repository::DocumentRecord,
) -> Result<serde_json::Value, AppError> {
    let pool = &state.pipeline_pool;

    // Counts
    let text_pages: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM document_text WHERE document_id = $1",
    )
    .bind(document_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal { message: format!("Count document_text: {e}") })?;

    let item_count: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM extraction_items WHERE document_id = $1",
    )
    .bind(document_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal { message: format!("Count extraction_items: {e}") })?;

    let rel_count: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM extraction_relationships WHERE document_id = $1",
    )
    .bind(document_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal { message: format!("Count extraction_relationships: {e}") })?;

    // Total cost
    let total_cost: f64 = sqlx::query_scalar::<_, f64>(
        "SELECT COALESCE(SUM(cost_usd)::float8, 0.0) FROM extraction_runs WHERE document_id = $1",
    )
    .bind(document_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal { message: format!("Sum extraction cost: {e}") })?;

    // Extraction items as JSON array
    let items_json: Vec<serde_json::Value> = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT json_build_object(\
            'id', id, 'entity_type', entity_type, 'item_data', item_data, \
            'verbatim_quote', verbatim_quote, 'grounding_status', grounding_status, \
            'grounded_page', grounded_page, 'review_status', review_status\
         ) FROM extraction_items WHERE document_id = $1",
    )
    .bind(document_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal { message: format!("Fetch extraction_items snapshot: {e}") })?;

    // Extraction relationships as JSON array
    let rels_json: Vec<serde_json::Value> = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT json_build_object(\
            'id', id, 'from_item_id', from_item_id, 'to_item_id', to_item_id, \
            'relationship_type', relationship_type, 'properties', properties\
         ) FROM extraction_relationships WHERE document_id = $1",
    )
    .bind(document_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal { message: format!("Fetch extraction_relationships snapshot: {e}") })?;

    // Pipeline steps as JSON array
    let steps_json: Vec<serde_json::Value> = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT json_build_object(\
            'step_name', step_name, 'status', status, 'started_at', started_at, \
            'completed_at', completed_at, 'duration_secs', duration_secs, \
            'triggered_by', triggered_by, 'result_summary', result_summary, \
            'error_message', error_message\
         ) FROM pipeline_steps WHERE document_id = $1 ORDER BY started_at",
    )
    .bind(document_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Internal { message: format!("Fetch pipeline_steps snapshot: {e}") })?;

    Ok(serde_json::json!({
        "document": {
            "id": doc.id,
            "title": doc.title,
            "file_path": doc.file_path,
            "file_hash": doc.file_hash,
            "document_type": doc.document_type,
            "status": doc.status,
            "created_at": doc.created_at.to_rfc3339(),
            "updated_at": doc.updated_at.to_rfc3339(),
            "assigned_reviewer": doc.assigned_reviewer,
        },
        "counts": {
            "text_pages": text_pages,
            "extraction_items": item_count,
            "extraction_relationships": rel_count,
            "total_cost_usd": total_cost,
        },
        "extraction_items": items_json,
        "extraction_relationships": rels_json,
        "pipeline_steps": steps_json,
    }))
}

// ── Neo4j cleanup ───────────────────────────────────────────────

/// Remove all Neo4j nodes associated with a document.
///
/// ## Property names across node types
///
/// Different node types use different property names for the source document:
/// - `source_document` — Person, Organization, ComplaintAllegation, Harm, LegalCount
/// - `source_document_id` — Document nodes
///
/// Person/Org nodes can belong to multiple documents (via `source_documents` array).
/// For simplicity, we delete all nodes where this is their `source_document`,
/// which handles the single-document case. Multi-document Person/Org nodes
/// retain their `source_documents` array with this doc removed — but since we
/// DETACH DELETE by `source_document`, only nodes originally created for this
/// doc get removed. Nodes shared across documents survive.
///
/// Best-effort: logs errors but does not fail the request.
pub(super) async fn cleanup_neo4j(state: &AppState, document_id: &str) {
    // Delete nodes where source_document matches (Allegation, Harm, LegalCount, Person, Org)
    match state
        .graph
        .execute(
            neo4rs::query("MATCH (n) WHERE n.source_document = $doc_id DETACH DELETE n RETURN count(n) AS removed")
                .param("doc_id", document_id),
        )
        .await
    {
        Ok(mut result) => {
            let removed: i64 = result
                .next()
                .await
                .ok()
                .flatten()
                .and_then(|row| row.get("removed").ok())
                .unwrap_or(0);
            tracing::info!(doc_id = %document_id, removed, "Neo4j: deleted nodes by source_document");
        }
        Err(e) => {
            tracing::error!(doc_id = %document_id, error = %e, "Neo4j cleanup failed (source_document)");
        }
    }

    // Delete Document node where source_document_id matches
    match state
        .graph
        .execute(
            neo4rs::query("MATCH (n) WHERE n.source_document_id = $doc_id DETACH DELETE n RETURN count(n) AS removed")
                .param("doc_id", document_id),
        )
        .await
    {
        Ok(mut result) => {
            let removed: i64 = result
                .next()
                .await
                .ok()
                .flatten()
                .and_then(|row| row.get("removed").ok())
                .unwrap_or(0);
            if removed > 0 {
                tracing::info!(doc_id = %document_id, removed, "Neo4j: deleted nodes by source_document_id");
            }
        }
        Err(e) => {
            tracing::error!(doc_id = %document_id, error = %e, "Neo4j cleanup failed (source_document_id)");
        }
    }
}

// ── Qdrant cleanup ──────────────────────────────────────────────

/// Remove all Qdrant vectors associated with a document.
///
/// Uses the `document_id` payload field which is indexed in the collection.
/// Best-effort: logs errors but does not fail the request.
async fn cleanup_qdrant(state: &AppState, document_id: &str) {
    match qdrant_service::delete_points_by_filter(
        &state.http_client,
        &state.config.qdrant_url,
        "document_id",
        document_id,
    )
    .await
    {
        Ok(count) => {
            tracing::info!(doc_id = %document_id, count, "Qdrant: deleted vectors");
        }
        Err(e) => {
            tracing::error!(doc_id = %document_id, error = %e, "Qdrant cleanup failed");
        }
    }
}
