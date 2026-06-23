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

use super::delete_restate_purge::{attempt_restate_purge, inject_restate_purge_into_snapshot};
use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::models::document_status::{
    STATUS_COMPLETED, STATUS_INDEXED, STATUS_INGESTED, STATUS_PUBLISHED,
};
use crate::pipeline::steps::cleanup;
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
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{document_id}' not found"),
        })?;

    let title = doc.title.clone();
    let file_path = doc.file_path.clone();
    let previous_status = doc.status.clone();
    let invocation_id = doc.restate_invocation_id.clone();
    let reason = body.and_then(|b| b.0.reason);

    // Published documents require a reason
    if previous_status == STATUS_PUBLISHED && reason.is_none() {
        return Err(AppError::BadRequest {
            message: format!("Reason is required when deleting a {STATUS_PUBLISHED} document"),
            details: serde_json::json!({ "status": previous_status }),
        });
    }

    // 2. Build audit snapshot before deleting anything
    let mut snapshot = build_audit_snapshot(&state, &document_id, &doc).await?;

    // 2a. Purge the Restate workflow journal (best-effort). Must run
    // BEFORE the destructive Neo4j/Qdrant/Postgres steps so the
    // outcome can be recorded in the audit snapshot the next step
    // writes. The purge cannot block delete: any failure here is
    // logged and recorded, and the surrounding cleanup proceeds. See
    // `attempt_restate_purge` for the outcome matrix.
    let purge_outcome = attempt_restate_purge(
        &state.http_client,
        state.config.restate_admin_url.as_deref(),
        &document_id,
        invocation_id.as_deref(),
    )
    .await;
    inject_restate_purge_into_snapshot(&mut snapshot, invocation_id.as_deref(), &purge_outcome);

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

    // 4. Delete all PostgreSQL data in FK-safe order, atomically — and FIRST,
    // before any cross-store deletion (DELETE-ORDER-FIX).
    //
    // This delegates to delete_all_document_data in the repository, which
    // wraps all DELETEs in a single transaction. Either all data is removed
    // or none is — no partial deletion states.
    //
    // Why this runs BEFORE the Neo4j/Qdrant cleanup below: Postgres is the
    // source of truth. This step used to run AFTER the cross-store deletes, so
    // a failing/rolled-back PG transaction left the document half-wiped — gone
    // from the graph and vector stores but still present in Postgres (the
    // 2026-06-18 George inconsistency on the first failed delete). A PRE-commit
    // cross-store wipe is destructive and unrecoverable; a POST-commit
    // cross-store failure is recoverable — it is logged, and the audit log
    // written in step 3 already records the intended cleanup for an operator to
    // replay. Commit Postgres first, then do the best-effort external deletes.
    // Do not reorder these back.
    //
    // See documents.rs:delete_all_document_data for the detailed explanation
    // of delete ordering and why we do not use ON DELETE CASCADE on documents(id).
    pipeline_repository::documents::delete_all_document_data(&state.pipeline_pool, &document_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to delete document data for '{document_id}': {e}"),
        })?;

    // 5. Neo4j cleanup (best-effort, for documents that have been ingested).
    // Post-commit: a failure here is recoverable, not a half-wipe.
    let needs_graph_cleanup = matches!(
        previous_status.as_str(),
        STATUS_COMPLETED | STATUS_PUBLISHED | STATUS_INGESTED | STATUS_INDEXED
    );
    if needs_graph_cleanup {
        cleanup_neo4j(&state, &document_id).await;
    }

    // 6. Qdrant cleanup (best-effort, for documents that have been indexed).
    // Post-commit: a failure here is recoverable, not a half-wipe.
    let needs_vector_cleanup = matches!(
        previous_status.as_str(),
        STATUS_COMPLETED | STATUS_PUBLISHED | STATUS_INDEXED
    );
    if needs_vector_cleanup {
        cleanup_qdrant(&state, &document_id).await;
    }

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
    let text_pages: i64 =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM document_text WHERE document_id = $1")
            .bind(document_id)
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Count document_text: {e}"),
            })?;

    let item_count: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM extraction_items WHERE document_id = $1",
    )
    .bind(document_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Count extraction_items: {e}"),
    })?;

    let rel_count: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM extraction_relationships WHERE document_id = $1",
    )
    .bind(document_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Count extraction_relationships: {e}"),
    })?;

    // Total cost
    let total_cost: f64 = sqlx::query_scalar::<_, f64>(
        "SELECT COALESCE(SUM(cost_usd)::float8, 0.0) FROM extraction_runs WHERE document_id = $1",
    )
    .bind(document_id)
    .fetch_one(pool)
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Sum extraction cost: {e}"),
    })?;

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
    .map_err(|e| AppError::Internal {
        message: format!("Fetch extraction_items snapshot: {e}"),
    })?;

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
    .map_err(|e| AppError::Internal {
        message: format!("Fetch extraction_relationships snapshot: {e}"),
    })?;

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
    .map_err(|e| AppError::Internal {
        message: format!("Fetch pipeline_steps snapshot: {e}"),
    })?;

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

/// Remove all Neo4j state associated with a document, best-effort.
///
/// ## Why this delegates to the canonical `cleanup::cleanup_neo4j`
///
/// This endpoint and the pipeline's own teardown both need the identical
/// Neo4j cleanup: DETACH DELETE every node owned by the document (matched on
/// the scalar `source_document` for Allegation/Harm/LegalCount/Person/Org and
/// `source_document_id` for Document nodes), then strip the deleted id from
/// surviving shared Party nodes' `source_documents` arrays. Rather than keep a
/// second, drifting copy of that Cypher here, we call the one canonical
/// implementation in [`crate::pipeline::steps::cleanup::cleanup_neo4j`]. Nodes
/// owned by a *different* document (or carrying no `source_document` at all,
/// e.g. canonical Elements) are never matched by the deletes, so they survive.
///
/// Best-effort: the delete endpoint must not fail if Neo4j is unreachable, so
/// we log the typed [`CleanupError`] (which already carries the `doc_id`) and
/// return. The PostgreSQL deletion still proceeds; the audit log records the
/// intended cleanup for later verification.
pub(super) async fn cleanup_neo4j(state: &AppState, document_id: &str) {
    match cleanup::cleanup_neo4j(document_id, &state.graph).await {
        Ok(report) => {
            // `report` carries per-property delete counts plus shared-array
            // strip count — distinct observables for "deleted N", "stripped M",
            // and "no-op" (all zero) states.
            tracing::info!(doc_id = %document_id, ?report, "Neo4j: delete-path cleanup complete");
        }
        Err(e) => {
            tracing::error!(doc_id = %document_id, error = %e, "Neo4j cleanup failed (delete path)");
        }
    }
}

// ── Qdrant cleanup ──────────────────────────────────────────────

/// Remove all Qdrant vectors associated with a document.
///
/// Uses the `document_id` payload field which is indexed in the collection.
/// Best-effort: logs errors but does not fail the request.
pub(super) async fn cleanup_qdrant(state: &AppState, document_id: &str) {
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
