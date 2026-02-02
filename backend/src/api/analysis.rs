use axum::{extract::State, http::StatusCode, Json};

use crate::dto::AnalysisResponse;
use crate::repositories::AnalysisRepository;
use crate::state::AppState;

/// GET /analysis - Returns aggregated analysis data for the dashboard
pub async fn get_analysis(
    State(state): State<AppState>,
) -> Result<Json<AnalysisResponse>, StatusCode> {
    let repo = AnalysisRepository::new(state.graph.clone());

    match repo.get_analysis().await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            tracing::error!("Failed to fetch analysis data: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
