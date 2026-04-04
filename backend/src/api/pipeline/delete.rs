//! DELETE /api/admin/pipeline/documents/:id — Delete a document and all related data.
//!
//! ## Rust Learning: Transactional deletes
//!
//! When multiple tables reference a document via foreign keys, we must delete
//! child rows before the parent. Wrapping all deletes in a single transaction
//! ensures atomicity — either everything is removed or nothing is.

use axum::{
    extract::{Path, State},
    http::StatusCode,
};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::pipeline_repository;
use crate::state::AppState;

/// DELETE /api/admin/pipeline/documents/:id
///
/// Deletes a document and all related rows in FK-safe order within a single
/// transaction. After the transaction commits, deletes the PDF file from disk.
/// Returns 204 No Content on success.
pub async fn delete_document(
    user: AuthUser,
    State(state): State<AppState>,
    Path(document_id): Path<String>,
) -> Result<StatusCode, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, doc_id = %document_id, "DELETE /api/admin/pipeline/documents/:id");

    // Fetch the document to get file_path and title (404 if not found)
    let doc = pipeline_repository::get_document(&state.pipeline_pool, &document_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{document_id}' not found"),
        })?;

    let title = doc.title.clone();
    let file_path = doc.file_path.clone();

    // Delete all related rows in FK-safe order within a single transaction.
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

    // Delete the PDF file from disk. Log a warning on failure but don't fail
    // the request — the DB records are already deleted.
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
        "Deleted document '{}' (id: {}) by {}",
        title,
        document_id,
        user.username
    );

    Ok(StatusCode::NO_CONTENT)
}
