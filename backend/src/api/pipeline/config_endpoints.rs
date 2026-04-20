//! Configuration discovery + admin CRUD endpoints.
//!
//! Models live in the `llm_models` database table. Schemas and templates
//! live on the filesystem (YAML / Markdown) because they are authored
//! artifacts, not runtime state.
//!
//! Design: DOC_PROCESSING_CONFIG_DESIGN_v2.md Sections 3.4.1 and 3.2.1.

use axum::{
    extract::{Path as AxumPath, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::pipeline::config::ProcessingProfile;
use crate::repositories::pipeline_repository::models::{
    self, InsertModelInput, LlmModelRecord, UpdateModelInput,
};
use crate::state::AppState;

// ── Models: constants and DTOs ──────────────────────────────────

/// Providers the admin API accepts for a model's `provider` column.
/// Matches the dispatch in `backend/src/pipeline/providers.rs`; additions
/// here must be mirrored there (and vice-versa).
const ALLOWED_PROVIDERS: &[&str] = &["anthropic", "vllm", "openai"];

/// Postgres SQLSTATE for unique-constraint violations.
const UNIQUE_VIOLATION: &str = "23505";

#[derive(Debug, Serialize)]
pub struct ModelsResponse {
    pub models: Vec<LlmModelRecord>,
}

/// Body of POST /models — create a new model.
///
/// Mirrors `InsertModelInput` with an identical shape; defined as a
/// separate HTTP-layer DTO so the API contract can evolve independently
/// of the repository layer.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateModelInput {
    pub id: String,
    pub display_name: String,
    pub provider: String,
    #[serde(default)]
    pub api_endpoint: Option<String>,
    #[serde(default)]
    pub max_context_tokens: Option<i32>,
    #[serde(default)]
    pub max_output_tokens: Option<i32>,
    #[serde(default)]
    pub cost_per_input_token: Option<f64>,
    #[serde(default)]
    pub cost_per_output_token: Option<f64>,
    #[serde(default)]
    pub notes: Option<String>,
}

// ── Models: handlers ────────────────────────────────────────────

/// GET /api/admin/pipeline/models — list every model (active and inactive).
///
/// Admin UIs need the full set so operators can re-activate deactivated
/// models. The runtime extraction path uses `list_active_models` instead.
pub async fn list_models(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ModelsResponse>, AppError> {
    require_admin(&user)?;

    let rows = models::list_all_models(&state.pipeline_pool)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to list models: {e}"),
        })?;

    Ok(Json(ModelsResponse { models: rows }))
}

/// POST /api/admin/pipeline/models — create a new model row.
///
/// Validates the id is non-empty and the provider is one of
/// [`ALLOWED_PROVIDERS`]. Returns `409 Conflict` if the id already exists.
pub async fn create_model(
    user: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<CreateModelInput>,
) -> Result<Json<LlmModelRecord>, AppError> {
    require_admin(&user)?;

    let id = input.id.trim();
    if id.is_empty() {
        return Err(AppError::BadRequest {
            message: "Model id must not be empty".into(),
            details: serde_json::json!({"field": "id"}),
        });
    }
    if !ALLOWED_PROVIDERS.contains(&input.provider.as_str()) {
        return Err(AppError::BadRequest {
            message: format!(
                "Unknown provider '{}'; expected one of: {}",
                input.provider,
                ALLOWED_PROVIDERS.join(", ")
            ),
            details: serde_json::json!({"field": "provider"}),
        });
    }

    let repo_input = InsertModelInput {
        id: id.to_string(),
        display_name: input.display_name,
        provider: input.provider,
        api_endpoint: input.api_endpoint,
        max_context_tokens: input.max_context_tokens,
        max_output_tokens: input.max_output_tokens,
        cost_per_input_token: input.cost_per_input_token,
        cost_per_output_token: input.cost_per_output_token,
        notes: input.notes,
    };

    match models::insert_model(&state.pipeline_pool, &repo_input).await {
        Ok(record) => Ok(Json(record)),
        Err(e) => Err(map_insert_error(e, id)),
    }
}

