//! Document-specific update functions for the process endpoint.
//!
//! These functions update progress tracking, error details, and cancellation
//! state on the `documents` table. Separated from `mod.rs` to keep each
//! module under 300 lines (CLAUDE.md golden rule).

use sqlx::PgPool;

use super::PipelineRepoError;

/// Update document processing progress (called during async pipeline execution).
#[allow(clippy::too_many_arguments)]
pub async fn update_processing_progress(
    pool: &PgPool,
    document_id: &str,
    step: &str,
    step_label: &str,
    chunks_total: i32,
    chunks_processed: i32,
    entities_found: i32,
    percent_complete: i32,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "UPDATE documents SET
            processing_step = $2,
            processing_step_label = $3,
            chunks_total = $4,
            chunks_processed = $5,
            entities_found = $6,
            percent_complete = $7,
            updated_at = NOW()
         WHERE id = $1",
    )
    .bind(document_id)
    .bind(step)
    .bind(step_label)
    .bind(chunks_total)
    .bind(chunks_processed)
    .bind(entities_found)
    .bind(percent_complete)
    .execute(pool)
    .await?;
    Ok(())
}

/// Clear progress fields (called when processing completes or fails).
pub async fn clear_processing_progress(
    pool: &PgPool,
    document_id: &str,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "UPDATE documents SET
            processing_step = NULL,
            processing_step_label = NULL,
            chunks_total = 0,
            chunks_processed = 0,
            entities_found = 0,
            percent_complete = 0,
            updated_at = NOW()
         WHERE id = $1",
    )
    .bind(document_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Store error details when processing fails.
pub async fn set_processing_error(
    pool: &PgPool,
    document_id: &str,
    failed_step: &str,
    failed_chunk: Option<i32>,
    error_message: &str,
    error_suggestion: &str,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "UPDATE documents SET
            status = 'FAILED',
            failed_step = $2,
            failed_chunk = $3,
            error_message = $4,
            error_suggestion = $5,
            updated_at = NOW()
         WHERE id = $1",
    )
    .bind(document_id)
    .bind(failed_step)
    .bind(failed_chunk)
    .bind(error_message)
    .bind(error_suggestion)
    .execute(pool)
    .await?;
    Ok(())
}

/// Clear error fields (called when re-processing starts).
pub async fn clear_processing_errors(
    pool: &PgPool,
    document_id: &str,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "UPDATE documents SET
            failed_step = NULL,
            failed_chunk = NULL,
            error_message = NULL,
            error_suggestion = NULL,
            is_cancelled = FALSE,
            updated_at = NOW()
         WHERE id = $1",
    )
    .bind(document_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Set the is_cancelled flag.
pub async fn set_cancelled(
    pool: &PgPool,
    document_id: &str,
) -> Result<(), PipelineRepoError> {
    sqlx::query("UPDATE documents SET is_cancelled = TRUE, updated_at = NOW() WHERE id = $1")
        .bind(document_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Check if document is cancelled.
pub async fn is_cancelled(
    pool: &PgPool,
    document_id: &str,
) -> Result<bool, PipelineRepoError> {
    let row = sqlx::query_scalar::<_, bool>(
        "SELECT is_cancelled FROM documents WHERE id = $1",
    )
    .bind(document_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.unwrap_or(false))
}

/// Store auto-write summary counts.
pub async fn set_write_summary(
    pool: &PgPool,
    document_id: &str,
    entities_written: i32,
    entities_flagged: i32,
    relationships_written: i32,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "UPDATE documents SET
            entities_written = $2,
            entities_flagged = $3,
            relationships_written = $4,
            updated_at = NOW()
         WHERE id = $1",
    )
    .bind(document_id)
    .bind(entities_written)
    .bind(entities_flagged)
    .bind(relationships_written)
    .execute(pool)
    .await?;
    Ok(())
}
