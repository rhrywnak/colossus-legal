//! Process-endpoint progress writers on the `documents` table.
//!
//! Populates the Processing-tab UI's per-step progress and failure
//! surface. Reads on the same columns live in
//! [`super::document_records`] (the `DocumentRecord` SELECT carries
//! them); writes live here because the column set evolves on a
//! different cadence — every progress-display change visits this file,
//! while the canonical CRUD stays stable.

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

/// Persist failure details to the `documents` table.
///
/// Writes three columns the frontend reads to render the FAILED-state
/// UI (`DocumentCard.tsx` and `ProcessingPanel.tsx`):
/// - `failed_step` — the step name that failed (e.g. `"ingest"`).
/// - `error_message` — the operator-facing failure string.
/// - `error_suggestion` — optional recovery hint from
///   `PipelineRegistry::suggest_recovery`. `None` is bound as SQL
///   NULL and the frontend hides the "Suggestion:" line.
///
/// Called by the Restate workflow's top-level failure handler. The
/// legacy worker path doesn't call this — its `pipeline_jobs_*`
/// trigger projects terminal job status onto `documents.status` but
/// has no equivalent error-detail projection, so the legacy "Failed
/// at: X" surface has always shown empty (pre-existing bug B3 from
/// the progress audit, now fixed for the Restate path).
///
/// This function is best-effort at the caller's discretion: a DB
/// failure here would mask the underlying step failure. Callers
/// should log and continue, not propagate.
pub async fn update_document_failure(
    pool: &PgPool,
    document_id: &str,
    failed_step: &str,
    error_message: &str,
    error_suggestion: Option<&str>,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "UPDATE documents SET
            failed_step = $2,
            error_message = $3,
            error_suggestion = $4,
            updated_at = NOW()
         WHERE id = $1",
    )
    .bind(document_id)
    .bind(failed_step)
    .bind(error_message)
    .bind(error_suggestion)
    .execute(pool)
    .await?;
    Ok(())
}
