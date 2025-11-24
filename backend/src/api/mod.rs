use axum::{
    extract::State,
    http::StatusCode,
    routing::get,
    Router,
};

use crate::state::AppState;

/// Minimal API router for the backend.
///
/// For now, we keep this intentionally small: a single health
/// check endpoint. All of the more complex Codex-generated routes
/// are preserved in the WIP branch (`wip/codex-refactor-2025-11`)
/// and can be reintroduced later in small, well-designed steps.
pub fn create_router() -> Router<AppState> {
    Router::new().route("/health", get(health_check))
}

async fn health_check(
    State(_state): State<AppState>,
) -> (StatusCode, &'static str) {
    (StatusCode::OK, "OK")
}