/// PUT /api/admin/pipeline/models/:id — patch a model's fields.
///
/// Any field omitted from the request body is left unchanged (`COALESCE`
/// in the repository). Returns `404 Not Found` if the id does not exist.
pub async fn update_model(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(model_id): AxumPath<String>,
    Json(input): Json<UpdateModelInput>,
) -> Result<Json<LlmModelRecord>, AppError> {
    require_admin(&user)?;

    if let Some(provider) = input.provider.as_deref() {
        if !ALLOWED_PROVIDERS.contains(&provider) {
            return Err(AppError::BadRequest {
                message: format!(
                    "Unknown provider '{provider}'; expected one of: {}",
                    ALLOWED_PROVIDERS.join(", ")
                ),
                details: serde_json::json!({"field": "provider"}),
            });
        }
    }

    let updated = models::update_model(&state.pipeline_pool, &model_id, &input)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update model '{model_id}': {e}"),
        })?;

    updated.map(Json).ok_or_else(|| AppError::NotFound {
        message: format!("Model '{model_id}' not found"),
    })
}

/// DELETE /api/admin/pipeline/models/:id — delete a model row.
///
/// Refuses the delete (`409 Conflict`) if any profile YAML under
/// `processing_profile_dir` references this model's id. The check is
/// filesystem-based because profiles live on disk, not in the database.
pub async fn delete_model(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(model_id): AxumPath<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&user)?;

    let referencing = profiles_referencing_model(
        &state.config.processing_profile_dir,
        &model_id,
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Failed to scan profile directory: {e}"),
    })?;

    if !referencing.is_empty() {
        return Err(AppError::Conflict {
            message: format!(
                "Model '{model_id}' is referenced by {} profile(s)",
                referencing.len()
            ),
            details: serde_json::json!({"referenced_by": referencing}),
        });
    }

    let deleted = models::delete_model(&state.pipeline_pool, &model_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to delete model '{model_id}': {e}"),
        })?;

    if !deleted {
        return Err(AppError::NotFound {
            message: format!("Model '{model_id}' not found"),
        });
    }

    Ok(Json(serde_json::json!({"deleted": model_id})))
}

/// PUT /api/admin/pipeline/models/:id/toggle — flip `is_active`.
///
/// Returns the updated row. `404 Not Found` if the id does not exist.
pub async fn toggle_model(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(model_id): AxumPath<String>,
) -> Result<Json<LlmModelRecord>, AppError> {
    require_admin(&user)?;

    let updated = models::toggle_model_active(&state.pipeline_pool, &model_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to toggle model '{model_id}': {e}"),
        })?;

    updated.map(Json).ok_or_else(|| AppError::NotFound {
        message: format!("Model '{model_id}' not found"),
    })
}

// ── Models: helpers ─────────────────────────────────────────────

/// Map an `sqlx::Error` from `insert_model` into the correct HTTP error.
///
/// A `23505` unique-violation surfaces as `409 Conflict`; anything else
/// is an unexpected internal failure.
fn map_insert_error(e: sqlx::Error, id: &str) -> AppError {
    if let sqlx::Error::Database(db_err) = &e {
        if db_err.code().as_deref() == Some(UNIQUE_VIOLATION) {
            return AppError::Conflict {
                message: format!("Model '{id}' already exists"),
                details: serde_json::json!({"id": id}),
            };
        }
    }
    AppError::Internal {
        message: format!("Failed to insert model: {e}"),
    }
}

/// Return a list of profile filenames under `profile_dir` that mention
/// `model_id` anywhere in their content.
///
/// The match is a simple substring scan — it's intentionally loose because
/// profiles use `extraction_model: <id>` (and optionally `synthesis_model:`),
/// and a substring hit is sufficient to block a delete. False positives are
/// rare and erring on the side of "don't delete" is the correct bias.
async fn profiles_referencing_model(
    profile_dir: &str,
    model_id: &str,
) -> Result<Vec<String>, std::io::Error> {
    let mut matches = Vec::new();

    let mut entries = match tokio::fs::read_dir(profile_dir).await {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(matches),
        Err(e) => return Err(e),
    };

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }
        let content = match tokio::fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(
                    file = %path.display(),
                    error = %e,
                    "Failed to read profile file while scanning for model references (skipping)"
                );
                continue;
            }
        };
        if content.contains(model_id) {
            matches.push(entry.file_name().to_string_lossy().to_string());
        }
    }

    matches.sort();
    Ok(matches)
}

