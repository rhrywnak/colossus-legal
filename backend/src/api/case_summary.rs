use axum::{extract::State, http::StatusCode, Json};

use crate::dto::case_summary::CaseSummaryResponse;
use crate::repositories::CaseSummaryRepository;
use crate::state::AppState;

/// GET /case-summary — analytical dashboard data for case intelligence briefing
pub async fn get_case_summary(
    State(state): State<AppState>,
) -> Result<Json<CaseSummaryResponse>, StatusCode> {
    let repo = CaseSummaryRepository::new(state.graph.clone());

    match repo.get_case_summary().await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            tracing::error!("Failed to fetch case summary: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
