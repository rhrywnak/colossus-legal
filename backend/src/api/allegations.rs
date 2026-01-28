use axum::{extract::State, http::StatusCode, Json};

use crate::dto::AllegationsResponse;
use crate::repositories::AllegationRepository;
use crate::state::AppState;

/// GET /allegations - Returns all complaint allegations
pub async fn list_allegations(
    State(state): State<AppState>,
) -> Result<Json<AllegationsResponse>, StatusCode> {
    let repo = AllegationRepository::new(state.graph.clone());

    match repo.list_allegations().await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            tracing::error!("Failed to fetch allegations: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
