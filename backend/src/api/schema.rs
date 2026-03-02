use axum::{extract::State, http::StatusCode, Json};

use crate::auth::AuthUser;
use crate::dto::SchemaResponse;
use crate::repositories::SchemaRepository;
use crate::state::AppState;

/// GET /schema - Returns database schema statistics
pub async fn get_schema(
    user: Option<AuthUser>,
    State(state): State<AppState>,
) -> Result<Json<SchemaResponse>, StatusCode> {
    if let Some(ref u) = user {
        tracing::info!("{} GET /schema", u.username);
    }
    let repo = SchemaRepository::new(state.graph.clone());

    match repo.get_schema_stats().await {
        Ok(schema) => Ok(Json(schema)),
        Err(e) => {
            tracing::error!("Schema query failed: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
