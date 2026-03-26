//! GET /api/admin/documents/:id/extracts — Serve extract JSON files.
//!
//! Returns the Claude extraction JSON file for a document, used by
//! the Document Workspace for completeness verification (highlighting
//! extracted text on the source PDF).
//!
//! The extract files live on disk at:
//!   {DOCUMENT_STORAGE_PATH}/extracts/{extract_path}
//!
//! where `extract_path` comes from the Document node's property in Neo4j
//! (NOT constructed from the document ID — filenames vary).
//!
//! ## Rust Learning: Reading files asynchronously
//!
//! We use `tokio::fs::read_to_string` instead of `std::fs::read_to_string`
//! because we're inside an async handler. The std version would block the
//! entire Tokio runtime thread while reading the file. The tokio version
//! yields the thread back to the runtime while the OS does the I/O, so
//! other requests can be served in the meantime.

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use neo4rs::query;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::state::AppState;

/// GET /admin/documents/:id/extracts
///
/// Looks up the document's `extract_path` in Neo4j, then reads and
/// returns the raw JSON file from disk. The backend does NOT parse the
/// JSON — extract schemas vary by document type (Complaint vs Discovery
/// vs Brief), so the frontend handles schema differences.
pub async fn get_document_extracts(
    user: AuthUser,
    State(state): State<AppState>,
    Path(document_id): Path<String>,
) -> Result<Response, AppError> {
    require_admin(&user)?;

    tracing::info!(
        user = %user.username,
        doc_id = %document_id,
        "GET /admin/documents/{}/extracts", document_id
    );

    // Step 1: Query Neo4j for the extract_path property
    let cypher = "MATCH (d:Document {id: $doc_id}) RETURN d.extract_path AS extract_path";
    let mut result = state
        .graph
        .execute(query(cypher).param("doc_id", &*document_id))
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Neo4j query failed: {e}"),
        })?;

    let row = result
        .next()
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Neo4j row fetch failed: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document not found: {document_id}"),
        })?;

    let extract_filename: String = row.get("extract_path").map_err(|_| AppError::NotFound {
        message: format!("No extract file for document {document_id}"),
    })?;

    // Step 2: Validate filename (security: prevent path traversal)
    if extract_filename.contains("..") || extract_filename.contains('/') || extract_filename.contains('\\') {
        return Err(AppError::BadRequest {
            message: "Invalid extract path".to_string(),
            details: serde_json::json!({}),
        });
    }

    // Step 3: Read the file from disk
    let full_path = format!(
        "{}/extracts/{}",
        state.config.document_storage_path, extract_filename
    );

    match tokio::fs::read_to_string(&full_path).await {
        Ok(contents) => Ok((
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            contents,
        )
            .into_response()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::error!("Extract file not found on disk: {}", full_path);
            Err(AppError::NotFound {
                message: format!("Extract file missing from disk: {extract_filename}"),
            })
        }
        Err(e) => {
            tracing::error!("Failed to read extract file {}: {}", full_path, e);
            Err(AppError::Internal {
                message: "Failed to read extract file".to_string(),
            })
        }
    }
}
