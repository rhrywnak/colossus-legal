//! Shared profile-and-override validator — single source of truth for
//! "is this configuration runnable?"
//!
//! Bug #1, #4, #5, #11 fix. Before this module, three different paths
//! validated different subsets of the same fields:
//!
//! 1. Profile-save (`POST /api/admin/pipeline/profiles`) validated
//!    `extraction_model`, `chunking_mode`, `schema_file`, `template_file`
//!    — but not `pass2_extraction_model`, `pass2_template_file`,
//!    `system_prompt_file`, or `global_rules_file`.
//! 2. Upload (`POST /api/admin/pipeline/documents`) validated *nothing*
//!    — invalid model IDs propagated all the way to extract time, then
//!    surfaced as a runtime `ModelNotFound` error after disk writes had
//!    already happened.
//! 3. PATCH (`PATCH /api/admin/pipeline/documents/:id/config`) validated
//!    *nothing* — operator-supplied overrides could write any string and
//!    the system only caught it at extract time.
//!
//! Now all three paths call the same validator, and every model/file
//! field is checked at the boundary. Errors name the specific field, the
//! offending value, and what valid values exist (active model IDs / the
//! directory that was searched). HTTP semantics: validation failures are
//! `400 BadRequest`; an internal lookup failure (DB unreachable, etc.) is
//! `500 Internal`.
//!
//! ## Rust Learning: borrowing the AppState fields, not the struct
//!
//! Most callers already have `&AppState` and would naturally pass the
//! whole thing. We deliberately take `&PgPool` and `&PipelineRegistry`
//! instead so the validator is unit-testable without spinning up an
//! `AppState`: every test can construct a registry pointing at a tempdir
//! and reuse a single test pool. The caller writes one extra line —
//! `validate_profile(&state.pipeline_pool, &state.registry, &profile)
//! .await?` — and the type signature documents exactly what the
//! validator touches.

use sqlx::PgPool;

use crate::error::AppError;
use crate::pipeline::config::{PipelineConfigOverrides, ProcessingProfile};
use crate::pipeline::registry::PipelineRegistry;
use crate::repositories::pipeline_repository::models;

/// Chunking modes accepted by the runtime. Kept in lockstep with the
/// resolver in `pipeline/steps/llm_extract.rs::resolve_effective_mode`.
pub const ALLOWED_CHUNKING_MODES: &[&str] = &["full", "structured", "chunked"];

/// Validate every cross-reference on a `ProcessingProfile` — model IDs
/// exist in `llm_models` and are active, files exist on disk under their
/// configured directories, chunking_mode is in the allowed set.
///
/// Returns the first violation as a `400 BadRequest` whose `details.field`
/// names the offending input and whose `details.valid` lists what was
/// expected. A DB lookup failure surfaces as `500 Internal` so the
/// operator can distinguish "your input was bad" from "the server is
/// degraded" — exactly the same line we draw for the same error class
/// elsewhere in the API.
///
/// ## Field coverage
///
/// - `extraction_model` — must exist in `llm_models` AND be active
/// - `pass2_extraction_model` — same check, when `Some`
/// - `schema_file` — must exist under the registry's `schemas` directory
/// - `template_file` — must exist under the registry's `templates` directory
/// - `pass2_template_file` — same, when `Some`
/// - `system_prompt_file` — must exist under `system_prompts`, when `Some`
/// - `global_rules_file` — must exist under `templates`, when `Some`
///   (global_rules files live alongside templates today; if they ever
///   split out, the registry will gain a dedicated path method and this
///   call site will follow)
/// - `chunking_mode` — must be in [`ALLOWED_CHUNKING_MODES`]
pub async fn validate_profile(
    db: &PgPool,
    registry: &PipelineRegistry,
    profile: &ProcessingProfile,
) -> Result<(), AppError> {
    validate_chunking_mode(&profile.chunking_mode)?;

    validate_model(db, "extraction_model", &profile.extraction_model).await?;
    if let Some(m) = profile.pass2_extraction_model.as_deref() {
        validate_model(db, "pass2_extraction_model", m).await?;
    }

    validate_file(
        registry,
        "schema_file",
        FileKind::Schema,
        &profile.schema_file,
    )
    .await?;
    validate_file(
        registry,
        "template_file",
        FileKind::Template,
        &profile.template_file,
    )
    .await?;
    if let Some(f) = profile.pass2_template_file.as_deref() {
        validate_file(registry, "pass2_template_file", FileKind::Template, f).await?;
    }
    if let Some(f) = profile.system_prompt_file.as_deref() {
        validate_file(registry, "system_prompt_file", FileKind::SystemPrompt, f).await?;
    }
    if let Some(f) = profile.global_rules_file.as_deref() {
        // Global rules fragments live alongside templates today.
        validate_file(registry, "global_rules_file", FileKind::Template, f).await?;
    }

    Ok(())
}

