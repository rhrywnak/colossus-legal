use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::dto::EvidenceChainResponse;
use crate::repositories::EvidenceChainRepository;
use crate::state::AppState;

/// GET /allegations/:id/evidence-chain
///
/// Returns the complete evidence chain for a single allegation,
/// including motion claims, evidence items, and linked documents.
pub async fn get_evidence_chain(
    State(state): State<AppState>,
    Path(allegation_id): Path<String>,
) -> Result<Json<EvidenceChainResponse>, StatusCode> {
    let repo = EvidenceChainRepository::new(state.graph.clone());

    match repo.get_evidence_chain(&allegation_id).await {
        Ok(Some(response)) => Ok(Json(response)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            tracing::error!("Failed to fetch evidence chain: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
