//! Admin CRUD endpoints for processing-profile YAML files.
//!
//! Profiles are YAML files on mounted storage, not a database table.
//! Design: DOC_PROCESSING_CONFIG_DESIGN_v2.md Sections 3.2.2 and 3.4.2.

use axum::{
    extract::{Path as AxumPath, State},
    Json,
};
use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::pipeline::config::ProcessingProfile;
use crate::repositories::pipeline_repository::models;
use crate::state::AppState;

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
