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
/// Used by the Admin UI and the pre-processing Configuration Panel to
/// populate the model-selection dropdown. Inactive models are excluded so
/// operators cannot accidentally choose a retired model.
pub async fn list_active_models(db: &PgPool) -> Result<Vec<LlmModelRecord>, sqlx::Error> {
    let sql = format!(
        "SELECT {SELECT_COLUMNS} FROM llm_models WHERE is_active = true \
         ORDER BY display_name"
    );
    sqlx::query_as::<_, LlmModelRecord>(&sql)
        .fetch_all(db)
        .await
}
