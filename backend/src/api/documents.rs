use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::DateTime;
use serde::Deserialize;
use serde_json::json;

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

#[derive(Deserialize)]
pub struct RecentDocumentsQuery {
    pub limit: Option<i64>,
}

pub async fn list_recent_documents(
    State(state): State<AppState>,
    Query(params): Query<RecentDocumentsQuery>,
) -> Result<Json<Vec<DocumentDto>>, AppError> {
    let limit = params.limit.unwrap_or(10);
    if limit <= 0 {
        return Err(AppError::BadRequest {
            message: "limit must be positive".to_string(),
            details: json!({ "field": "limit" }),
        });
    }

    let repo = DocumentRepository::new(state.graph.clone());
    let documents = repo
        .list_recent_documents(limit)
        .await
        .map_err(|_| AppError::Internal {
            message: "failed to list recent documents".to_string(),
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
