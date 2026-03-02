use axum::{extract::State, http::StatusCode, Json};

use crate::auth::AuthUser;
use crate::dto::CaseResponse;
use crate::repositories::CaseRepository;
use crate::state::AppState;

/// GET /case - Returns case metadata, parties, and stats
pub async fn get_case(
    user: Option<AuthUser>,
    State(state): State<AppState>,
) -> Result<Json<CaseResponse>, StatusCode> {
    if let Some(ref u) = user {
        tracing::info!("{} GET /case", u.username);
    }
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