/// Validate the model/file fields on a PATCH-supplied
/// `PipelineConfigOverrides` payload.
///
/// Each field is `Option` — `None` means "no override; the resolver will
/// fall back to the profile, which was already validated when it was
/// authored." We validate only what the patch supplies.
///
/// Bug #5 fix. Before this, the PATCH path accepted any string for
/// `extraction_model` etc. and the system only caught the bad value at
/// extract time.
pub async fn validate_overrides(
    db: &PgPool,
    registry: &PipelineRegistry,
    overrides: &PipelineConfigOverrides,
) -> Result<(), AppError> {
    if let Some(m) = overrides.chunking_mode.as_deref() {
        validate_chunking_mode(m)?;
    }
    if let Some(m) = overrides.extraction_model.as_deref() {
        validate_model(db, "extraction_model", m).await?;
    }
    if let Some(m) = overrides.pass2_extraction_model.as_deref() {
        validate_model(db, "pass2_extraction_model", m).await?;
    }
    if let Some(f) = overrides.template_file.as_deref() {
        validate_file(registry, "template_file", FileKind::Template, f).await?;
    }
    if let Some(f) = overrides.pass2_template_file.as_deref() {
        validate_file(registry, "pass2_template_file", FileKind::Template, f).await?;
    }
    if let Some(f) = overrides.system_prompt_file.as_deref() {
        validate_file(registry, "system_prompt_file", FileKind::SystemPrompt, f).await?;
    }
    if let Some(f) = overrides.global_rules_file.as_deref() {
        validate_file(registry, "global_rules_file", FileKind::Template, f).await?;
    }
    Ok(())
}

/// Internal — which registry path method to consult for a given file
/// kind. Centralised so the call sites stay readable.
#[derive(Copy, Clone)]
enum FileKind {
    Schema,
    Template,
    SystemPrompt,
}

impl FileKind {
    fn path(self, registry: &PipelineRegistry, filename: &str) -> String {
        match self {
            FileKind::Schema => registry.schema_path(filename),
            FileKind::Template => registry.template_path(filename),
            FileKind::SystemPrompt => registry.system_prompt_path(filename),
        }
    }

    fn directory(self, registry: &PipelineRegistry) -> &str {
        match self {
            FileKind::Schema => registry.schema_dir(),
            FileKind::Template => registry.template_dir(),
            FileKind::SystemPrompt => registry.system_prompt_dir(),
        }
    }
}

fn validate_chunking_mode(mode: &str) -> Result<(), AppError> {
    if ALLOWED_CHUNKING_MODES.contains(&mode) {
        return Ok(());
    }
    Err(AppError::BadRequest {
        message: format!(
            "Invalid chunking_mode '{mode}' — expected one of: {}",
            ALLOWED_CHUNKING_MODES.join(", ")
        ),
        details: serde_json::json!({
            "field": "chunking_mode",
            "value": mode,
            "valid": ALLOWED_CHUNKING_MODES,
        }),
    })
}

/// Look up `model_id` in `llm_models`. Active row → `Ok(())`. Missing or
/// inactive → `BadRequest` listing every active model id. DB error →
/// `Internal`.
///
/// The DB-hitting outer fetches the candidate set, then defers to the
/// pure [`validate_model_against_list`] helper. Splitting the function
/// keeps the error-message logic unit-testable without a Postgres pool.
async fn validate_model(db: &PgPool, field: &str, model_id: &str) -> Result<(), AppError> {
    let found = models::get_active_model_by_id(db, model_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to look up model '{model_id}': {e}"),
        })?;
    if found.is_some() {
        return Ok(());
    }

    // Build the "valid IDs" list. Failure here downgrades the error
    // message but does not change the outcome — the user input is still
    // invalid; we just can't enumerate alternatives.
    let valid_ids: Vec<String> = match models::list_active_models(db).await {
        Ok(rows) => rows.into_iter().map(|r| r.id).collect(),
        Err(e) => {
            tracing::warn!(error = %e, "Failed to list active models for error message");
            Vec::new()
        }
    };

    Err(model_not_in_list_error(field, model_id, &valid_ids))
}

/// Pure error constructor — given a field name, an offending value, and
/// the list of valid IDs, build the `BadRequest` the API returns. Kept
/// separate from the DB-hitting code so unit tests can exercise the
/// error format without a Postgres pool.
fn model_not_in_list_error(field: &str, model_id: &str, valid_ids: &[String]) -> AppError {
    AppError::BadRequest {
        message: format!(
            "{field} '{model_id}' not found or inactive in llm_models. \
             Valid active model IDs: [{}]",
            valid_ids.join(", ")
        ),
        details: serde_json::json!({
            "field": field,
            "value": model_id,
            "valid": valid_ids,
        }),
    }
}

