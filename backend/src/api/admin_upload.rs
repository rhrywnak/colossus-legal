//! POST /api/admin/upload — Upload a PDF file to the documents directory.
//!
//! ## Rust Learning: Multipart Form Data
//!
//! Axum's `Multipart` extractor handles multipart/form-data requests.
//! Unlike `Json<T>`, which deserializes the entire body at once, `Multipart`
//! streams fields one at a time. You call `next_field().await` in a loop,
//! check each field's name/content_type, and process accordingly.
//! This is memory-efficient for large files because you can write chunks
//! to disk as they arrive rather than buffering the entire file in RAM.

use axum::{
    extract::{Multipart, State},
    Json,
};
use serde::Serialize;
use std::path::PathBuf;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::audit_repository::log_admin_action;
use crate::state::AppState;

/// Maximum upload size: 50 MB.
const MAX_FILE_SIZE: usize = 50 * 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct UploadResponse {
    pub filename: String,
    pub size_bytes: usize,
    pub path: String,
}

/// POST /api/admin/upload — Accept a single PDF file upload.
///
/// Validates:
/// - Only `.pdf` files (by extension and content type)
/// - Max 50 MB
/// - No overwrite — returns 409 if the file already exists
pub async fn upload_file(
    user: AuthUser,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, "POST /api/admin/upload");

    // Extract the first field named "file"
    let field = loop {
        match multipart.next_field().await {
            Ok(Some(f)) => {
                if f.name() == Some("file") {
                    break f;
                }
                // Skip non-"file" fields
            }
            Ok(None) => {
                return Err(AppError::BadRequest {
                    message: "No 'file' field in multipart upload".to_string(),
                    details: serde_json::json!({}),
                });
            }
            Err(e) => {
                return Err(AppError::BadRequest {
                    message: format!("Failed to read multipart field: {e}"),
                    details: serde_json::json!({}),
                });
            }
        }
    };

    // Validate content type
    let content_type = field.content_type().unwrap_or("").to_string();
    if content_type != "application/pdf" {
        return Err(AppError::BadRequest {
            message: format!("Only PDF files accepted, got content type: {content_type}"),
            details: serde_json::json!({ "content_type": content_type }),
        });
    }

    // Get and validate filename
    let filename = field.file_name().map(|s| s.to_string()).unwrap_or_default();
    if filename.is_empty() {
        return Err(AppError::BadRequest {
            message: "File has no filename".to_string(),
            details: serde_json::json!({}),
        });
    }
    if !filename.to_lowercase().ends_with(".pdf") {
        return Err(AppError::BadRequest {
            message: "Only .pdf files are accepted".to_string(),
            details: serde_json::json!({ "filename": filename }),
        });
    }

    // Prevent path traversal in filename
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err(AppError::BadRequest {
            message: "Invalid filename — must be a plain filename".to_string(),
            details: serde_json::json!({ "filename": filename }),
        });
    }

    // Check if file already exists — 409 Conflict
    let dest_path: PathBuf = [&state.config.document_storage_path, &filename]
        .iter()
        .collect();
    if dest_path.exists() {
        return Err(AppError::Conflict {
            message: format!("File '{filename}' already exists"),
            details: serde_json::json!({ "filename": filename }),
        });
    }

    // Read file bytes (with size limit)
    let data = field.bytes().await.map_err(|e| AppError::BadRequest {
        message: format!("Failed to read file data: {e}"),
        details: serde_json::json!({}),
    })?;

    if data.len() > MAX_FILE_SIZE {
        return Err(AppError::BadRequest {
            message: format!(
                "File too large: {} bytes (max {} bytes)",
                data.len(),
                MAX_FILE_SIZE
            ),
            details: serde_json::json!({
                "size_bytes": data.len(),
                "max_bytes": MAX_FILE_SIZE,
            }),
        });
    }

    // Write to disk
    tokio::fs::write(&dest_path, &data)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to write file to disk: {e}"),
        })?;

    let size_bytes = data.len();
    let storage_path = format!(
        "{}/{}",
        state.config.document_storage_path.trim_end_matches('/'),
        filename
    );

    tracing::info!(user = %user.username, filename = %filename, size = size_bytes, "File uploaded");

    log_admin_action(
        &state.audit_repo,
        &user.username,
        "document.upload",
        Some("document"),
        Some(&filename),
        Some(serde_json::json!({ "size_bytes": size_bytes, "filename": &filename })),
    )
    .await;

    Ok(Json(UploadResponse {
        filename,
        size_bytes,
        path: storage_path,
    }))
}
