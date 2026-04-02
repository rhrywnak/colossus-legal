//! GET /documents/:id/file — Serve a pipeline document's PDF.
//!
//! Open to all authenticated users (not admin-only) so that
//! non-admin users can view PDFs for published documents.

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
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{document_id}' not found"),
        })?;

    // 2. Validate file_path
    let file_path = doc.file_path;
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

    let file = File::open(&full_path).await.map_err(|_| AppError::NotFound {
        message: "File not found on disk".to_string(),
    })?;

    // 4. Stream response with PDF headers
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/pdf")
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{file_path}\""),
        )
        .body(body)
        .map_err(|_| AppError::Internal {
            message: "Failed to build response".to_string(),
        })
}
