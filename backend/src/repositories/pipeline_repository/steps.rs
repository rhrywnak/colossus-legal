//! Pipeline step execution logging.
//!
//! Records when each pipeline step starts, completes, or fails.
//! Used by the `/history` endpoint to show execution history.

use serde::Serialize;
use sqlx::PgPool;

/// A single pipeline step execution record.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PipelineStepRecord {
    pub id: i32,
    pub document_id: String,
    pub step_name: String,
    pub status: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_secs: Option<f64>,
    pub triggered_by: Option<String>,
    pub input_params: serde_json::Value,
    pub result_summary: serde_json::Value,
    pub error_message: Option<String>,
}

/// Record the start of a pipeline step. Returns the step ID.
pub async fn record_step_start(
    pool: &PgPool,
    document_id: &str,
    step_name: &str,
    triggered_by: &str,
    input_params: &serde_json::Value,
) -> Result<i32, sqlx::Error> {
    sqlx::query_scalar::<_, i32>(
        "INSERT INTO pipeline_steps (document_id, step_name, status, triggered_by, input_params)
         VALUES ($1, $2, 'running', $3, $4) RETURNING id",
    )
    .bind(document_id)
    .bind(step_name)
    .bind(triggered_by)
    .bind(input_params)
    .fetch_one(pool)
    .await
}

/// Record successful completion of a pipeline step.
pub async fn record_step_complete(
    pool: &PgPool,
    step_id: i32,
    duration_secs: f64,
    result_summary: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE pipeline_steps
         SET status = 'completed', completed_at = NOW(),
             duration_secs = $1, result_summary = $2
         WHERE id = $3",
    )
    .bind(duration_secs)
    .bind(result_summary)
    .bind(step_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Record failure of a pipeline step.
pub async fn record_step_failure(
    pool: &PgPool,
    step_id: i32,
    duration_secs: f64,
    error_message: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE pipeline_steps
         SET status = 'failed', completed_at = NOW(),
             duration_secs = $1, error_message = $2
         WHERE id = $3",
    )
    .bind(duration_secs)
    .bind(error_message)
    .bind(step_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Fetch all pipeline steps for a document, most recent first.
pub async fn get_steps_for_document(
    pool: &PgPool,
    document_id: &str,
) -> Result<Vec<PipelineStepRecord>, sqlx::Error> {
    sqlx::query_as::<_, PipelineStepRecord>(
        "SELECT id, document_id, step_name, status, started_at, completed_at,
                duration_secs, triggered_by, input_params, result_summary, error_message
         FROM pipeline_steps WHERE document_id = $1 ORDER BY started_at DESC",
    )
    .bind(document_id)
    .fetch_all(pool)
    .await
}