// ── Schemas endpoint ────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SchemasResponse {
    pub schemas: Vec<SchemaInfo>,
}

#[derive(Debug, Serialize)]
pub struct SchemaInfo {
    pub filename: String,
    pub document_type: String,
    pub version: String,
    pub description: String,
    pub entity_type_count: usize,
    pub entity_types: Vec<String>,
}

/// GET /api/admin/pipeline/schemas — list available extraction schemas.
pub async fn list_schemas(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<SchemasResponse>, AppError> {
    require_admin(&user)?;

    let schema_dir = &state.config.extraction_schema_dir;
    let mut schemas = Vec::new();

    let mut entries = tokio::fs::read_dir(schema_dir)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to read schema directory: {e}"),
        })?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| AppError::Internal {
        message: format!("Failed to read directory entry: {e}"),
    })? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
            let filename = entry.file_name().to_string_lossy().to_string();
            match colossus_extract::ExtractionSchema::from_file(&path) {
                Ok(schema) => {
                    let entity_types: Vec<String> = schema
                        .entity_types
                        .iter()
                        .map(|et| et.name.clone())
                        .collect();
                    schemas.push(SchemaInfo {
                        filename,
                        document_type: schema.document_type,
                        version: schema.version,
                        description: schema.description,
                        entity_type_count: entity_types.len(),
                        entity_types,
                    });
                }
                Err(e) => {
                    tracing::warn!(file = %filename, error = %e, "Skipping invalid schema file");
                }
            }
        }
    }

    schemas.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(Json(SchemasResponse { schemas }))
}

// ── Templates endpoint ──────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct TemplatesResponse {
    pub templates: Vec<TemplateInfo>,
}

#[derive(Debug, Serialize)]
pub struct TemplateInfo {
    pub filename: String,
    pub preview: String,
    pub size_bytes: u64,
}

/// GET /api/admin/pipeline/templates — list available prompt templates.
pub async fn list_templates(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<TemplatesResponse>, AppError> {
    require_admin(&user)?;

    let template_dir = &state.config.extraction_template_dir;
    let mut templates = Vec::new();

    let mut entries = tokio::fs::read_dir(template_dir)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to read template directory: {e}"),
        })?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| AppError::Internal {
        message: format!("Failed to read directory entry: {e}"),
    })? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let filename = entry.file_name().to_string_lossy().to_string();
            let metadata = entry.metadata().await.map_err(|e| AppError::Internal {
                message: format!("Failed to read file metadata: {e}"),
            })?;
            let content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
            let preview: String = content.chars().take(500).collect();
            templates.push(TemplateInfo {
                filename,
                preview,
                size_bytes: metadata.len(),
            });
        }
    }

    templates.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(Json(TemplatesResponse { templates }))
}

// ── Profiles: constants, DTOs, and validation ───────────────────

/// File-extension constants for profile YAML files.
const PROFILE_EXT: &str = "yaml";
const INACTIVE_EXT: &str = "yaml.inactive";

/// Chunking modes accepted in a `ProcessingProfile.chunking_mode`.
/// Mirrors the dispatch in `backend/src/pipeline/steps/llm_extract.rs`.
const ALLOWED_CHUNKING_MODES: &[&str] = &["full", "chunked"];

#[derive(Debug, Serialize)]
pub struct ProfilesResponse {
    pub profiles: Vec<ProcessingProfile>,
}

/// Whitelist-validate a profile name to prevent path traversal.
///
/// Accepts only `[A-Za-z0-9_-]+`. The name ends up in a filesystem path
/// (`{dir}/{name}.yaml`), so rejecting anything else is the safest gate:
/// no dots, no slashes, no backslashes, no null bytes, no empty string.
fn validate_profile_name(name: &str) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::BadRequest {
            message: "Profile name must not be empty".into(),
            details: serde_json::json!({"field": "name"}),
        });
    }
    let ok = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if !ok {
        return Err(AppError::BadRequest {
            message: format!(
                "Invalid profile name '{name}' — only letters, digits, '_' and '-' are allowed"
            ),
            details: serde_json::json!({"field": "name"}),
        });
    }
    Ok(())
}

