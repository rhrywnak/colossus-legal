//! Admin CRUD endpoints for extraction prompt template files (.md).
//!
//! Templates are Markdown files under `extraction_template_dir` that the
//! extraction pipeline loads to build per-chunk LLM prompts.
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

/// Required extension for template files.
const TEMPLATE_EXT: &str = ".md";

/// Max bytes of template content returned as a list-view preview.
const PREVIEW_CHAR_LIMIT: usize = 500;

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
///
/// Scans `extraction_template_dir` for `.md` files. A short `preview`
/// (first [`PREVIEW_CHAR_LIMIT`] chars) is returned so the admin list
/// view can show the header without a per-row extra fetch.
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
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let filename = entry.file_name().to_string_lossy().to_string();
        let metadata = entry.metadata().await.map_err(|e| AppError::Internal {
            message: format!("Failed to read file metadata: {e}"),
        })?;
        let content = tokio::fs::read_to_string(&path).await.unwrap_or_default();
        let preview: String = content.chars().take(PREVIEW_CHAR_LIMIT).collect();
        templates.push(TemplateInfo {
            filename,
            preview,
            size_bytes: metadata.len(),
        });
    }

    templates.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(Json(TemplatesResponse { templates }))
}

/// GET /api/admin/pipeline/templates/:filename — read a single template.
pub async fn get_template(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>,
) -> Result<Json<FileContentResponse>, AppError> {
    require_admin(&user)?;
    validate_filename(&filename)?;

    let path = std::path::Path::new(&state.config.extraction_template_dir).join(&filename);
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Err(AppError::NotFound {
            message: format!("Template '{filename}' not found"),
        });
    }

    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to read template '{filename}': {e}"),
        })?;
    let size_bytes = content.len() as u64;

    Ok(Json(FileContentResponse {
        filename,
        content,
        size_bytes,
    }))
}

/// POST /api/admin/pipeline/templates — create a new template file.
///
/// Filename must pass [`validate_filename`] and end in [`TEMPLATE_EXT`].
/// Returns `409 Conflict` if the file already exists.
pub async fn create_template(
    user: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<CreateFileInput>,
) -> Result<Json<FileContentResponse>, AppError> {
    require_admin(&user)?;
    validate_filename(&input.filename)?;
    require_extension(&input.filename, TEMPLATE_EXT)?;

    let path = std::path::Path::new(&state.config.extraction_template_dir).join(&input.filename);
    if tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Err(AppError::Conflict {
            message: format!("Template '{}' already exists", input.filename),
            details: serde_json::json!({"filename": input.filename}),
        });
    }

    tokio::fs::write(&path, &input.content)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to write template '{}': {e}", input.filename),
        })?;

    let size_bytes = input.content.len() as u64;
    Ok(Json(FileContentResponse {
        filename: input.filename,
        content: input.content,
        size_bytes,
    }))
}

/// PUT /api/admin/pipeline/templates/:filename — overwrite an existing template.
///
/// `404 Not Found` if the file doesn't exist.
pub async fn update_template(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>,
    Json(input): Json<UpdateFileInput>,
) -> Result<Json<FileContentResponse>, AppError> {
    require_admin(&user)?;
    validate_filename(&filename)?;

    let path = std::path::Path::new(&state.config.extraction_template_dir).join(&filename);
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Err(AppError::NotFound {
            message: format!("Template '{filename}' not found"),
        });
    }

    tokio::fs::write(&path, &input.content)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to write template '{filename}': {e}"),
        })?;

    let size_bytes = input.content.len() as u64;
    Ok(Json(FileContentResponse {
        filename,
        content: input.content,
        size_bytes,
    }))
}

/// DELETE /api/admin/pipeline/templates/:filename — delete a template.
///
/// Refuses the delete (`409 Conflict`) if any profile YAML references
/// this filename (as `template_file` or `system_prompt_file`). The check
/// is a substring scan of profile content — see
/// [`shared::profiles_referencing`](super::shared::profiles_referencing).
pub async fn delete_template(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&user)?;
    validate_filename(&filename)?;

    let path = std::path::Path::new(&state.config.extraction_template_dir).join(&filename);
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Err(AppError::NotFound {
            message: format!("Template '{filename}' not found"),
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
                "Template '{filename}' is referenced by {} profile(s)",
                referencing.len()
            ),
            details: serde_json::json!({"referenced_by": referencing}),
        });
    }

    tokio::fs::remove_file(&path)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to delete template '{filename}': {e}"),
        })?;

    Ok(Json(serde_json::json!({"deleted": filename})))
}
