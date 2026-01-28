use axum::{extract::State, http::StatusCode, Json};

use crate::dto::EvidenceResponse;
use crate::repositories::EvidenceRepository;
use crate::state::AppState;

/// GET /evidence - Returns all evidence items
pub async fn list_evidence(
    State(state): State<AppState>,
) -> Result<Json<EvidenceResponse>, StatusCode> {
    let repo = EvidenceRepository::new(state.graph.clone());

    match repo.list_evidence().await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            tracing::error!("Failed to fetch evidence: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
