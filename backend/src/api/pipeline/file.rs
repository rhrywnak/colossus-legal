//! GET /documents/:id/file — Serve a pipeline document's source file.
//!
//! Content-Type is set from the document's detected `mime_type`
//! (PDF, DOCX, or plain text). Pre-multi-format documents have NULL
//! `mime_type` and fall back to `application/pdf` so historical
//! behaviour is preserved.
//!
//! Open to all authenticated users (not admin-only) so that
//! non-admin users can view documents that have been published.
//!
//! ## Single source for both file routes
//! The actual serving logic lives in [`serve_document_file`], which BOTH the
//! admin route (`file_handler`, here) and the public route
//! (`crate::api::documents::get_document_file`) call. The on-disk location is
//! read from Postgres `documents.file_path` (the source of truth — the Neo4j
//! Document node never carries a file path), so both routes resolve and stream
//! files through one identical implementation with no risk of drift.

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

/// Admin route handler — `GET /api/admin/pipeline/documents/:id/file`.
///
/// A thin wrapper: it enforces the `AuthUser` extractor at the route boundary,
/// then delegates to the shared [`serve_document_file`]. The public route does
/// the same, so there is exactly one serving implementation.
pub async fn file_handler(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(document_id): Path<String>,
) -> Result<Response<Body>, AppError> {
    serve_document_file(&state, &document_id).await
}

/// Serve a document's source file inline, resolved from Postgres + disk.
///
/// The single entry point shared by both file routes. Auth is intentionally NOT
/// handled here — each route keeps its own authentication at the handler
/// boundary — so this takes only what it needs to locate and stream the file.
/// The file-open-and-stream mechanics live in [`stream_inline_file`] so this
/// stays a short lookup-and-validate function.
pub async fn serve_document_file(
    state: &AppState,
    document_id: &str,
) -> Result<Response<Body>, AppError> {
    // 1. Look up the document row in the pipeline DB (the source of truth for
    //    the on-disk location — the Neo4j node carries no file path).
    let doc = pipeline_repository::get_document(&state.pipeline_pool, document_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{document_id}' not found"),
        })?;

    // 2. Read file_path + the detected MIME type so .docx/.txt serve with the
    //    correct Content-Type instead of forcing everything through PDF.
    //    `mime_type` is NULL for documents uploaded before multi-format support
    //    (migration 20260430103319); the fallback preserves prior behavior.
    let file_path = doc.file_path;
    let content_type = doc
        .mime_type
        .unwrap_or_else(|| "application/pdf".to_string());
    if file_path.is_empty() {
        return Err(AppError::NotFound {
            message: "Document has no file".to_string(),
        });
    }
    if !is_safe_stored_filename(&file_path) {
        return Err(AppError::BadRequest {
            message: "Invalid file path".to_string(),
            details: serde_json::json!({}),
        });
    }

    // 3. Resolve the on-disk path and stream the file inline.
    let full_path = resolve_storage_path(&state.config.document_storage_path, &file_path);
    stream_inline_file(&full_path, &content_type, &file_path, document_id).await
}

/// Open a file on disk and return it as an inline, streamed HTTP response.
///
/// `download_name` is the filename offered to the browser (the bare stored
/// filename); `document_id` is used only for error-log context. Failures are
/// logged with `tracing::error!` (path + cause) and returned as `AppError`.
///
/// ## Rust Learning: Streaming large files
/// Instead of reading the whole file into memory, we open a `tokio::fs::File`
/// and wrap it in `ReaderStream`. Axum streams the bytes to the client as they
/// are read from disk — constant memory usage regardless of file size.
async fn stream_inline_file(
    full_path: &str,
    content_type: &str,
    download_name: &str,
    document_id: &str,
) -> Result<Response<Body>, AppError> {
    let file = File::open(full_path).await.map_err(|e| {
        tracing::error!(
            document_id = %document_id,
            path = %full_path,
            error = %e,
            "Failed to open document file on disk"
        );
        AppError::NotFound {
            message: "File not found on disk".to_string(),
        }
    })?;

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{download_name}\""),
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

/// Whether a stored filename is safe to join onto the storage root.
///
/// `documents.file_path` holds a bare filename (e.g. `doc-x.pdf`), never a
/// nested path. Anything containing a path separator or a parent-dir reference
/// is rejected so a malformed/hostile row cannot escape the storage directory
/// (path-traversal guard). Pure (no I/O) so the rule is unit-testable.
fn is_safe_stored_filename(file_path: &str) -> bool {
    !(file_path.contains("..") || file_path.contains('/') || file_path.contains('\\'))
}

/// Join the configured storage root and a document's stored filename into the
/// absolute on-disk path.
///
/// Trims a trailing slash on the root so a configured `"/data/documents/"` and
/// `"/data/documents"` both yield `"/data/documents/{file}"` (no double slash).
/// Pure (no I/O) so the path-construction contract is unit-testable without a
/// live filesystem.
fn resolve_storage_path(storage_root: &str, file_path: &str) -> String {
    format!("{}/{}", storage_root.trim_end_matches('/'), file_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_safe_stored_filename_accepts_a_bare_filename() {
        assert!(is_safe_stored_filename("doc-awad-complaint-11-1-13.pdf"));
        assert!(is_safe_stored_filename("a.docx"));
    }

    #[test]
    fn is_safe_stored_filename_rejects_separators_and_parent_refs() {
        // Calls the ACTUAL guard, not a restatement of its predicate, so the
        // test fails if the guard is ever weakened/removed.
        for bad in ["../etc/passwd", "a/b", "a\\b", "..", "dir/doc.pdf"] {
            assert!(!is_safe_stored_filename(bad), "guard must reject {bad}");
        }
    }

    #[test]
    fn resolve_storage_path_joins_root_and_filename() {
        assert_eq!(
            resolve_storage_path("/data/documents", "doc-x.pdf"),
            "/data/documents/doc-x.pdf"
        );
    }

    #[test]
    fn resolve_storage_path_trims_trailing_slash_to_avoid_double_slash() {
        // A configured root WITH a trailing slash must still yield exactly one
        // separator — this is the bug the public route had before reuse (it did
        // not trim), so lock it for both routes that now share this helper.
        assert_eq!(
            resolve_storage_path("/data/documents/", "doc-x.pdf"),
            "/data/documents/doc-x.pdf"
        );
    }
}
