//! POST /preview-prompt — assemble the full extraction prompt a given
//! document would be sent, without actually calling the LLM.
//!
//! Lets an operator see the exact rendered template (with
//! `{{document_text}}` / `{{chunk_text}}` substituted and `{{schema_json}}`
//! inserted) plus an input-token and cost estimate before clicking
//! Process. Mirrors the setup phase of `pipeline::steps::llm_extract` up
//! through template substitution — no DB writes, no provider call.
//!
//! Design: DOC_PROCESSING_CONFIG_DESIGN_v2.md Section 3.5.

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use colossus_extract::{FixedSizeSplitter, TextSplitter};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::pipeline::config::{resolve_config, ProcessingProfile};
use crate::repositories::pipeline_repository::{self, models};
use crate::state::AppState;

/// Rough character-to-token ratio used for the input-token estimate.
/// Four characters per token is a commonly-cited approximation for
/// English text with Anthropic tokenizers; the preview response makes
/// clear this is an *estimate*.
const CHARS_PER_TOKEN: i64 = 4;

/// Fallback chunk_size / chunk_overlap when the resolved config has
/// `None` for either. Must match the fallbacks in `llm_extract.rs` so
/// the preview reflects what the extractor actually does.
const FALLBACK_CHUNK_SIZE: i32 = 8000;
const FALLBACK_CHUNK_OVERLAP: i32 = 500;

/// String value of `chunking_mode` that selects the single-call path.
const CHUNKING_MODE_FULL: &str = "full";

#[derive(Debug, Deserialize)]
pub struct PreviewPromptInput {
    pub document_id: String,
    #[serde(default)]
    pub profile_name: Option<String>,
    #[serde(default)]
    pub template_file: Option<String>,
    #[serde(default)]
    pub schema_file: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PromptPreviewResponse {
    pub profile_name: String,
    pub template_file: String,
    pub schema_file: String,
    pub model: String,
    pub chunking_mode: String,
    pub assembled_prompt: String,
    pub estimated_input_tokens: i64,
    pub estimated_cost_usd: Option<f64>,
    pub chunk_count: Option<i32>,
    pub notes: Vec<String>,
}

/// POST /api/admin/pipeline/preview-prompt — render the prompt a document
/// would be sent without calling the LLM.
///
/// Resolution order matches `llm_extract.rs`: per-document `pipeline_config`
/// overrides → profile → system defaults, with `input` overrides layered on
/// top of that for preview-time "what if" exploration.
pub async fn preview_prompt(
    user: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<PreviewPromptInput>,
) -> Result<Json<PromptPreviewResponse>, AppError> {
    require_admin(&user)?;
    let db = &state.pipeline_pool;
    let mut notes: Vec<String> = Vec::new();

    // 1. Document text.
    let pages = pipeline_repository::get_document_text(db, &input.document_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to load document text: {e}"),
        })?;
    if pages.is_empty() {
        return Err(AppError::BadRequest {
            message: format!("Document '{}' has no extracted text pages", input.document_id),
            details: serde_json::json!({"document_id": input.document_id}),
        });
    }
    let full_text = pages
        .iter()
        .map(|p| format!("--- Page {} ---\n{}", p.page_number, p.text_content))
        .collect::<Vec<_>>()
        .join("\n\n");

