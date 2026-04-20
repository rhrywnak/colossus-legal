//! Admin CRUD endpoints for system-prompt Markdown files.
//!
//! System prompts are optional `.md` files referenced by profile YAMLs via
//! `system_prompt_file`. They live under `system_prompt_dir` so they can
//! be authored and revised independently of chunk-extract templates.
//!
//! Design: DOC_PROCESSING_CONFIG_DESIGN_v2.md Section 3.4.3.

use axum::{
    extract::{Path as AxumPath, State},
    Json,
};
use serde::Serialize;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::state::AppState;

use super::shared::{
    profiles_referencing, require_extension, validate_filename, CreateFileInput,
    FileContentResponse, UpdateFileInput,
};

/// Required extension for system-prompt files.
const SYSTEM_PROMPT_EXT: &str = ".md";

/// Max bytes of system-prompt content returned as a list-view preview.
const PREVIEW_CHAR_LIMIT: usize = 500;

#[derive(Debug, Serialize)]
pub struct SystemPromptsResponse {
    pub system_prompts: Vec<SystemPromptInfo>,
}

#[derive(Debug, Serialize)]
pub struct SystemPromptInfo {
    pub filename: String,
    pub preview: String,
    pub size_bytes: u64,
}

/// GET /api/admin/pipeline/system-prompts — list system-prompt files.
pub async fn list_system_prompts(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<SystemPromptsResponse>, AppError> {
    require_admin(&user)?;

    let dir = &state.config.system_prompt_dir;
    let mut out = Vec::new();

    let mut entries = match tokio::fs::read_dir(dir).await {
        Ok(r) => r,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Json(SystemPromptsResponse {
                system_prompts: out,
            }));
        }
        Err(e) => {
            return Err(AppError::Internal {
                message: format!("Failed to read system-prompt directory: {e}"),
            });
        }
    };

    while let Some(entry) = entries.next_entry().await.map_err(|e| AppError::Internal {
        message: format!("Failed to read directory entry: {e}"),
    })? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let filename = entry.file_name().to_string_lossy().to_string();
        let metadata = entry.metadata().await.map_err(|e| AppError::Internal {
            message: format!("Failed to read file metadata: {e}"),
        })?;
        let content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
        let preview: String = content.chars().take(PREVIEW_CHAR_LIMIT).collect();
        out.push(SystemPromptInfo {
            filename,
            preview,
            size_bytes: metadata.len(),
        });
    }

    out.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(Json(SystemPromptsResponse {
        system_prompts: out,
    }))
}

/// GET /api/admin/pipeline/system-prompts/:filename — read one file.
pub async fn get_system_prompt(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>,
) -> Result<Json<FileContentResponse>, AppError> {
    require_admin(&user)?;
    validate_filename(&filename)?;

    let path = std::path::Path::new(&state.config.system_prompt_dir).join(&filename);
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Err(AppError::NotFound {
            message: format!("System prompt '{filename}' not found"),
        });
    }

    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to read system prompt '{filename}': {e}"),
        })?;
    let size_bytes = content.len() as u64;

    Ok(Json(FileContentResponse {
        filename,
        content,
        size_bytes,
    }))
}

/// POST /api/admin/pipeline/system-prompts — create a new file.
pub async fn create_system_prompt(
    user: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<CreateFileInput>,
) -> Result<Json<FileContentResponse>, AppError> {
    require_admin(&user)?;
    validate_filename(&input.filename)?;
    require_extension(&input.filename, SYSTEM_PROMPT_EXT)?;

    let path = std::path::Path::new(&state.config.system_prompt_dir).join(&input.filename);
    if tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Err(AppError::Conflict {
            message: format!("System prompt '{}' already exists", input.filename),
            details: serde_json::json!({"filename": input.filename}),
        });
    }

    tokio::fs::write(&path, &input.content)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to write system prompt '{}': {e}", input.filename),
        })?;

    let size_bytes = input.content.len() as u64;
    Ok(Json(FileContentResponse {
        filename: input.filename,
        content: input.content,
        size_bytes,
    }))
}

/// PUT /api/admin/pipeline/system-prompts/:filename — overwrite a file.
pub async fn update_system_prompt(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>,
    Json(input): Json<UpdateFileInput>,
) -> Result<Json<FileContentResponse>, AppError> {
    require_admin(&user)?;
    validate_filename(&filename)?;

    let path = std::path::Path::new(&state.config.system_prompt_dir).join(&filename);
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Err(AppError::NotFound {
            message: format!("System prompt '{filename}' not found"),
        });
    }

    tokio::fs::write(&path, &input.content)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to write system prompt '{filename}': {e}"),
        })?;

    let size_bytes = input.content.len() as u64;
    Ok(Json(FileContentResponse {
        filename,
        content: input.content,
        size_bytes,
    }))
}

/// DELETE /api/admin/pipeline/system-prompts/:filename — delete a file.
///
/// Refuses the delete (`409 Conflict`) if any profile YAML references
/// this filename as `system_prompt_file`.
pub async fn delete_system_prompt(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&user)?;
    validate_filename(&filename)?;

    let path = std::path::Path::new(&state.config.system_prompt_dir).join(&filename);
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Err(AppError::NotFound {
            message: format!("System prompt '{filename}' not found"),
        });
    }

    let referencing = profiles_referencing(&state.config.processing_profile_dir, &filename)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to scan profile directory: {e}"),
        })?;

    if !referencing.is_empty() {
        return Err(AppError::Conflict {
            message: format!(
                "System prompt '{filename}' is referenced by {} profile(s)",
                referencing.len()
            ),
            details: serde_json::json!({"referenced_by": referencing}),
        });
    }

    tokio::fs::remove_file(&path)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to delete system prompt '{filename}': {e}"),
        })?;

    Ok(Json(serde_json::json!({"deleted": filename})))
}