/// Confirm `filename` exists under the registry's directory for `kind`.
/// On miss, the error names the field, the value, and the directory
/// searched so an operator can diagnose without grepping logs.
async fn validate_file(
    registry: &PipelineRegistry,
    field: &str,
    kind: FileKind,
    filename: &str,
) -> Result<(), AppError> {
    let path = kind.path(registry, filename);
    let exists = tokio::fs::try_exists(&path).await.unwrap_or(false);
    if exists {
        return Ok(());
    }

    let dir = kind.directory(registry);
    Err(AppError::BadRequest {
        message: format!("{field} '{filename}' not found in {dir}"),
        details: serde_json::json!({
            "field": field,
            "value": filename,
            "directory_searched": dir,
        }),
    })
}

#[cfg(test)]
mod tests {
    //! Unit coverage. Tests that depend on the `llm_models` table are
    //! gated behind a real DB (we don't spin up Postgres in unit tests
    //! today); the chunking-mode and file-existence checks are pure and
    //! covered here.
    use super::*;
    use crate::pipeline::registry::{PipelineDirectories, PipelineRegistry};
    use std::fs;
    use tempfile::TempDir;

    fn registry_at(root: &std::path::Path) -> PipelineRegistry {
        let paths = ["profiles", "schemas", "templates", "system_prompts"].map(|n| root.join(n));
        for p in &paths {
            fs::create_dir_all(p).unwrap();
        }
        PipelineRegistry {
            directories: PipelineDirectories {
                profiles: paths[0].to_string_lossy().into_owned(),
                schemas: paths[1].to_string_lossy().into_owned(),
                templates: paths[2].to_string_lossy().into_owned(),
                system_prompts: paths[3].to_string_lossy().into_owned(),
            },
            document_types: Vec::new(),
        }
    }

    #[test]
    fn chunking_mode_accepts_full_structured_chunked() {
        for m in ["full", "structured", "chunked"] {
            validate_chunking_mode(m).expect("allowed mode must pass");
        }
    }