    // 2. Pipeline config + overrides.
    let pipe_config = pipeline_repository::get_pipeline_config(db, &input.document_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to load pipeline_config: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("No pipeline_config for document '{}'", input.document_id),
        })?;
    let overrides = pipeline_repository::get_pipeline_config_overrides(db, &input.document_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to load overrides: {e}"),
        })?;

    // 3. Profile name: input > override > schema-derived > "default".
    let profile_name = input
        .profile_name
        .clone()
        .or_else(|| overrides.profile_name.clone())
        .unwrap_or_else(|| default_profile_name_from_schema(&pipe_config.schema_file));

    // 4. Load profile.
    let profile = ProcessingProfile::load(&state.config.processing_profile_dir, &profile_name)
        .map_err(|e| AppError::BadRequest {
            message: format!("Failed to load profile '{profile_name}': {e}"),
            details: serde_json::json!({"profile_name": profile_name}),
        })?;

    // 5. Resolve three-level hierarchy, then layer input overrides on top.
    let mut resolved = resolve_config(&profile, &overrides);
    apply_preview_overrides(&mut resolved, &input, &mut notes);

    // 6. Load template + schema files.
    let template_path =
        std::path::Path::new(&state.config.extraction_template_dir).join(&resolved.template_file);
    let template_text =
        tokio::fs::read_to_string(&template_path)
            .await
            .map_err(|e| AppError::BadRequest {
                message: format!(
                    "Failed to read template '{}': {e}",
                    resolved.template_file
                ),
                details: serde_json::json!({"field": "template_file"}),
            })?;

    let schema_path =
        std::path::Path::new(&state.config.extraction_schema_dir).join(&resolved.schema_file);
    let schema = colossus_extract::ExtractionSchema::from_file(&schema_path).map_err(|e| {
        AppError::BadRequest {
            message: format!("Failed to load schema '{}': {e}", resolved.schema_file),
            details: serde_json::json!({"field": "schema_file"}),
        }
    })?;
    let schema_json = serde_json::to_string_pretty(&schema).map_err(|e| AppError::Internal {
        message: format!("Failed to serialize schema: {e}"),
    })?;

    // 7. Assemble the prompt + compute chunk_count depending on mode.
    let (assembled_prompt, chunk_count) = if resolved.chunking_mode == CHUNKING_MODE_FULL {
        notes.push("Full document mode — single LLM call".to_string());
        let prompt = template_text
            .replace("{{schema_json}}", &schema_json)
            .replace("{{document_text}}", &full_text);
        (prompt, None)
    } else {
        let chunk_size = resolved.chunk_size.unwrap_or(FALLBACK_CHUNK_SIZE).max(1) as usize;
        let chunk_overlap = resolved
            .chunk_overlap
            .unwrap_or(FALLBACK_CHUNK_OVERLAP)
            .max(0) as usize;
        let chunks = FixedSizeSplitter::with_config(chunk_size, chunk_overlap).split(&full_text);
        let chunk_total = chunks.len() as i32;
        notes.push(format!(
            "Chunked mode — template will be applied to {chunk_total} chunk(s)"
        ));
        notes.push("Showing preview with first chunk text".to_string());
        let first_chunk_text = chunks
            .first()
            .map(|c| c.text.as_str())
            .unwrap_or(full_text.as_str());
        let prompt = template_text
            .replace("{{schema_json}}", &schema_json)
            .replace("{{chunk_text}}", first_chunk_text);
        (prompt, Some(chunk_total))
    };

    // 8. Token + cost estimates.
    let estimated_input_tokens = (assembled_prompt.len() as i64) / CHARS_PER_TOKEN;
    let estimated_cost_usd = estimate_cost(
        db,
        &resolved.model,
        estimated_input_tokens,
        &mut notes,
    )
    .await;

    Ok(Json(PromptPreviewResponse {
        profile_name: resolved.profile_name,
        template_file: resolved.template_file,
        schema_file: resolved.schema_file,
        model: resolved.model,
        chunking_mode: resolved.chunking_mode,
        assembled_prompt,
        estimated_input_tokens,
        estimated_cost_usd,
        chunk_count,
        notes,
    }))
}

/// Derive a profile name from the schema filename as a last-resort fallback.
///
/// Mirrors `pipeline::steps::llm_extract::default_profile_name_from_schema`
/// so preview and the real extraction path agree on the implicit profile.
fn default_profile_name_from_schema(schema_file: &str) -> String {
    schema_file
        .trim_end_matches(".yaml")
        .trim_end_matches("_v2")
        .to_string()
}

/// Layer preview-only overrides from the request body onto a resolved config.
///
/// `template_file` and `schema_file` in `PreviewPromptInput` are admin-ui
/// "what if" levers that are NOT stored anywhere — they only affect this
/// preview response. Each applied override adds a `notes` entry so the
/// response makes clear the preview diverges from the persisted config.
fn apply_preview_overrides(
    resolved: &mut crate::pipeline::config::ResolvedConfig,
    input: &PreviewPromptInput,
    notes: &mut Vec<String>,
) {
    if let Some(tf) = input.template_file.clone() {
        notes.push(format!(
            "Preview override: template_file='{tf}' (profile default: '{}')",
            resolved.template_file
        ));
        resolved.template_file = tf;
    }
    if let Some(sf) = input.schema_file.clone() {
        notes.push(format!(
            "Preview override: schema_file='{sf}' (profile default: '{}')",
            resolved.schema_file
        ));
        resolved.schema_file = sf;
    }
}

/// Estimate the input cost for this preview run.
///
/// Looks up the resolved model in `llm_models`. If the row doesn't exist,
/// is inactive, or has `cost_per_input_token = NULL`, returns `None` and
/// adds an explanatory `notes` entry — the UI renders "—" instead of $0.
async fn estimate_cost(
    db: &sqlx::PgPool,
    model_id: &str,
    estimated_input_tokens: i64,
    notes: &mut Vec<String>,
) -> Option<f64> {
    match models::get_active_model_by_id(db, model_id).await {
        Ok(Some(m)) => match m.cost_per_input_token {
            Some(rate) => Some(rate * estimated_input_tokens as f64),
            None => {
                notes.push(format!(
                    "Model '{model_id}' has no cost_per_input_token — cost unavailable"
                ));
                None
            }
        },
        Ok(None) => {
            notes.push(format!(
                "Model '{model_id}' not found or inactive — cost unavailable"
            ));
            None
        }
        Err(e) => {
            tracing::warn!(error = %e, "Cost lookup failed — returning None");
            notes.push("Cost lookup failed — see server logs".to_string());
            None
        }
    }
}
