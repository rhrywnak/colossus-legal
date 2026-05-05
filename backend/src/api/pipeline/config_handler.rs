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
use crate::pipeline::config::{
    resolve_config, PipelineConfigOverrides, ProcessingProfile, ResolvedConfig,
};
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
    /// Per-document Pass 2 (synthesis) template override. Populated from
    /// the `pass2_template_file` column on `pipeline_config`. `None`
    /// means "use the profile default."
    #[serde(default)]
    pub pass2_template_file: Option<String>,
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
            pass2_template_file: input.pass2_template_file,
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
        pass2_template_file: overrides.pass2_template_file,
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

/// Compose the resolved-config payload: run [`resolve_config`] on the
/// profile + overrides, then overwrite `schema_file` with the
/// authoritative base value from `pipeline_config.schema_file`.
///
/// ## Why the schema_file overwrite
///
/// `resolve_config()` reads `schema_file` from the profile (since
/// `PipelineConfigOverrides` does not carry a `schema_file` field — by
/// architectural design, schema_file is the *base* `pipeline_config`
/// column, not an override). When the profile loaded for resolution
/// doesn't match the document's actual configuration (the bug we lived
/// through with v4 vs v5 complaints sharing `document_type=complaint`),
/// the resolved schema_file would be the profile's default, not the
/// document's persisted value.
///
/// The fix: after `resolve_config` runs, surface the persisted base
/// value as the authoritative schema_file in the resolved payload.
/// This preserves the base-vs-override architectural distinction
/// without expanding `PipelineConfigOverrides`.
fn build_resolved_config_payload(
    profile: &ProcessingProfile,
    overrides: &PipelineConfigOverrides,
    base_schema_file: &str,
) -> ResolvedConfig {
    let mut resolved = resolve_config(profile, overrides);
    resolved.schema_file = base_schema_file.to_string();
    resolved
}

/// GET /api/admin/pipeline/documents/:id/resolved-config — return the
/// fully resolved config a runtime extraction would use.
///
/// This is the backend authority for the audit-trail panel in the UI.
/// It replaces the broken client-side approach that matched profiles by
/// `document_type` (which fails when multiple schemas share a
/// document_type — e.g., v4 and v5 complaints both have
/// `document_type=complaint`).
///
/// Reads `pipeline_config` (for the persisted base values and
/// `profile_name`), loads the named profile from disk, runs
/// [`resolve_config`], overwrites `schema_file` with the persisted
/// base, and returns `ResolvedConfig` as JSON.
///
/// Errors:
/// - 404 if no `pipeline_config` row exists for the document.
/// - 404 if the row exists but has no `profile_name` (a malformed row;
///   uploads always populate it).
/// - 500 if the profile YAML fails to load (missing file, parse error).
pub async fn get_resolved_config_handler(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(doc_id): AxumPath<String>,
) -> Result<Json<ResolvedConfig>, AppError> {
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

    let profile_name = overrides.profile_name.as_deref().ok_or_else(|| AppError::NotFound {
        message: format!("Document '{doc_id}' has no profile_name on its pipeline_config row"),
    })?;

    let profile = ProcessingProfile::load(&state.config.processing_profile_dir, profile_name)
        .map_err(|e| AppError::Internal {
            message: format!("Failed to load profile '{profile_name}': {e}"),
        })?;

    let resolved = build_resolved_config_payload(&profile, &overrides, &base_config.schema_file);

    Ok(Json(resolved))
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

    // Catches the v5 deployment bug end-to-end at the resolved-config
    // payload level: profile says v4 (simulating the frontend-style
    // "match profile by document_type" failure mode where the wrong
    // profile gets loaded), pipeline_config has v5 base schema_file +
    // v5 pass2_template_file override (set by upload from the actual
    // v5 profile). The resolved payload must surface v5 for both fields.
    //
    // Before WI-FIX-2 + WI-FIX-3: pass2_template_file would be v4
    // (resolve_config ignored the override) and schema_file would be v4
    // (resolve_config read from profile, ignoring the persisted base).
    // Now both surface as v5.
    #[test]
    fn resolved_config_returns_v5_values_when_pipeline_config_has_v5_overrides() {
        // Profile YAML simulates the wrong-profile-loaded scenario:
        // labelled v5 but with v4 internals. The fields the bug was
        // about (schema_file, pass2_template_file) are v4 here.
        let yaml = r#"
name: complaint_v5
display_name: "Complaint v5 (test fixture)"
schema_file: complaint_v4.yaml
template_file: pass1_complaint_v4.md
pass2_template_file: pass2_complaint_v4.md
extraction_model: claude-sonnet-4-6
"#;
        let profile = ProcessingProfile::from_yaml_str(yaml).unwrap();

        // Overrides reflect what upload populated from the *actual*
        // v5 profile: pass2_template_file is v5 (the override
        // mechanism's job).
        let overrides = PipelineConfigOverrides {
            profile_name: Some("complaint_v5".into()),
            pass2_template_file: Some("pass2_complaint_v5.md".into()),
            ..Default::default()
        };

        // Base pipeline_config.schema_file is v5 (set at upload from
        // the actual v5 profile, persisted in the NOT NULL column).
        let base_schema_file = "complaint_v5.yaml";

        let resolved = build_resolved_config_payload(&profile, &overrides, base_schema_file);

        assert_eq!(
            resolved.schema_file, "complaint_v5.yaml",
            "resolved schema_file must come from the persisted base \
             pipeline_config.schema_file, not from the profile that \
             happened to be loaded"
        );
        assert_eq!(
            resolved.pass2_template_file.as_deref(),
            Some("pass2_complaint_v5.md"),
            "resolved pass2_template_file must come from the per-document \
             override, not from the profile default"
        );
    }
}
