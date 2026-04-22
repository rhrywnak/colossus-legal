//! PATCH /api/admin/pipeline/documents/:id/config — update per-document
//! override columns on `pipeline_config`.
//!
//! Covers the runtime "tweak a single doc's extraction parameters" flow
//! triggered by the Process-tab Configuration Panel. All body fields are
//! optional; fields left unset preserve the existing column value.
//!
//! Design: DOC_PROCESSING_CONFIG_DESIGN_v2.md Section 3.2.3.

use axum::{
    extract::{Path as AxumPath, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::pipeline::config::PipelineConfigOverrides;
use crate::repositories::pipeline_repository::{self, PipelineRepoError};
use crate::state::AppState;

/// PATCH request body / GET response body. Mirrors [`PipelineConfigOverrides`].
///
/// Every field is optional — omitting a field preserves the existing
/// column value. Passing `null` would currently be deserialised the same
/// as omission (the column stays). Explicit-clear semantics can be added
/// later if needed.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct PatchConfigInput {
    #[serde(default)]
    pub profile_name: Option<String>,
    #[serde(default)]
    pub extraction_model: Option<String>,
    #[serde(default)]
    pub template_file: Option<String>,
    #[serde(default)]
    pub system_prompt_file: Option<String>,
    #[serde(default)]
    pub chunking_mode: Option<String>,
    #[serde(default)]
    pub chunk_size: Option<i32>,
    #[serde(default)]
    pub chunk_overlap: Option<i32>,
    #[serde(default)]
    pub max_tokens: Option<i32>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub run_pass2: Option<bool>,
}

impl From<PatchConfigInput> for PipelineConfigOverrides {
    fn from(input: PatchConfigInput) -> Self {
        PipelineConfigOverrides {
            profile_name: input.profile_name,
            extraction_model: input.extraction_model,
            template_file: input.template_file,
            system_prompt_file: input.system_prompt_file,
            chunking_mode: input.chunking_mode,
            chunk_size: input.chunk_size,
            chunk_overlap: input.chunk_overlap,
            max_tokens: input.max_tokens,
            temperature: input.temperature,
            run_pass2: input.run_pass2,
        }
    }
}

/// GET /api/admin/pipeline/documents/:id/config — read per-document overrides.
///
/// Returns the `pipeline_config` row's nullable override columns as a
/// [`PatchConfigInput`]-shaped JSON body — symmetric with what the PATCH
/// endpoint accepts. The Configuration Panel uses this to seed its initial
/// state after upload so the user sees the auto-populated profile values
/// instead of frontend fallbacks.
///
/// Returns `404` if no `pipeline_config` row exists for the document —
/// a GET for a nonexistent document should not silently return all-null fields.
pub async fn get_config_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
) -> Result<Json<PatchConfigInput>, AppError> {
    require_admin(&user)?;

    let exists = pipeline_repository::get_pipeline_config(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to read pipeline_config: {e}"),
        })?;
    if exists.is_none() {
        return Err(AppError::NotFound {
            message: format!("No pipeline_config for document '{doc_id}'"),
        });
    }

    let overrides = pipeline_repository::get_pipeline_config_overrides(
        &state.pipeline_pool,
        &doc_id,
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to read pipeline_config overrides: {e}"),
    })?;

    Ok(Json(PatchConfigInput {
        profile_name: overrides.profile_name,
        extraction_model: overrides.extraction_model,
        template_file: overrides.template_file,
        system_prompt_file: overrides.system_prompt_file,
        chunking_mode: overrides.chunking_mode,
        chunk_size: overrides.chunk_size,
        chunk_overlap: overrides.chunk_overlap,
        max_tokens: overrides.max_tokens,
        temperature: overrides.temperature,
        run_pass2: overrides.run_pass2,
    }))
}

/// PATCH /api/admin/pipeline/documents/:id/config — partial update.
///
/// Returns `{"updated": true}` on success, `404` if no `pipeline_config`
/// row exists for the document.
pub async fn patch_config_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
    Json(input): Json<PatchConfigInput>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&user)?;

    let overrides: PipelineConfigOverrides = input.into();

    pipeline_repository::patch_pipeline_config_overrides(
        &state.pipeline_pool,
        &doc_id,
        &overrides,
    )
    .await
    .map_err(|e| match e {
        PipelineRepoError::NotFound(id) => AppError::NotFound {
            message: format!("No pipeline_config for document '{id}'"),
        },
        PipelineRepoError::Database(msg) => AppError::Internal {
            message: format!("Failed to patch pipeline_config: {msg}"),
        },
    })?;

    Ok(Json(serde_json::json!({"updated": true})))
}
