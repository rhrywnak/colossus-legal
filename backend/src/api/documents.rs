use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use chrono::DateTime;
use serde_json::json;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::{
    dto::{DocumentCreateRequest, DocumentDto, DocumentUpdateRequest},
    error::AppError,
    repositories::document_repository::{DocumentRepository, DocumentRepositoryError},
    state::AppState,
};

const ALLOWED_DOC_TYPES: &[&str] = &["pdf", "motion", "ruling", "evidence", "filing"];

fn validate_title(title: &str) -> Result<(), AppError> {
    if title.trim().is_empty() {
        return Err(AppError::BadRequest {
            message: "title must not be empty".to_string(),
            details: json!({ "field": "title" }),
        });
    }
    Ok(())
}

fn validate_doc_type(doc_type: &str) -> Result<(), AppError> {
    if !ALLOWED_DOC_TYPES.contains(&doc_type) {
        return Err(AppError::BadRequest {
            message: "doc_type must be one of: pdf, motion, ruling, evidence, filing".to_string(),
            details: json!({ "field": "doc_type" }),
        });
    }
    Ok(())
}

fn validate_created_at(created_at: &str) -> Result<String, AppError> {
    DateTime::parse_from_rfc3339(created_at)
        .map(|dt| dt.to_rfc3339())
        .map_err(|_| AppError::BadRequest {
            message: "created_at must be ISO-8601".to_string(),
            details: json!({ "field": "created_at" }),
        })
}

pub async fn list_documents(
    State(state): State<AppState>,
) -> Result<Json<Vec<DocumentDto>>, AppError> {
    let repo = DocumentRepository::new(state.graph.clone());
    let documents = repo
        .list_documents()
        .await
        .map_err(|_| AppError::Internal {
            message: "failed to list documents".to_string(),
        })?;

    let dtos = documents.into_iter().map(DocumentDto::from).collect();

    Ok(Json(dtos))
}

pub async fn get_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<DocumentDto>, AppError> {
    let repo = DocumentRepository::new(state.graph.clone());
    let document = repo.get_document_by_id(&id).await.map_err(map_repo_error)?;

    Ok(Json(DocumentDto::from(document)))
}

pub async fn create_document(
    State(state): State<AppState>,
    Json(payload): Json<DocumentCreateRequest>,
) -> Result<(axum::http::StatusCode, Json<DocumentDto>), AppError> {
    validate_title(&payload.title)?;
    validate_doc_type(&payload.doc_type)?;
    let normalized_created_at = match payload.created_at.as_deref() {
        Some(value) => Some(validate_created_at(value)?),
        None => None,
    };

    let repo = DocumentRepository::new(state.graph.clone());
    let document = repo
        .create_document(DocumentCreateRequest {
            created_at: normalized_created_at,
            ..payload
        })
        .await
        .map_err(map_repo_error)?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(DocumentDto::from(document)),
    ))
}

pub async fn update_document(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<DocumentUpdateRequest>,
) -> Result<Json<DocumentDto>, AppError> {
    if let Some(title) = payload.title.as_deref() {
        validate_title(title)?;
    }
    if let Some(doc_type) = payload.doc_type.as_deref() {
        validate_doc_type(doc_type)?;
    }
    let normalized_created_at = match payload.created_at.as_deref() {
        Some(value) => Some(validate_created_at(value)?),
        None => None,
    };

    let repo = DocumentRepository::new(state.graph.clone());
    let updated = repo
        .update_document(
            &id,
            DocumentUpdateRequest {
                created_at: normalized_created_at,
                ..payload
            },
        )
        .await
        .map_err(map_repo_error)?;

    Ok(Json(DocumentDto::from(updated)))
}

/// Serve a document's PDF file
pub async fn get_document_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    // 1. Get document from Neo4j
    let repo = DocumentRepository::new(state.graph.clone());
    let document = repo.get_document_by_id(&id).await.map_err(map_repo_error)?;

    // 2. Check if file_path exists
    let file_path = document.file_path.ok_or_else(|| AppError::NotFound {
        message: "Document has no associated file".to_string(),
    })?;

    // 3. Validate filename (security: prevent path traversal)
    if file_path.contains("..") || file_path.contains('/') || file_path.contains('\\') {
        return Err(AppError::BadRequest {
            message: "Invalid file path".to_string(),
            details: json!({}),
        });
    }

    // 4. Build full path
    let full_path = format!("{}/{}", state.config.document_storage_path, file_path);

    // 5. Open file
    let file = File::open(&full_path).await.map_err(|_| AppError::NotFound {
        message: "File not found on disk".to_string(),
    })?;

    // 6. Stream response
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    // 7. Return with PDF headers
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/pdf")
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{}\"", file_path),
        )
        .body(body)
        .unwrap())
}

fn map_repo_error(err: DocumentRepositoryError) -> AppError {
    match err {
        DocumentRepositoryError::NotFound => AppError::NotFound {
            message: "document not found".to_string(),
        },
        DocumentRepositoryError::Mapping(_) | DocumentRepositoryError::Value(_) => {
            AppError::Internal {
                message: "failed to process document".to_string(),
            }
        }
        DocumentRepositoryError::CreationFailed => AppError::Internal {
            message: "failed to create document".to_string(),
        },
        DocumentRepositoryError::Neo4j(_) => AppError::Internal {
            message: "database error".to_string(),
        },
    }
}
