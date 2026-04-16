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
//! The `sha2` crate uses a trait-based API: `Sha256::new()` → `.update(bytes)`
//! → `.finalize()` → `format!("{:x}", hash)`. The `Digest` trait is generic —
//! swap `Sha256` for `Sha512` and the code stays the same.

use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::document_repository::DocumentRepository;
use crate::state::AppState;

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
    pub status: String,
}

/// Response for the admin document list.
#[derive(Debug, Serialize)]
pub struct ListDocumentsResponse {
    pub documents: Vec<DocumentSummary>,
    pub total: usize,
}

/// Request body for registering a document with optional content hash verification.
///
/// Both `id` and `file_path` are optional:
/// - If `id` is omitted, the repository auto-generates a timestamp-based ID.
/// - If `file_path` is omitted, SHA-256 hashing and PDF verification are skipped.
#[derive(Debug, serde::Deserialize)]
pub struct RegisterDocumentRequest {
    /// Optional document ID. If omitted or empty, the repo generates one.
    pub id: Option<String>,
    pub title: String,
    pub doc_type: String,
    pub created_at: Option<String>,
    pub description: Option<String>,
    /// PDF filename on disk (relative to DOCUMENT_STORAGE_PATH).
    /// When provided, the file is verified and SHA-256 hashed.
    pub file_path: Option<String>,
}

/// POST /api/admin/documents — Register a document with PDF verification.
pub async fn register_document(
    user: AuthUser,
    State(state): State<AppState>,
    Json(req): Json<RegisterDocumentRequest>,
) -> Result<(StatusCode, Json<RegisterDocumentResponse>), AppError> {
    require_admin(&user)?;
    tracing::info!(
        user = %user.username,
        doc_id = ?req.id,
        "POST /api/admin/documents"
    );

    // Resolve the explicit ID, if provided. Empty strings treated as None.
    let explicit_id = req
        .id
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    // 1. If file_path is provided, validate and compute content hash.
    //    If omitted, skip PDF verification and hashing entirely.
    let (file_path, content_hash) = match &req.file_path {
        Some(fp) if !fp.trim().is_empty() => {
            let fp = fp.trim().to_string();

            // Prevent path traversal
            if fp.contains("..") || fp.contains('/') || fp.contains('\\') {
                return Err(AppError::BadRequest {
                    message: "file_path must be a plain filename, no path separators or .."
                        .to_string(),
                    details: json!({ "field": "file_path" }),
                });
            }

            // Build the full path and check the PDF exists on disk
            let pdf_path: PathBuf = [&state.config.document_storage_path, &fp].iter().collect();

            if !pdf_path.exists() {
                return Err(AppError::BadRequest {
                    message: format!(
                        "PDF not found at {}. Upload the file before registering.",
                        pdf_path.display()
                    ),
                    details: json!({ "field": "file_path", "path": pdf_path.display().to_string() }),
                });
            }

            // Compute SHA-256 content hash
            let file_bytes = tokio::fs::read(&pdf_path)
                .await
                .map_err(|e| AppError::Internal {
                    message: format!("Failed to read PDF: {e}"),
                })?;

            let mut hasher = Sha256::new();
            hasher.update(&file_bytes);
            let hash = format!("{:x}", hasher.finalize());

            (Some(fp), Some(hash))
        }
        _ => (None, None),
    };

    let repo = DocumentRepository::new(state.graph.clone());

    // 2. Check for duplicate by explicit ID (only if one was provided)
    if let Some(ref id) = explicit_id {
        match repo.get_document_by_id(id).await {
            Ok(_) => {
                return Err(AppError::Conflict {
                    message: format!("Document with id '{id}' already exists"),
                    details: json!({ "existing_id": id }),
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
    }

    // 3. Check for duplicate by content hash (only if we computed one)
    if let Some(ref hash) = content_hash {
        match repo.find_by_content_hash(hash).await {
            Ok(Some(existing)) => {
                return Err(AppError::Conflict {
                    message: format!(
                        "A document with identical content already exists: '{}' ({})",
                        existing.title, existing.id
                    ),
                    details: json!({
                        "existing_id": existing.id,
                        "existing_title": existing.title,
                        "content_hash": hash,
                    }),
                });
            }
            Ok(None) => {}
            Err(e) => {
                return Err(AppError::Internal {
                    message: format!("Failed to check content hash: {e:?}"),
                });
            }
        }
    }

    // 4. Create the Document node via the repository.
    //    Use explicit ID if provided, otherwise the repo auto-generates one.
    let create_req = crate::dto::document::DocumentCreateRequest {
        title: req.title.clone(),
        doc_type: req.doc_type,
        created_at: req.created_at,
        description: req.description,
        file_path,
        uploaded_at: None,
        related_claim_id: None,
        source_url: None,
    };

    let document = match explicit_id {
        Some(id) => repo.create_document_with_id(&id, create_req).await,
        None => repo.create_document(create_req).await,
    }
    .map_err(|e| AppError::Internal {
        message: format!("Failed to create document: {e:?}"),
    })?;

    // 5. Set the content_hash if we computed one
    if let Some(ref hash) = content_hash {
        repo.set_content_hash(&document.id, hash)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Document created but failed to set content_hash: {e:?}"),
            })?;
    }

    tracing::info!(user = %user.username, doc_id = %document.id, "Document registered");

    let doc_id = document.id.clone();
    let doc_title = document.title.clone();
    let doc_type = document.doc_type.clone().unwrap_or_default();

    log_admin_action(
        &state.audit_repo,
        &user.username,
        "document.register",
        Some("document"),
        Some(&doc_id),
        Some(json!({ "title": &doc_title, "doc_type": &doc_type })),
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(RegisterDocumentResponse {
            pdf_url: format!("/documents/{doc_id}/file"),
            id: doc_id,
            title: doc_title,
            content_hash: content_hash.unwrap_or_default(),
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
                status: doc.status,
            }
        })
        .collect();

    let total = documents.len();
    Ok(Json(ListDocumentsResponse { documents, total }))
}
