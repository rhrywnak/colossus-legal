use axum::{extract::State, http::StatusCode, Json};

use crate::dto::PersonsResponse;
use crate::repositories::PersonRepository;
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
