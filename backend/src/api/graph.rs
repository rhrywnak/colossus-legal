use axum::{extract::{Query, State}, http::StatusCode, Json};
use serde::Deserialize;

use crate::dto::GraphResponse;
use crate::repositories::GraphRepository;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct GraphQuery {
    pub count_id: Option<String>,
}

/// GET /graph/legal-proof - Returns nodes/edges for evidence chain visualization
pub async fn get_legal_proof_graph(
    State(state): State<AppState>,
    Query(params): Query<GraphQuery>,
) -> Result<Json<GraphResponse>, StatusCode> {
    let repo = GraphRepository::new(state.graph.clone());

    match repo.get_legal_proof_graph(params.count_id.as_deref()).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            tracing::error!("Failed to fetch graph: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
