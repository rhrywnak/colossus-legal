//! Admin endpoints for document management.
//!
//! These endpoints live at `/api/admin/documents` and require admin access.
//! They extend the base `/documents` CRUD with:
//! - SHA-256 content hashing for deduplication
//! - PDF existence verification before registration
//! - Evidence counts in the list response
//!
//! ## Rust Learning: SHA-256 Hashing with the `Digest` Trait
//!
//! The `sha2` crate uses a trait-based API:
//!
//! ```rust,ignore
//! use sha2::{Sha256, Digest};
//! let mut hasher = Sha256::new();       // Create the hasher
//! hasher.update(file_bytes);            // Feed it data (can call multiple times)
//! let hash = hasher.finalize();         // Get the result
//! let hex = format!("{:x}", hash);      // Convert to hex string
//! ```
//!
//! The `Digest` trait is generic — you could swap `Sha256` for `Sha512`
//! and the rest of the code stays the same. This is Rust's trait
//! polymorphism at work, similar to how our `VectorRetriever` trait
//! lets us swap Qdrant for a mock in tests.

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::document_repository::DocumentRepository;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// Response after successful document registration with content hash.
#[derive(Debug, Serialize)]
pub struct RegisterDocumentResponse {
    pub id: String,
    pub title: String,
    pub content_hash: String,
    pub pdf_url: String,
}

/// A document summary with evidence count for admin listing.
#[derive(Debug, Serialize)]
pub struct DocumentSummary {
    pub id: String,
    pub title: String,
    pub doc_type: Option<String>,
    pub created_at: Option<String>,
    pub file_path: Option<String>,
    pub content_hash: Option<String>,
    pub evidence_count: i64,
    pub has_pdf: bool,
}

/// Response for the admin document list.
#[derive(Debug, Serialize)]
pub struct ListDocumentsResponse {
    pub documents: Vec<DocumentSummary>,
    pub total: usize,
}

// ---------------------------------------------------------------------------
// Request types
// ---------------------------------------------------------------------------

