use axum::{extract::State, http::StatusCode, Json};

use crate::dto::ContradictionsResponse;
use crate::repositories::ContradictionRepository;
use crate::state::AppState;

/// GET /contradictions - Returns all evidence contradictions
pub async fn list_contradictions(
    State(state): State<AppState>,
) -> Result<Json<ContradictionsResponse>, StatusCode> {
    let repo = ContradictionRepository::new(state.graph.clone());

    match repo.list_contradictions().await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            tracing::error!("Failed to fetch contradictions: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
