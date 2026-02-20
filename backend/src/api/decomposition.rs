// =============================================================================
// backend/src/api/decomposition.rs
// =============================================================================
//
// Axum route handlers for the Decomposition API (Phase F, Feature F.1).
//
// Follows the exact same pattern as allegations.rs / evidence_chain.rs:
//   - State(state) to get AppState
//   - Create repo from state.graph.clone()
//   - match on result, log errors, return StatusCode
// =============================================================================

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::dto::decomposition::{
    AllegationDetailResponse, DecompositionResponse, RebuttalsResponse,
};
use crate::repositories::{
    AllegationDetailRepository, DecompositionRepository, RebuttalsRepository,
};
use crate::state::AppState;

/// GET /decomposition — Overview of all 18 allegations with characterizations
pub async fn list_decomposition(
    State(state): State<AppState>,
) -> Result<Json<DecompositionResponse>, StatusCode> {
    let repo = DecompositionRepository::new(state.graph.clone());

    match repo.get_decomposition().await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            tracing::error!("Failed to fetch decomposition: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /allegations/:id/detail — Deep dive into one allegation
pub async fn get_allegation_detail(
    State(state): State<AppState>,
    Path(allegation_id): Path<String>,
) -> Result<Json<AllegationDetailResponse>, StatusCode> {
    let repo = AllegationDetailRepository::new(state.graph.clone());

    match repo.get_allegation_detail(&allegation_id).await {
        Ok(Some(response)) => Ok(Json(response)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!(
                "Failed to fetch allegation detail for '{}': {:?}",
                allegation_id,
                e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /rebuttals — All REBUTS grouped by George's claims
pub async fn list_rebuttals(
    State(state): State<AppState>,
) -> Result<Json<RebuttalsResponse>, StatusCode> {
    let repo = RebuttalsRepository::new(state.graph.clone());

    match repo.get_rebuttals().await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            tracing::error!("Failed to fetch rebuttals: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
