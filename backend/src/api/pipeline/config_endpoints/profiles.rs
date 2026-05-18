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
use crate::pipeline::registry::PipelineRegistry;
use crate::pipeline::validation::validate_profile;
use crate::state::AppState;

/// File-extension constants for profile YAML files.
const PROFILE_EXT: &str = "yaml";
const INACTIVE_EXT: &str = "yaml.inactive";

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

/// Resolve a path-parameter name to its on-disk profile basename and full path.
///
/// The endpoint receives a path parameter that may be either:
///
/// 1. A registry **document_type** key (e.g. `"complaint"`) — the
///    UI's primary call shape. The registry maps it to a
///    `profile_file` like `"complaint_v5_1.yaml"`, and we return
///    the basename `"complaint_v5_1"` and the full file path.
/// 2. A literal **profile filename basename** (e.g. `"complaint_v5_1"`)
///    — admin tooling that already knows the filename. No registry
///    indirection applies and we pass it through.
///
/// Trying the registry first, then falling back to the literal name,
/// keeps both call shapes working. Mirrors the resolution upload.rs
/// performs on the document_type → profile_file mapping.
///
/// `ext` is the file extension without leading dot (`"yaml"` for
/// active profiles, `"yaml.inactive"` for deactivated ones).
fn resolve_profile_basename_and_path(
    registry: &PipelineRegistry,
    name: &str,
    ext: &str,
) -> (String, std::path::PathBuf) {
    if let Some(entry) = registry.document_type(name) {
        // Registry `profile_file` values are full filenames ending in
        // `.yaml`. Strip the suffix so the basename can be re-used as
        // the on-disk file's internal `name:` field (update_profile)
        // and so `profile_path` doesn't double-append the extension.
        let basename = entry
            .profile_file
            .strip_suffix(".yaml")
            .unwrap_or(&entry.profile_file)
            .to_string();
        let path = profile_path(registry.profile_dir(), &basename, ext);
        (basename, path)
    } else {
        let path = profile_path(registry.profile_dir(), name, ext);
        (name.to_string(), path)
    }
}

/// Validate a profile's cross-references before writing it to disk.
///
/// Bug #4 fix — used to omit `pass2_extraction_model`,
/// `pass2_template_file`, `system_prompt_file`, and `global_rules_file`
/// from the check. All cross-reference validation now lives in
/// [`crate::pipeline::validation::validate_profile`], shared with the
/// upload and PATCH paths so every entry point catches the same
/// inconsistencies with the same error format.
async fn validate_profile_references(
    state: &AppState,
    profile: &ProcessingProfile,
) -> Result<(), AppError> {
    validate_profile(&state.pipeline_pool, &state.registry, profile).await
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

    let dir = state.registry.profile_dir();
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
/// `:name` may be either a registry document_type (e.g. `"complaint"`)
/// or a literal profile filename basename (e.g. `"complaint_v5_1"`).
/// The registry mapping is tried first so the UI can fetch by
/// document_type without knowing the versioned profile filename.
///
/// `404 Not Found` if the resolved file doesn't exist.
pub async fn get_profile(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(name): AxumPath<String>,
) -> Result<Json<ProcessingProfile>, AppError> {
    require_admin(&user)?;
    validate_profile_name(&name)?;

    let (_basename, path) = resolve_profile_basename_and_path(&state.registry, &name, PROFILE_EXT);
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

    let path = profile_path(state.registry.profile_dir(), &profile.name, PROFILE_EXT);
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
/// `:name` may be a registry document_type or a literal profile filename
/// basename (same resolution as `get_profile`). The resolved filename is
/// authoritative — if the body contains a different `profile.name`, the
/// resolved file is the one overwritten and the body's `name` field is
/// rewritten to match the file basename. This avoids a "rename via update"
/// foot-gun and keeps the YAML's internal `name:` field in lockstep with
/// its filename.
///
/// `404` if the resolved file doesn't exist.
pub async fn update_profile(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(name): AxumPath<String>,
    Json(mut profile): Json<ProcessingProfile>,
) -> Result<Json<ProcessingProfile>, AppError> {
    require_admin(&user)?;
    validate_profile_name(&name)?;

    let (basename, path) = resolve_profile_basename_and_path(&state.registry, &name, PROFILE_EXT);
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Err(AppError::NotFound {
            message: format!("Profile '{name}' not found"),
        });
    }

    // Resolved basename wins — overwrite the body name so the on-disk
    // file and its internal `name` field stay in sync.
    profile.name = basename;
    validate_profile_references(&state, &profile).await?;

    write_profile_yaml(&path, &profile).await?;
    Ok(Json(profile))
}

/// DELETE /api/admin/pipeline/profiles/:name — deactivate a profile.
///
/// `:name` may be a registry document_type or a literal profile filename
/// basename. The resolved file is renamed
/// `{basename}.yaml` → `{basename}.yaml.inactive`. Never hard-deletes —
/// admins can restore by renaming back. `404` if the source doesn't exist;
/// if the destination already exists, it is overwritten (re-deactivate).
pub async fn deactivate_profile(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(name): AxumPath<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&user)?;
    validate_profile_name(&name)?;

    let (basename, src) = resolve_profile_basename_and_path(&state.registry, &name, PROFILE_EXT);
    if !tokio::fs::try_exists(&src).await.unwrap_or(false) {
        return Err(AppError::NotFound {
            message: format!("Profile '{name}' not found"),
        });
    }
    let dst = profile_path(state.registry.profile_dir(), &basename, INACTIVE_EXT);

    tokio::fs::rename(&src, &dst)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to rename profile '{name}': {e}"),
        })?;

    Ok(Json(serde_json::json!({
        "deactivated": basename,
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
    tokio::fs::write(path, yaml)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to write profile '{}': {e}", path.display()),
        })
}
