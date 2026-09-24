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

use crate::services::ai_jobs_usage::{file_refusal, usage};

use super::shared::{validate_filename, CreateFileInput, FileContentResponse, UpdateFileInput};

/// Max bytes of template content returned as a list-view preview.
const PREVIEW_CHAR_LIMIT: usize = 500;

#[derive(Debug, Serialize)]
pub struct TemplatesResponse {
    pub templates: Vec<TemplateInfo>,
    /// "New versions of these files arrive with a release." — the tab is
    /// read-only (ruling Q3), and says so.
    pub read_only_note: String,
    /// The "Used for" column's heading.
    pub used_for_label: String,
}

#[derive(Debug, Serialize)]
pub struct TemplateInfo {
    pub filename: String,
    pub preview: String,
    pub size_bytes: u64,
    /// The AI jobs using this file, in panel words; `None` when none does.
    pub used_for: Option<String>,
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
    let used = usage(&state).await.map_err(|e| AppError::Internal {
        message: format!("Failed to read which jobs use each template: {e}"),
    })?;

    let template_dir = state.registry.template_dir();
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
            used_for: used.file(&filename),
            filename,
            preview,
            size_bytes: metadata.len(),
        });
    }

    templates.sort_by(|a, b| a.filename.cmp(&b.filename));
    let settings = state.settings.current();
    let w = &settings.admin_wording.ai_jobs;
    Ok(Json(TemplatesResponse {
        templates,
        read_only_note: w.files_note.clone(),
        used_for_label: w.used_for_label.clone(),
    }))
}

/// GET /api/admin/pipeline/templates/:filename — read a single template.
pub async fn get_template(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>,
) -> Result<Json<FileContentResponse>, AppError> {
    require_admin(&user)?;
    validate_filename(&filename)?;

    let path = state.registry.template_path(&filename);
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

/// POST /api/admin/pipeline/templates — REFUSED (ruling Q3, 2026-09-24).
///
/// ## Domain note: why every write here is refused
///
/// The instructions folder is read-only to the backend on DEV and PROD (root
/// owns it; the backend runs as `appuser`), so a write here could only ever fail
/// half-way. New versions of these files arrive with a release — through the
/// repo and `scripts/push-templates.sh` — which is also the only way a fresh
/// install gets them. A file an AI job uses is named in the refusal, with the
/// job, so nobody goes looking for why.
pub async fn create_template(
    user: AuthUser,
    State(state): State<AppState>,
    Json(input): Json<CreateFileInput>,
) -> Result<Json<FileContentResponse>, AppError> {
    require_admin(&user)?;
    Err(refusal(&state, &input.filename).await)
}

/// PUT /api/admin/pipeline/templates/:filename — REFUSED; see [`create_template`].
pub async fn update_template(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>,
    Json(_input): Json<UpdateFileInput>,
) -> Result<Json<FileContentResponse>, AppError> {
    require_admin(&user)?;
    Err(refusal(&state, &filename).await)
}

/// DELETE /api/admin/pipeline/templates/:filename — REFUSED; see [`create_template`].
pub async fn delete_template(
    user: AuthUser,
    State(state): State<AppState>,
    AxumPath(filename): AxumPath<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&user)?;
    Err(refusal(&state, &filename).await)
}

/// The 409 for any write: it names the jobs when one uses the file, and says the
/// folder is read-only otherwise. Logged, so a refused click is visible later.
async fn refusal(state: &AppState, filename: &str) -> AppError {
    match usage(state).await {
        Ok(used) => {
            let settings = state.settings.current();
            let message = file_refusal(&settings.admin_wording.ai_jobs, &used, filename);
            tracing::warn!(file = filename, %message, "templates: write refused (read-only folder)");
            AppError::Conflict {
                message,
                details: serde_json::json!({ "reason": "templates_read_only" }),
            }
        }
        Err(e) => {
            tracing::error!(file = filename, error = %e, "templates: could not read which jobs use the file");
            AppError::Internal {
                message: format!("Failed to read which jobs use '{filename}': {e}"),
            }
        }
    }
}