/// Build the on-disk path for a profile file (`.yaml` or `.yaml.inactive`).
fn profile_path(dir: &str, name: &str, ext: &str) -> std::path::PathBuf {
    std::path::Path::new(dir).join(format!("{name}.{ext}"))
}

/// Validate a profile's cross-references before writing it to disk.
///
/// - `chunking_mode` must be in [`ALLOWED_CHUNKING_MODES`]
/// - `extraction_model` must exist in `llm_models` AND be active
/// - `schema_file` must exist under `extraction_schema_dir`
/// - `template_file` must exist under `extraction_template_dir`
///
/// Returns `BadRequest` for any violation with a `details.field` pointing
/// the admin UI at the offending input.
async fn validate_profile_references(
    state: &AppState,
    profile: &ProcessingProfile,
) -> Result<(), AppError> {
    if !ALLOWED_CHUNKING_MODES.contains(&profile.chunking_mode.as_str()) {
        return Err(AppError::BadRequest {
            message: format!(
                "Invalid chunking_mode '{}' — expected one of: {}",
                profile.chunking_mode,
                ALLOWED_CHUNKING_MODES.join(", ")
            ),
            details: serde_json::json!({"field": "chunking_mode"}),
        });
    }

    let model = models::get_active_model_by_id(&state.pipeline_pool, &profile.extraction_model)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to look up model: {e}"),
        })?;
    if model.is_none() {
        return Err(AppError::BadRequest {
            message: format!(
                "extraction_model '{}' not found or inactive in llm_models",
                profile.extraction_model
            ),
            details: serde_json::json!({"field": "extraction_model"}),
        });
    }

    let schema_path =
        std::path::Path::new(&state.config.extraction_schema_dir).join(&profile.schema_file);
    if !tokio::fs::try_exists(&schema_path).await.unwrap_or(false) {
        return Err(AppError::BadRequest {
            message: format!(
                "schema_file '{}' not found in extraction_schema_dir",
                profile.schema_file
            ),
            details: serde_json::json!({"field": "schema_file"}),
        });
    }

    let template_path =
        std::path::Path::new(&state.config.extraction_template_dir).join(&profile.template_file);
    if !tokio::fs::try_exists(&template_path).await.unwrap_or(false) {
        return Err(AppError::BadRequest {
            message: format!(
                "template_file '{}' not found in extraction_template_dir",
                profile.template_file
            ),
            details: serde_json::json!({"field": "template_file"}),
        });
    }

    Ok(())
}

// ── Profiles: handlers ──────────────────────────────────────────

/// GET /api/admin/pipeline/profiles — list every active profile.
///
/// Scans `processing_profile_dir` for `.yaml` files (skipping
/// `.yaml.inactive`). A parse failure on one file is logged and skipped
/// so one malformed profile doesn't hide the rest.
pub async fn list_profiles(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ProfilesResponse>, AppError> {
    require_admin(&user)?;

    let dir = &state.config.processing_profile_dir;
    let mut profiles = Vec::new();

    let mut entries = tokio::fs::read_dir(dir)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to read profile directory: {e}"),
        })?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| AppError::Internal {
        message: format!("Failed to read directory entry: {e}"),
    })? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some(PROFILE_EXT) {
            continue;
        }
        match ProcessingProfile::from_file(&path) {
            Ok(p) => profiles.push(p),
            Err(e) => tracing::warn!(
                file = %path.display(),
                error = %e,
                "Skipping invalid profile file"
            ),
        }
    }

    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Json(ProfilesResponse { profiles }))
}

/// GET /api/admin/pipeline/profiles/:name — read a single profile.
///
/// `404 Not Found` if `{name}.yaml` doesn't exist.
pub async fn get_profile(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(name): AxumPath<String>,
) -> Result<Json<ProcessingProfile>, AppError> {
    require_admin(&user)?;
    validate_profile_name(&name)?;

    let path = profile_path(&state.config.processing_profile_dir, &name, PROFILE_EXT);
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Err(AppError::NotFound {
            message: format!("Profile '{name}' not found"),
        });
    }
    let profile = ProcessingProfile::from_file(&path).map_err(|e| AppError::Internal {
        message: format!("Failed to load profile '{name}': {e}"),
    })?;
    Ok(Json(profile))
}

