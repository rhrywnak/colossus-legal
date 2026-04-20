//! Repository layer for the `llm_models` table.
//!
//! The `llm_models` table is the runtime registry of LLM models available
//! to the pipeline. It lives in the database (not in config files) because:
//!
//! - Models go active / inactive without redeploying the backend.
//! - Per-token costs are updated when providers change pricing.
//! - Admin UI needs to list, edit, and audit them at runtime.
//!
//! Design: DOC_PROCESSING_CONFIG_DESIGN_v2.md Sections 3.2.1 and 3.8.
//!
//! ## Rust Learning: NUMERIC → f64 via SQL cast
//!
//! The `cost_per_*_token` columns are `NUMERIC(12,8)` in Postgres.
//! Direct NUMERIC binding requires the `sqlx` `rust_decimal` feature,
//! which this project does not enable. Instead we cast to `float8` in
//! the SELECT list so sqlx decodes as `Option<f64>` — the same pattern
//! used for `extraction_runs.cost_usd`.

use serde::{Deserialize, Serialize};
use sqlx::PgPool;

/// Input payload for inserting a new `llm_models` row.
///
/// Mirrors the nullable columns of the table. Optional fields become
/// `NULL` in the database.
#[derive(Debug, Clone, Deserialize)]
pub struct InsertModelInput {
    pub id: String,
    pub display_name: String,
    pub provider: String,
    pub api_endpoint: Option<String>,
    pub max_context_tokens: Option<i32>,
    pub max_output_tokens: Option<i32>,
    pub cost_per_input_token: Option<f64>,
    pub cost_per_output_token: Option<f64>,
    pub notes: Option<String>,
}

/// Input payload for updating an existing `llm_models` row.
///
/// Every field is `Option` so the caller can PATCH a subset. `None`
/// leaves the existing column value untouched (COALESCE in the UPDATE).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct UpdateModelInput {
    pub display_name: Option<String>,
    pub provider: Option<String>,
    pub api_endpoint: Option<String>,
    pub max_context_tokens: Option<i32>,
    pub max_output_tokens: Option<i32>,
    pub cost_per_input_token: Option<f64>,
    pub cost_per_output_token: Option<f64>,
    pub is_active: Option<bool>,
    pub notes: Option<String>,
}

/// A row from the `llm_models` registry.
///
/// Each row represents one LLM accessible to the pipeline. The `id` is the
/// canonical model name (e.g. `claude-sonnet-4-6`) and is what
/// `pipeline_config.pass1_model` and `processing_profiles.extraction_model`
/// reference.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LlmModelRecord {
    /// Canonical model ID (e.g. `claude-sonnet-4-6`). Primary key.
    pub id: String,

    /// Human-readable name for UI display.
    pub display_name: String,

    /// Provider backend (`anthropic`, `vllm`, etc.) — routes to the correct
    /// `LlmProvider` implementation in `colossus_extract`.
    pub provider: String,

    /// Custom API endpoint URL. `None` means use the provider default.
    pub api_endpoint: Option<String>,

    /// Maximum input context window in tokens.
    pub max_context_tokens: Option<i32>,

    /// Maximum output tokens the model will generate per call.
    pub max_output_tokens: Option<i32>,

    /// Cost per input token in USD. Decoded from `NUMERIC(12,8)` via a
    /// SQL `::float8` cast — see the module doc comment.
    pub cost_per_input_token: Option<f64>,

    /// Cost per output token in USD. Decoded from `NUMERIC(12,8)` via a
    /// SQL `::float8` cast.
    pub cost_per_output_token: Option<f64>,

    /// Whether this model is currently selectable. Inactive models stay in
    /// the table for audit purposes but cannot be used for new extractions.
    pub is_active: bool,

    /// When this row was inserted.
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Free-form notes (e.g. "primary extraction model", "deprecated").
    pub notes: Option<String>,
}

/// SELECT column list shared by all `llm_models` queries in this module.
///
/// `NUMERIC(12,8)` cost columns are cast to `float8` so sqlx decodes them
/// as `Option<f64>` without needing the `rust_decimal` feature.
const SELECT_COLUMNS: &str = "id, display_name, provider, api_endpoint, \
    max_context_tokens, max_output_tokens, \
    cost_per_input_token::float8 AS cost_per_input_token, \
    cost_per_output_token::float8 AS cost_per_output_token, \
    is_active, created_at, notes";

/// Fetch a single model by ID. Returns `None` if the ID does not exist.
///
/// This does NOT filter by `is_active` — use [`get_active_model_by_id`]
/// for the runtime-selection path. This function exists for Admin UI
/// flows that need to inspect or re-activate an inactive model.
pub async fn get_model_by_id(
    db: &PgPool,
    model_id: &str,
) -> Result<Option<LlmModelRecord>, sqlx::Error> {
    let sql = format!("SELECT {SELECT_COLUMNS} FROM llm_models WHERE id = $1");
    sqlx::query_as::<_, LlmModelRecord>(&sql)
        .bind(model_id)
        .fetch_optional(db)
        .await
}

/// Fetch a model by ID only if `is_active = true`.
///
/// Returns `None` if the model does not exist OR if it exists but has been
/// deactivated. The extraction pipeline calls this: an inactive model must
/// never be used for new runs even if a stale `pipeline_config` references it.
pub async fn get_active_model_by_id(
    db: &PgPool,
    model_id: &str,
) -> Result<Option<LlmModelRecord>, sqlx::Error> {
    let sql = format!(
        "SELECT {SELECT_COLUMNS} FROM llm_models WHERE id = $1 AND is_active = true"
    );
    sqlx::query_as::<_, LlmModelRecord>(&sql)
        .bind(model_id)
        .fetch_optional(db)
        .await
}

