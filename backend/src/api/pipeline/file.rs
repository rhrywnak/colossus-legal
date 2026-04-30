//! GET /documents/:id/file — Serve a pipeline document's source file.
//!
//! Content-Type is set from the document's detected `mime_type`
//! (PDF, DOCX, or plain text). Pre-multi-format documents have NULL
//! `mime_type` and fall back to `application/pdf` so historical
//! behaviour is preserved.
//!
//! Open to all authenticated users (not admin-only) so that
//! non-admin users can view documents that have been published.

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::Response,
};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::repositories::pipeline_repository;
use crate::state::AppState;

/// Serve the PDF file for a pipeline document.
///
/// Looks up the document by ID in the pipeline database, reads
/// `file_path`, and streams the file from `DOCUMENT_STORAGE_PATH`.
///
/// ## Rust Learning: Streaming large files
///
/// Instead of reading the entire PDF into memory, we open a
/// `tokio::fs::File` and wrap it in `ReaderStream`. Axum streams
/// the bytes to the client as they're read from disk — constant
/// memory usage regardless of file size.
pub async fn file_handler(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(document_id): Path<String>,
) -> Result<Response<Body>, AppError> {
    // 1. Look up document in pipeline DB
    let doc = pipeline_repository::get_document(&state.pipeline_pool, &document_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{document_id}' not found"),
        })?;

    // 2. Validate file_path. Also capture the detected MIME type so we can
    //    serve .docx and .txt with the correct Content-Type instead of
    //    forcing every download through `application/pdf`.
    //
    //    `mime_type` is NULL for documents uploaded before multi-format
    //    support (migration 20260430103319). Falling back to
    //    `application/pdf` preserves the prior behavior for those rows.
    let file_path = doc.file_path;
    let content_type = doc
        .mime_type
        .unwrap_or_else(|| "application/pdf".to_string());
    if file_path.is_empty() {
        return Err(AppError::NotFound {
            message: "Document has no file".to_string(),
        });
    }
    if file_path.contains("..") || file_path.contains('/') || file_path.contains('\\') {
        return Err(AppError::BadRequest {
            message: "Invalid file path".to_string(),
            details: serde_json::json!({}),
        });
    }

    // 3. Build full path and open file
    let full_path = format!(
        "{}/{}",
        state.config.document_storage_path.trim_end_matches('/'),
        file_path
    );

    let file = File::open(&full_path)
        .await
        .map_err(|e| {
            tracing::error!(
                document_id = %document_id,
                path = %full_path,
                error = %e,
                "Failed to open document PDF on disk"
            );
            AppError::NotFound {
                message: "File not found on disk".to_string(),
            }
        })?;

    // 4. Stream response with the document's detected Content-Type.
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type.as_str())
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{file_path}\""),
        )
        .body(body)
        .map_err(|e| {
            tracing::error!(
                document_id = %document_id,
                path = %full_path,
                error = %e,
                "Failed to build response"
            );
            AppError::Internal {
                message: "Failed to build response".to_string(),
            }
        })
}
