use axum::{extract::State, http::StatusCode, Json};

use crate::auth::AuthUser;
use crate::dto::HarmsResponse;
use crate::repositories::HarmRepository;
use crate::state::AppState;

/// GET /harms - Returns all harms/damages
pub async fn list_harms(
    user: Option<AuthUser>,
    State(state): State<AppState>,
) -> Result<Json<HarmsResponse>, StatusCode> {
    if let Some(ref u) = user {
        tracing::info!("{} GET /harms", u.username);
    }
    let repo = HarmRepository::new(state.graph.clone());

    match repo.list_harms().await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            tracing::error!("Failed to fetch harms: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
