use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::dto::person_detail::PersonDetailResponse;
use crate::dto::PersonsResponse;
use crate::repositories::{PersonDetailRepository, PersonRepository};
use crate::state::AppState;

/// GET /persons - Returns all persons in the database
pub async fn list_persons(
    State(state): State<AppState>,
) -> Result<Json<PersonsResponse>, StatusCode> {
    let repo = PersonRepository::new(state.graph.clone());

    match repo.list_persons().await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            tracing::error!("Failed to fetch persons: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /persons/:id/detail - Returns person profile with all statements
pub async fn get_person_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PersonDetailResponse>, StatusCode> {
    let repo = PersonDetailRepository::new(state.graph.clone());

    match repo.get_person_detail(&id).await {
        Ok(Some(response)) => Ok(Json(response)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to fetch person detail for {}: {:?}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
