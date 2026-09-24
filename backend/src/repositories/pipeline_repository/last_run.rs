//! The numbers behind Admin → Data → Last document run
//! (CC_TASK_MODEL_JOBS_PANEL_v1, board 5). Read-only.
//!
//! ## Domain note: why these queries differ from `api::pipeline::metrics`
//!
//! The Overview metrics this replaces counted `documents.status = 'COMPLETED'`
//! and averaged `result_summary->>'grounding_rate'`. The pipeline writes neither:
//! the Completeness step ends a document at `PUBLISHED` (`completeness.rs:342`)
//! and the Verify step records `grounding_pct` (`verify.rs:187`). Every number
//! below reads what the pipeline writes today (inventory of 2026-09-24).

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::PipelineRepoError;
use crate::models::document_status::{
    STATUS_PUBLISHED, STEP_STATUS_COMPLETED, STEP_STATUS_FAILED, STEP_STATUS_RUNNING,
};

/// Document totals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::FromRow)]
pub struct DocumentCounts {
    pub total: i64,
    /// Documents in the pipeline's finished status (`PUBLISHED`).
    pub finished: i64,
}

/// One step's history.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct StepStats {
    pub step_name: String,
    pub runs: i64,
    pub avg_secs: Option<f64>,
    pub failed: i64,
    /// When this step last failed; `None` when it never has.
    pub last_failed_at: Option<DateTime<Utc>>,
}

/// Everything the tab shows, read in one pass.
#[derive(Debug, Clone, PartialEq)]
pub struct LastRunFacts {
    pub counts: DocumentCounts,
    /// Mean quote-check percentage over each document's LATEST verify run.
    pub quote_pct: Option<f64>,
    /// When the last step finished (or started, for one still running).
    pub last_activity: Option<DateTime<Utc>>,
    pub steps: Vec<StepStats>,
    /// Steps running now; above zero means a run is in progress.
    pub running: i64,
    /// Mean total step time of a finished document, for the estimate.
    pub avg_doc_secs: Option<f64>,
}

/// Read the tab's facts.
///
/// # Errors
/// A database failure, from whichever of the five reads failed.
pub async fn last_run_facts(pool: &PgPool) -> Result<LastRunFacts, PipelineRepoError> {
    let counts: DocumentCounts = sqlx::query_as(
        "SELECT COUNT(*) AS total, COUNT(*) FILTER (WHERE status = $1) AS finished FROM documents",
    )
    .bind(STATUS_PUBLISHED)
    .fetch_one(pool)
    .await?;

    // DISTINCT ON keeps one verify row per document — its latest — so a document
    // verified three times counts once, at its current figure.
    let (quote_pct,): (Option<f64>,) = sqlx::query_as(
        "SELECT AVG(pct) FROM ( \
            SELECT DISTINCT ON (document_id) (result_summary->>'grounding_pct')::float8 AS pct \
              FROM pipeline_steps \
             WHERE step_name = 'verify' AND status = $1 \
               AND result_summary->>'grounding_pct' IS NOT NULL \
             ORDER BY document_id, started_at DESC) latest",
    )
    .bind(STEP_STATUS_COMPLETED)
    .fetch_one(pool)
    .await?;

    let (last_activity, running): (Option<DateTime<Utc>>, i64) = sqlx::query_as(
        "SELECT MAX(COALESCE(completed_at, started_at)), COUNT(*) FILTER (WHERE status = $1) \
           FROM pipeline_steps",
    )
    .bind(STEP_STATUS_RUNNING)
    .fetch_one(pool)
    .await?;

    let steps = step_stats(pool).await?;

    let (avg_doc_secs,): (Option<f64>,) = sqlx::query_as(
        "SELECT AVG(doc_secs) FROM ( \
            SELECT ps.document_id, SUM(ps.duration_secs)::float8 AS doc_secs \
              FROM pipeline_steps ps JOIN documents d ON d.id = ps.document_id \
             WHERE d.status = $1 AND ps.status = $2 GROUP BY ps.document_id) per_doc",
    )
    .bind(STATUS_PUBLISHED)
    .bind(STEP_STATUS_COMPLETED)
    .fetch_one(pool)
    .await?;

    Ok(LastRunFacts {
        counts,
        quote_pct,
        last_activity,
        steps,
        running,
        avg_doc_secs,
    })
}

/// Each step's run count, mean time, failures and last failure.
///
/// # Errors
/// A database failure.
async fn step_stats(pool: &PgPool) -> Result<Vec<StepStats>, PipelineRepoError> {
    let steps: Vec<StepStats> = sqlx::query_as(
        "SELECT step_name, COUNT(*) AS runs, AVG(duration_secs)::float8 AS avg_secs, \
                COUNT(*) FILTER (WHERE status = $1) AS failed, \
                MAX(started_at) FILTER (WHERE status = $1) AS last_failed_at \
           FROM pipeline_steps GROUP BY step_name",
    )
    .bind(STEP_STATUS_FAILED)
    .fetch_all(pool)
    .await?;
    Ok(steps)
}
