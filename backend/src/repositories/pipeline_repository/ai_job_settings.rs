//! The settings rows that make up the AI jobs on Admin → Overview, and their
//! change history. Read-only (CC_TASK_MODEL_JOBS_PANEL_v1, Stage A).
//!
//! A row belongs to a job when its `ai_job` column is set (migration
//! `20260924145150_ai_job_columns_on_app_settings`). The panel is built from
//! these rows, never from a list in the code, so a job added by a later
//! migration appears on the panel by itself.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::app_settings::AppSettingChangeRecord;
use super::PipelineRepoError;

/// One job setting: which job, what it chooses, and its current value.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct AiJobSettingRow {
    pub key: String,
    pub value: String,
    /// The job token (`answer_analysis`, …).
    pub ai_job: String,
    /// `model` | `instructions` | `effort` — as stored; the service refuses an
    /// unknown one by name.
    pub ai_role: String,
    pub updated_by: String,
    pub updated_at: DateTime<Utc>,
}

/// Every row that belongs to a job, ordered by job then role so the service
/// meets each job's rows together.
///
/// `COALESCE(ai_role, '')` keeps a row whose job is set but whose role is not
/// in the result — the service then names it as malformed rather than the
/// query silently dropping it (Standing Rule 1).
///
/// # Errors
/// A database failure.
pub async fn list_ai_job_settings(
    pool: &PgPool,
) -> Result<Vec<AiJobSettingRow>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, AiJobSettingRow>(
        "SELECT key, value, ai_job, COALESCE(ai_role, '') AS ai_role, updated_by, updated_at \
           FROM app_settings \
          WHERE ai_job IS NOT NULL \
          ORDER BY ai_job, ai_role, key",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Every recorded change to any of `keys`, most recent first.
///
/// ## Rust Learning: binding a slice as a Postgres array
///
/// `= ANY($1)` with a `&[String]` bound sends ONE query for all the keys: sqlx
/// encodes the slice as a `TEXT[]`. The alternative — one query per key — is the
/// N+1 shape the house avoids on page-load paths.
///
/// # Errors
/// A database failure.
pub async fn list_changes_for_keys(
    pool: &PgPool,
    keys: &[String],
) -> Result<Vec<AppSettingChangeRecord>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, AppSettingChangeRecord>(
        "SELECT key, old_value, new_value, actor, at FROM app_setting_changes \
          WHERE key = ANY($1) ORDER BY at DESC",
    )
    .bind(keys)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Settings that name a model but belong to no job: `(meaning, model id)`.
///
/// The panel lists these so a model setting added without an `ai_job` is still
/// seen (Law 22: no job may stay configurable only in Settings).
///
/// # Errors
/// A database failure.
pub async fn list_loose_model_settings(
    pool: &PgPool,
) -> Result<Vec<(String, String)>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT s.meaning, s.value FROM app_settings s \
           JOIN llm_models m ON m.id = s.value \
          WHERE s.ai_job IS NULL ORDER BY s.key",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
