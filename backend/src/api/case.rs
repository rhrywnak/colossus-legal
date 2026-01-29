use axum::{extract::State, http::StatusCode, Json};

use crate::dto::CaseResponse;
use crate::repositories::CaseRepository;
use crate::state::AppState;

/// GET /case - Returns case metadata, parties, and stats
pub async fn get_case(State(state): State<AppState>) -> Result<Json<CaseResponse>, StatusCode> {
    let repo = CaseRepository::new(state.graph.clone());

    match repo.get_case().await {
        Ok(Some(response)) => Ok(Json(response)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to fetch case: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
