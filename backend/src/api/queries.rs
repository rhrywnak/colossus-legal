use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::auth::AuthUser;
use crate::dto::query::{QueryListResponse, QueryResultResponse};
use crate::repositories::query_repository::QueryRepository;
use crate::state::AppState;

/// GET /queries — list all available pre-registered queries.
pub async fn list_queries(
    user: Option<AuthUser>,
    State(state): State<AppState>,
) -> Json<QueryListResponse> {
    if let Some(ref u) = user {
        tracing::info!("{} GET /queries", u.username);
    }
    let repo = QueryRepository::new(state.graph.clone());
    Json(repo.list_queries())
}

/// GET /queries/:id/run — execute a pre-registered query by id.
pub async fn run_query(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<QueryResultResponse>, StatusCode> {
    if let Some(ref u) = user {
        tracing::info!("{} GET /queries/{}/run", u.username, id);
    }
    let repo = QueryRepository::new(state.graph.clone());
    match repo.run_query(&id).await {
        Ok(result) => Ok(Json(result)),
        Err(crate::repositories::query_repository::QueryRepositoryError::NotFound(_)) => {
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            tracing::error!("Failed to run query {}: {:?}", id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