/// POST /api/admin/pipeline/profiles — create a new profile YAML file.
///
/// Validates the payload, refuses to overwrite an existing file (409),
/// then writes the YAML to disk. The body-provided `profile.name` is the
/// source of truth; it must pass [`validate_profile_name`] and cannot
/// collide with an existing `.yaml` file under the profile directory.
pub async fn create_profile(
    user: AuthUser,
    State(state): State<AppState>,
    Json(profile): Json<ProcessingProfile>,
) -> Result<Json<ProcessingProfile>, AppError> {
    require_admin(&user)?;
    validate_profile_name(&profile.name)?;
    validate_profile_references(&state, &profile).await?;

    let path = profile_path(&state.config.processing_profile_dir, &profile.name, PROFILE_EXT);
    if tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Err(AppError::Conflict {
            message: format!("Profile '{}' already exists", profile.name),
            details: serde_json::json!({"name": profile.name}),
        });
    }

    write_profile_yaml(&path, &profile).await?;
    Ok(Json(profile))
}

/// PUT /api/admin/pipeline/profiles/:name — overwrite an existing profile.
///
/// `404` if the file doesn't exist. The path param is authoritative —
/// if the body contains a different `profile.name`, the file at `:name`
/// is still the one overwritten. This avoids a "rename via update" foot-gun.
pub async fn update_profile(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(name): AxumPath<String>,
    Json(mut profile): Json<ProcessingProfile>,
) -> Result<Json<ProcessingProfile>, AppError> {
    require_admin(&user)?;
    validate_profile_name(&name)?;

    let path = profile_path(&state.config.processing_profile_dir, &name, PROFILE_EXT);
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Err(AppError::NotFound {
            message: format!("Profile '{name}' not found"),
        });
    }

    // Path param wins — overwrite the body name so the on-disk file and
    // its internal `name` field stay in sync.
    profile.name = name;
    validate_profile_references(&state, &profile).await?;

    write_profile_yaml(&path, &profile).await?;
    Ok(Json(profile))
}

/// DELETE /api/admin/pipeline/profiles/:name — deactivate a profile.
///
/// Renames `{name}.yaml` → `{name}.yaml.inactive`. Never hard-deletes —
/// admins can restore by renaming back. `404` if the source doesn't exist;
/// if the destination already exists, it is overwritten (re-deactivate).
pub async fn deactivate_profile(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(name): AxumPath<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&user)?;
    validate_profile_name(&name)?;

    let src = profile_path(&state.config.processing_profile_dir, &name, PROFILE_EXT);
    if !tokio::fs::try_exists(&src).await.unwrap_or(false) {
        return Err(AppError::NotFound {
            message: format!("Profile '{name}' not found"),
        });
    }
    let dst = profile_path(&state.config.processing_profile_dir, &name, INACTIVE_EXT);

    tokio::fs::rename(&src, &dst)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to rename profile '{name}': {e}"),
        })?;

    Ok(Json(serde_json::json!({
        "deactivated": name,
        "renamed_to": dst.file_name().map(|n| n.to_string_lossy().into_owned()),
    })))
}

/// Serialize a profile to YAML and write it atomically-enough for our use.
///
/// Uses `tokio::fs::write` (truncate-and-write). Concurrent edits by two
/// admins racing on the same profile is not something we defend against
/// at this layer — the UI serializes edits per profile.
async fn write_profile_yaml(
    path: &std::path::Path,
    profile: &ProcessingProfile,
) -> Result<(), AppError> {
    let yaml = serde_yaml::to_string(profile).map_err(|e| AppError::Internal {
        message: format!("Failed to serialize profile: {e}"),
    })?;
    tokio::fs::write(path, yaml).await.map_err(|e| AppError::Internal {
        message: format!("Failed to write profile '{}': {e}", path.display()),
    })
}