/// List every active model, ordered by display name.
///
/// Used by the pre-processing Configuration Panel and the extraction
/// pipeline to populate the model-selection dropdown. Inactive models
/// are excluded so operators cannot accidentally choose a retired model.
pub async fn list_active_models(db: &PgPool) -> Result<Vec<LlmModelRecord>, sqlx::Error> {
    let sql = format!(
        "SELECT {SELECT_COLUMNS} FROM llm_models WHERE is_active = true \
         ORDER BY display_name"
    );
    sqlx::query_as::<_, LlmModelRecord>(&sql)
        .fetch_all(db)
        .await
}

/// List every model — active AND inactive — ordered by display name.
///
/// Used by the Admin > Models listing so operators can see (and re-activate)
/// models they previously deactivated. The client uses each row's
/// `is_active` flag to render the toggle state.
pub async fn list_all_models(db: &PgPool) -> Result<Vec<LlmModelRecord>, sqlx::Error> {
    let sql = format!(
        "SELECT {SELECT_COLUMNS} FROM llm_models ORDER BY display_name"
    );
    sqlx::query_as::<_, LlmModelRecord>(&sql)
        .fetch_all(db)
        .await
}

/// Insert a new `llm_models` row and return it.
///
/// The `is_active` flag defaults to `true` in the schema, so a freshly
/// created model is immediately selectable. Returns `sqlx::Error::Database`
/// with constraint code `23505` (unique_violation) if the id already exists —
/// the handler maps this to `409 Conflict`.
pub async fn insert_model(
    db: &PgPool,
    input: &InsertModelInput,
) -> Result<LlmModelRecord, sqlx::Error> {
    let sql = format!(
        "INSERT INTO llm_models \
           (id, display_name, provider, api_endpoint, \
            max_context_tokens, max_output_tokens, \
            cost_per_input_token, cost_per_output_token, notes) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         RETURNING {SELECT_COLUMNS}"
    );
    sqlx::query_as::<_, LlmModelRecord>(&sql)
        .bind(&input.id)
        .bind(&input.display_name)
        .bind(&input.provider)
        .bind(&input.api_endpoint)
        .bind(input.max_context_tokens)
        .bind(input.max_output_tokens)
        .bind(input.cost_per_input_token)
        .bind(input.cost_per_output_token)
        .bind(&input.notes)
        .fetch_one(db)
        .await
}

/// Update the non-`None` fields of an existing `llm_models` row and return it.
///
/// Uses `COALESCE($n, column)` for every field, so `None` leaves the
/// existing value in place. Returns `Ok(None)` if the id does not exist —
/// the handler maps this to `404 Not Found`.
pub async fn update_model(
    db: &PgPool,
    model_id: &str,
    input: &UpdateModelInput,
) -> Result<Option<LlmModelRecord>, sqlx::Error> {
    let sql = format!(
        "UPDATE llm_models SET \
           display_name = COALESCE($2, display_name), \
           provider = COALESCE($3, provider), \
           api_endpoint = COALESCE($4, api_endpoint), \
           max_context_tokens = COALESCE($5, max_context_tokens), \
           max_output_tokens = COALESCE($6, max_output_tokens), \
           cost_per_input_token = COALESCE($7, cost_per_input_token), \
           cost_per_output_token = COALESCE($8, cost_per_output_token), \
           is_active = COALESCE($9, is_active), \
           notes = COALESCE($10, notes) \
         WHERE id = $1 \
         RETURNING {SELECT_COLUMNS}"
    );
    sqlx::query_as::<_, LlmModelRecord>(&sql)
        .bind(model_id)
        .bind(&input.display_name)
        .bind(&input.provider)
        .bind(&input.api_endpoint)
        .bind(input.max_context_tokens)
        .bind(input.max_output_tokens)
        .bind(input.cost_per_input_token)
        .bind(input.cost_per_output_token)
        .bind(input.is_active)
        .bind(&input.notes)
        .fetch_optional(db)
        .await
}

/// Delete an `llm_models` row by id.
///
/// Returns `true` if a row was deleted, `false` if no row matched the id.
/// The caller is responsible for checking profile YAML references before
/// calling this — the database does not enforce that constraint because
/// profiles live on the filesystem.
pub async fn delete_model(db: &PgPool, model_id: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM llm_models WHERE id = $1")
        .bind(model_id)
        .execute(db)
        .await?;
    Ok(result.rows_affected() > 0)
}

/// Flip the `is_active` flag on an `llm_models` row and return the updated row.
///
/// Returns `Ok(None)` if the id does not exist. The flip is atomic — no
/// read-modify-write race window — because the UPDATE reads the current
/// value in the same statement that writes the new value.
pub async fn toggle_model_active(
    db: &PgPool,
    model_id: &str,
) -> Result<Option<LlmModelRecord>, sqlx::Error> {
    let sql = format!(
        "UPDATE llm_models SET is_active = NOT is_active \
         WHERE id = $1 RETURNING {SELECT_COLUMNS}"
    );
    sqlx::query_as::<_, LlmModelRecord>(&sql)
        .bind(model_id)
        .fetch_optional(db)
        .await
}
