use axum::{
    extract::{Path, State},
    response::Response,
    Json,
};
use chrono::DateTime;
use serde_json::json;

use crate::{
    auth::{require_edit, AuthUser},
    dto::{DocumentCreateRequest, DocumentDto, DocumentUpdateRequest},
    error::AppError,
    repositories::document_repository::{DocumentRepository, DocumentRepositoryError},
    state::AppState,
};

const ALLOWED_DOC_TYPES: &[&str] = &[
    "complaint",
    "discovery",
    "motion",
    "court_ruling",
    "appellate_brief",
    "affidavit",
];

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
            message: "doc_type must be one of: complaint, discovery, motion, court_ruling, appellate_brief, affidavit".to_string(),
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
    user: Option<AuthUser>,
    State(state): State<AppState>,
) -> Result<Json<Vec<DocumentDto>>, AppError> {
    if let Some(ref u) = user {
        tracing::info!("{} GET /documents", u.username);
    }
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
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<DocumentDto>, AppError> {
    if let Some(ref u) = user {
        tracing::info!("{} GET /documents/{}", u.username, id);
    }
    let repo = DocumentRepository::new(state.graph.clone());
    let document = repo.get_document_by_id(&id).await.map_err(map_repo_error)?;

    Ok(Json(DocumentDto::from(document)))
}

pub async fn create_document(
    user: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<DocumentCreateRequest>,
) -> Result<(axum::http::StatusCode, Json<DocumentDto>), AppError> {
    require_edit(&user)?;
    tracing::info!("{} POST /documents", user.username);
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
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<DocumentUpdateRequest>,
) -> Result<Json<DocumentDto>, AppError> {
    require_edit(&user)?;
    tracing::info!("{} PUT /documents/{}", user.username, id);
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

/// Serve a document's source file.
///
/// ## Why this delegates to the pipeline file module
/// The on-disk file location lives in the **Postgres** `documents.file_path`
/// column (written at upload time), NOT on the Neo4j `Document` node — the
/// ingest pipeline never sets a path on the graph node. So this public route
/// reuses the single Postgres-backed serving function shared with the admin
/// file route (`pipeline::file::serve_document_file`): one implementation, two
/// thin handlers, no drift.
///
/// (Previously this handler read the Neo4j node's `file_path`, which is always
/// null for pipeline-ingested documents — so it 404'd for the entire corpus,
/// including the complaint behind the "View Complaint" link.)
///
/// Auth: requires an authenticated `AuthUser`. The model is
/// "authenticated users may view all documents"; anonymous access (the prior
/// `Option<AuthUser>`) was an oversight and is removed so this route matches
/// the admin file route.
pub async fn get_document_file(
    user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    tracing::info!("{} GET /documents/{}/file", user.username, id);
    crate::api::pipeline::file::serve_document_file(&state, &id).await
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
