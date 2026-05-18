//! Read-only document state and aggregate queries.
//!
//! Small SELECTs against the `documents` table that don't naturally
//! belong with the canonical CRUD (in [`super::document_records`]) or
//! with the progress writers (in [`super::documents_progress`]):
//!
//! - [`is_cancelled`] — single-row cancellation-flag read consumed by
//!   step-loop polling.
//! - [`count_documents`] — total-row count for the dashboard summary.
//! - [`has_document_of_type`] — existence query used by registry-style
//!   guards.

use sqlx::PgPool;

use super::PipelineRepoError;

/// Check if document is cancelled.
///
/// Returns `false` for a missing row (`fetch_optional` → `None`). The
/// cancellation flag is the asynchronous "stop processing" signal the
/// step-loop polls between steps; treating an absent document as
/// "not cancelled" is correct — a step that has lost its document
/// would fail downstream for a more specific reason than cancellation.
pub async fn is_cancelled(pool: &PgPool, document_id: &str) -> Result<bool, PipelineRepoError> {
    let row = sqlx::query_scalar::<_, bool>("SELECT is_cancelled FROM documents WHERE id = $1")
        .bind(document_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.unwrap_or(false))
}

/// Count total documents in the pipeline.
pub async fn count_documents(pool: &PgPool) -> Result<i64, PipelineRepoError> {
    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM documents")
        .fetch_one(pool)
        .await?;
    Ok(count)
}

/// Check if at least one document of the given type exists.
pub async fn has_document_of_type(
    pool: &PgPool,
    doc_type: &str,
) -> Result<bool, PipelineRepoError> {
    let count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM documents WHERE document_type = $1")
            .bind(doc_type)
            .fetch_one(pool)
            .await?;
    Ok(count > 0)
}