/// Request body for registering a document with content hash verification.
///
/// Unlike the base `DocumentCreateRequest`, this requires `file_path` (not
/// optional) because admin registration always verifies the PDF on disk.
#[derive(Debug, serde::Deserialize)]
pub struct RegisterDocumentRequest {
    pub id: String,
    pub title: String,
    pub doc_type: String,
    pub created_at: Option<String>,
    pub description: Option<String>,
    /// PDF filename on disk (relative to DOCUMENT_STORAGE_PATH).
    pub file_path: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /api/admin/documents — Register a document with PDF verification.
///
/// 1. Verifies the PDF file exists on disk
/// 2. Computes SHA-256 content hash
/// 3. Checks for duplicate by ID
/// 4. Checks for duplicate by content hash
/// 5. Creates the Document node via the repository
/// 6. Sets content_hash on the node
pub async fn register_document(
    user: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<RegisterDocumentRequest>,
) -> Result<(StatusCode, Json<RegisterDocumentResponse>), AppError> {
    require_admin(&user)?;
    tracing::info!(
        user = %user.username,
        doc_id = %req.id,
        "POST /api/admin/documents"
    );

    // 1. Validate file_path — prevent path traversal
    if req.file_path.contains("..") || req.file_path.contains('/') || req.file_path.contains('\\')
    {
        return Err(AppError::BadRequest {
            message: "file_path must be a plain filename, no path separators or ..".to_string(),
            details: json!({ "field": "file_path" }),
        });
    }

    // 2. Build the full path and check the PDF exists on disk
    let pdf_path: PathBuf = [&state.config.document_storage_path, &req.file_path]
        .iter()
        .collect();

    if !pdf_path.exists() {
        return Err(AppError::BadRequest {
            message: format!(
                "PDF not found at {}. Upload the file before registering.",
                pdf_path.display()
            ),
            details: json!({ "field": "file_path", "path": pdf_path.display().to_string() }),
        });
    }

    // 3. Compute SHA-256 content hash
    //
    // We read the entire file into memory. This is fine for legal PDFs
    // (typically 100KB–5MB). For very large files, we'd use a streaming
    // approach with tokio::io::AsyncReadExt, but that's not needed here.
    let file_bytes = tokio::fs::read(&pdf_path).await.map_err(|e| {
        AppError::Internal {
            message: format!("Failed to read PDF: {e}"),
        }
    })?;

    let mut hasher = Sha256::new();
    hasher.update(&file_bytes);
    let content_hash = format!("{:x}", hasher.finalize());

    let repo = DocumentRepository::new(state.graph.clone());

    // 4. Check for duplicate by ID
    match repo.get_document_by_id(&req.id).await {
        Ok(_) => {
            return Err(AppError::Conflict {
                message: format!("Document with id '{}' already exists", req.id),
                details: json!({ "existing_id": req.id }),
            });
        }
        Err(crate::repositories::document_repository::DocumentRepositoryError::NotFound) => {
            // Good — no duplicate
        }
        Err(e) => {
            return Err(AppError::Internal {
                message: format!("Failed to check for duplicate: {e:?}"),
            });
        }
    }

    // 5. Check for duplicate by content hash
    match repo.find_by_content_hash(&content_hash).await {
        Ok(Some(existing)) => {
            return Err(AppError::Conflict {
                message: format!(
                    "A document with identical content already exists: '{}' ({})",
                    existing.title, existing.id
                ),
                details: json!({
                    "existing_id": existing.id,
                    "existing_title": existing.title,
                    "content_hash": content_hash,
                }),
            });
        }
        Ok(None) => {
            // Good — no duplicate content
        }
        Err(e) => {
            return Err(AppError::Internal {
                message: format!("Failed to check content hash: {e:?}"),
            });
        }
    }

    // 6. Create the Document node via the existing repository
    let create_req = crate::dto::document::DocumentCreateRequest {
        title: req.title.clone(),
        doc_type: req.doc_type,
        created_at: req.created_at,
        description: req.description,
        file_path: Some(req.file_path),
        uploaded_at: None,
        related_claim_id: None,
        source_url: None,
    };

    let document = repo.create_document(create_req).await.map_err(|e| {
        AppError::Internal {
            message: format!("Failed to create document: {e:?}"),
        }
    })?;

    // 7. Set the content_hash (not part of base create)
    repo.set_content_hash(&document.id, &content_hash)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Document created but failed to set content_hash: {e:?}"),
        })?;

    tracing::info!(
        user = %user.username,
        doc_id = %document.id,
        hash = %content_hash,
        "Document registered"
    );

    Ok((
        StatusCode::CREATED,
        Json(RegisterDocumentResponse {
            pdf_url: format!("/documents/{}/file", document.id),
            id: document.id,
            title: document.title,
            content_hash,
        }),
    ))
}

/// GET /api/admin/documents — List all documents with evidence counts.
pub async fn list_documents(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ListDocumentsResponse>, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, "GET /api/admin/documents");

    let repo = DocumentRepository::new(state.graph.clone());
    let docs_with_counts = repo
        .list_documents_with_evidence_counts()
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to list documents: {e:?}"),
        })?;

    let storage_path = &state.config.document_storage_path;

    let documents: Vec<DocumentSummary> = docs_with_counts
        .into_iter()
        .map(|(doc, evidence_count)| {
            // Check if the PDF file actually exists on disk
            let has_pdf = doc
                .file_path
                .as_ref()
                .map(|fp| {
                    let full: PathBuf = [storage_path.as_str(), fp.as_str()].iter().collect();
                    full.exists()
                })
                .unwrap_or(false);

            DocumentSummary {
                id: doc.id,
                title: doc.title,
                doc_type: doc.doc_type,
                created_at: doc.created_at,
                file_path: doc.file_path,
                content_hash: None, // Not stored on the Document model struct
                evidence_count,
                has_pdf,
            }
        })
        .collect();

    let total = documents.len();
    Ok(Json(ListDocumentsResponse { documents, total }))
}