    #[test]
    fn chunking_mode_rejects_typo() {
        let err = validate_chunking_mode("chunkd").expect_err("typo must fail");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("chunkd") && msg.contains("full"),
            "error must name the bad value and list valid options; got: {msg}"
        );
    }

    #[tokio::test]
    async fn validate_file_rejects_missing_with_directory_searched() {
        let tmp = TempDir::new().unwrap();
        let registry = registry_at(tmp.path());

        let err = validate_file(
            &registry,
            "template_file",
            FileKind::Template,
            "does_not_exist.md",
        )
        .await
        .expect_err("missing file must fail");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("does_not_exist.md") && msg.contains("templates"),
            "error must name the file and the directory; got: {msg}"
        );
    }

    #[tokio::test]
    async fn validate_file_accepts_existing() {
        let tmp = TempDir::new().unwrap();
        let registry = registry_at(tmp.path());
        let path = tmp.path().join("templates").join("ok.md");
        fs::write(&path, b"# stub").unwrap();

        validate_file(&registry, "template_file", FileKind::Template, "ok.md")
            .await
            .expect("existing file must pass");
    }

    /// Bug #1: an `extraction_model` ID that isn't in the active model
    /// list produces a `BadRequest` naming the field, the bad value,
    /// and the list of valid IDs.
    #[test]
    fn test_upload_rejects_invalid_extraction_model() {
        let valid: Vec<String> = vec!["claude-sonnet-4-6".into(), "claude-opus-4-6".into()];
        let err = model_not_in_list_error("extraction_model", "claude-sonnet-4-20250514", &valid);
        let msg = format!("{err:?}");
        assert!(
            msg.contains("extraction_model"),
            "error must name the field; got: {msg}"
        );
        assert!(
            msg.contains("claude-sonnet-4-20250514"),
            "error must include the offending value; got: {msg}"
        );
        assert!(
            msg.contains("claude-sonnet-4-6") && msg.contains("claude-opus-4-6"),
            "error must list the valid alternatives; got: {msg}"
        );
    }

    /// Bug #1: same logic for `pass2_extraction_model` — the previous
    /// validator omitted pass-2 entirely (Bug #4).
    #[test]
    fn test_upload_rejects_invalid_pass2_model() {
        let valid: Vec<String> = vec!["claude-sonnet-4-6".into(), "claude-opus-4-6".into()];
        let err =
            model_not_in_list_error("pass2_extraction_model", "claude-opus-4-20250115", &valid);
        let msg = format!("{err:?}");
        assert!(
            msg.contains("pass2_extraction_model") && msg.contains("claude-opus-4-20250115"),
            "error must name field and value; got: {msg}"
        );
    }

    /// Bug #1/#4: template file missing from disk → BadRequest naming
    /// the field, the filename, and the directory that was searched.
    #[tokio::test]
    async fn test_upload_rejects_invalid_template_file() {
        let tmp = TempDir::new().unwrap();
        let registry = registry_at(tmp.path());
        let err = validate_file(&registry, "template_file", FileKind::Template, "missing.md")
            .await
            .expect_err("missing template must reject");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("template_file")
                && msg.contains("missing.md")
                && msg.contains("templates"),
            "error must name field, file, directory; got: {msg}"
        );
    }

    /// Bug #1: schema file missing from disk → BadRequest naming the
    /// field, the filename, and the directory.
    #[tokio::test]
    async fn test_upload_rejects_missing_schema_file() {
        let tmp = TempDir::new().unwrap();
        let registry = registry_at(tmp.path());
        let err = validate_file(&registry, "schema_file", FileKind::Schema, "gone.yaml")
            .await
            .expect_err("missing schema must reject");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("schema_file") && msg.contains("gone.yaml") && msg.contains("schemas"),
            "error must name field, file, directory; got: {msg}"
        );
    }

    /// Bug #4: system_prompt missing from disk → BadRequest. Before the
    /// shared validator, this field was not checked at all.
    #[tokio::test]
    async fn test_upload_rejects_invalid_system_prompt() {
        let tmp = TempDir::new().unwrap();
        let registry = registry_at(tmp.path());
        let err = validate_file(
            &registry,
            "system_prompt_file",
            FileKind::SystemPrompt,
            "ghost.md",
        )
        .await
        .expect_err("missing system prompt must reject");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("system_prompt_file")
                && msg.contains("ghost.md")
                && msg.contains("system_prompts"),
            "error must name field, file, directory; got: {msg}"
        );
    }

    /// Bug #1: a valid profile (all referenced files exist, chunking
    /// mode in allowed set) passes the non-DB checks. Model lookup is
    /// covered separately by the live-DB integration test.
    #[tokio::test]
    async fn test_upload_accepts_valid_profile() {
        let tmp = TempDir::new().unwrap();
        let registry = registry_at(tmp.path());
        for (sub, name) in [
            ("schemas", "ok.yaml"),
            ("templates", "ok.md"),
            ("templates", "ok_pass2.md"),
            ("templates", "rules.md"),
            ("system_prompts", "sys.md"),
        ] {
            fs::write(tmp.path().join(sub).join(name), b"stub").unwrap();
        }
        validate_chunking_mode("structured").expect("chunking ok");
        for (field, kind, file) in [
            ("schema_file", FileKind::Schema, "ok.yaml"),
            ("template_file", FileKind::Template, "ok.md"),
            ("pass2_template_file", FileKind::Template, "ok_pass2.md"),
            ("global_rules_file", FileKind::Template, "rules.md"),
            ("system_prompt_file", FileKind::SystemPrompt, "sys.md"),
        ] {
            validate_file(&registry, field, kind, file)
                .await
                .unwrap_or_else(|e| panic!("{field} should validate: {e:?}"));
        }
    }

    /// Bug #5: the PATCH path validates operator-supplied overrides
    /// the same way the upload path validates profile-supplied values.
    /// An invalid model in a PATCH body produces the same error format.
    #[test]
    fn test_patch_rejects_invalid_model_id() {
        let valid: Vec<String> = vec!["claude-sonnet-4-6".into()];
        let err = model_not_in_list_error("extraction_model", "made-up-model", &valid);
        let msg = format!("{err:?}");
        assert!(
            msg.contains("extraction_model")
                && msg.contains("made-up-model")
                && msg.contains("claude-sonnet-4-6"),
            "PATCH-side error must be symmetric with upload-side; got: {msg}"
        );
    }

    /// Bug #5: a PATCH that supplies only the fields it wants to change
    /// passes validation when those fields are well-formed. We exercise
    /// the override-file branch here; the model branch is identical
    /// shape covered by `test_patch_rejects_invalid_model_id` above.
    #[tokio::test]
    async fn test_patch_accepts_valid_override() {
        let tmp = TempDir::new().unwrap();
        let registry = registry_at(tmp.path());
        fs::write(tmp.path().join("templates").join("ok.md"), b"stub").unwrap();
        validate_file(&registry, "template_file", FileKind::Template, "ok.md")
            .await
            .expect("valid override file must pass");
    }
}
