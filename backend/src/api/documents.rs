use axum::{extract::State, Json};

use crate::{
    dto::DocumentDto, error::AppError, repositories::document_repository::DocumentRepository,
    state::AppState,
};

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
