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
use std::collections::HashMap;

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
///
/// `schema_file` is a GET-only field — sourced from the base
/// `pipeline_config.schema_file` column (not an override) so the Process
/// tab's completed card can show which schema a run used. The PATCH
/// handler's `From<PatchConfigInput> for PipelineConfigOverrides` does
/// not propagate `schema_file`, so posting it has no effect.
///
/// ## `#[serde(deny_unknown_fields)]`
///
/// A typo in a PATCH body (e.g. `"chunkign_config"` instead of
/// `"chunking_config"`) used to silently drop the unrecognised key and
/// return 200 OK with no effect — exactly the silent-fail mode the
/// audit gap report flagged. With this attribute, an unknown field
/// returns 400 from serde's deserializer so the operator sees the typo
/// immediately instead of debugging a "why didn't my override land"
/// mystery later.
///
/// ## Behavior change vs. earlier versions
///
/// Any client that was sending unrecognised fields (including
/// forward-compat fields it expected to be ignored) will now get a 400
/// instead of silent acceptance. The Configuration Panel — the only
/// known PATCH caller — sends exactly the fields below.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PatchConfigInput {
    #[serde(default)]
    pub profile_name: Option<String>,
    #[serde(default)]
    pub extraction_model: Option<String>,
    #[serde(default)]
    pub pass2_extraction_model: Option<String>,
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
    /// Per-document `chunking_config` override — merged on top of the
    /// profile's map at the *key level* by `resolve_config`. `None`
    /// means "no override; use the profile's map verbatim." See
    /// [`PipelineConfigOverrides::chunking_config`] for the
    /// three-state contract (None vs Some(empty) vs Some(non-empty)).
    #[serde(default)]
    pub chunking_config: Option<HashMap<String, serde_json::Value>>,
    /// Per-document `context_config` override. Same shape and contract
    /// as `chunking_config`.
    #[serde(default)]
    pub context_config: Option<HashMap<String, serde_json::Value>>,
    /// GET-only: base `pipeline_config.schema_file`. PATCH ignores this.
    #[serde(default)]
    pub schema_file: Option<String>,
}

impl From<PatchConfigInput> for PipelineConfigOverrides {
    fn from(input: PatchConfigInput) -> Self {
        PipelineConfigOverrides {
            profile_name: input.profile_name,
            extraction_model: input.extraction_model,
            pass2_extraction_model: input.pass2_extraction_model,
            template_file: input.template_file,
            system_prompt_file: input.system_prompt_file,
            chunking_mode: input.chunking_mode,
            chunk_size: input.chunk_size,
            chunk_overlap: input.chunk_overlap,
            max_tokens: input.max_tokens,
            temperature: input.temperature,
            run_pass2: input.run_pass2,
            chunking_config: input.chunking_config,
            context_config: input.context_config,
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

    let base_config = pipeline_repository::get_pipeline_config(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to read pipeline_config: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("No pipeline_config for document '{doc_id}'"),
        })?;

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
        pass2_extraction_model: overrides.pass2_extraction_model,
        template_file: overrides.template_file,
        system_prompt_file: overrides.system_prompt_file,
        chunking_mode: overrides.chunking_mode,
        chunk_size: overrides.chunk_size,
        chunk_overlap: overrides.chunk_overlap,
        max_tokens: overrides.max_tokens,
        temperature: overrides.temperature,
        run_pass2: overrides.run_pass2,
        chunking_config: overrides.chunking_config,
        context_config: overrides.context_config,
        schema_file: Some(base_config.schema_file),
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
        // The patch path never returns Deserialization today (the
        // helper only writes JSONB, never reads it), but exhaustive
        // matching on the enum keeps this arm honest if the path
        // ever gains a read step. Treat as 500 — it's a server-side
        // data-shape bug, not something the client can recover from.
        PipelineRepoError::Deserialization(msg) => AppError::Internal {
            message: format!("Failed to deserialize pipeline_config: {msg}"),
        },
    })?;

    Ok(Json(serde_json::json!({"updated": true})))
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Test #12: PATCH body with chunking_config map parses cleanly.
    #[test]
    fn patch_input_parses_chunking_config_map() {
        let body = r#"{"chunking_config": {"units_per_chunk": 3, "strategy": "qa_pair"}}"#;
        let input: PatchConfigInput =
            serde_json::from_str(body).expect("body must parse");
        let chunking = input.chunking_config.expect("chunking_config populated");
        assert_eq!(
            chunking.get("units_per_chunk").and_then(|v| v.as_i64()),
            Some(3)
        );
        assert_eq!(
            chunking.get("strategy").and_then(|v| v.as_str()),
            Some("qa_pair")
        );
        assert!(input.context_config.is_none());
    }

    // Test #13: PATCH body omitting both maps parses cleanly with both
    // fields as None (the no-override path that `#[serde(default)]`
    // exists to support).
    #[test]
    fn patch_input_parses_with_neither_map_present() {
        let body = r#"{"temperature": 0.2}"#;
        let input: PatchConfigInput =
            serde_json::from_str(body).expect("body must parse");
        assert!(input.chunking_config.is_none());
        assert!(input.context_config.is_none());
        assert_eq!(input.temperature, Some(0.2));
    }

    // PATCH body with context_config also parses (mirror of #12).
    #[test]
    fn patch_input_parses_context_config_map() {
        let body = r#"{"context_config": {"traversal_depth": 5}}"#;
        let input: PatchConfigInput =
            serde_json::from_str(body).expect("body must parse");
        let context = input.context_config.expect("context_config populated");
        assert_eq!(
            context.get("traversal_depth").and_then(|v| v.as_i64()),
            Some(5)
        );
    }

    // Decision #3: deny_unknown_fields rejects typos. A request body
    // like `{"chunkign_config": {...}}` (typo on the field name) used
    // to drop the field silently and return 200 OK with no effect.
    // Now serde returns a deserialization error, which the Axum layer
    // surfaces as 400 Bad Request — operator sees the typo immediately.
    #[test]
    fn patch_input_rejects_unknown_field_via_deny_unknown_fields() {
        let body = r#"{"chunkign_config": {"units_per_chunk": 3}}"#; // typo: chunkign vs chunking
        let result = serde_json::from_str::<PatchConfigInput>(body);
        let err = result.expect_err("typo'd field must be rejected, not silently dropped");
        let msg = err.to_string();
        assert!(
            msg.contains("chunkign_config"),
            "error must name the unknown field; got: {msg}"
        );
        assert!(
            msg.contains("unknown field"),
            "error must say 'unknown field'; got: {msg}"
        );
    }

    // From<PatchConfigInput> for PipelineConfigOverrides must propagate
    // both new fields (post-Instruction-C; before this change the impl
    // hardcoded both to None — that was the silent-drop bug Gap 1
    // identified).
    #[test]
    fn from_impl_propagates_chunking_and_context_config() {
        let mut chunking = HashMap::new();
        chunking.insert("units_per_chunk".to_string(), serde_json::json!(3));
        let mut context = HashMap::new();
        context.insert("traversal_depth".to_string(), serde_json::json!(5));
        let input = PatchConfigInput {
            chunking_config: Some(chunking.clone()),
            context_config: Some(context.clone()),
            ..Default::default()
        };
        let overrides: PipelineConfigOverrides = input.into();
        assert_eq!(overrides.chunking_config.as_ref(), Some(&chunking));
        assert_eq!(overrides.context_config.as_ref(), Some(&context));
    }
}
