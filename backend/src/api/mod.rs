use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post, put},
    Router,
};

use crate::state::AppState;
pub mod claims;

/// Minimal API router.
///
/// We intentionally expose only a health check here. All of the
/// original Codex-generated routes and logic are preserved in the
/// `wip/codex-refactor-2025-11` branch and can be reintroduced later
/// in small, well-structured feature branches.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .route("/claims", get(claims::list_claims))
        .route("/claims/:id", get(claims::get_claim))
        .route("/claims", post(claims::create_claim))
        .route("/claims/:id", put(claims::update_claim))
}

async fn health_check(
    State(_state): State<AppState>,
) -> (StatusCode, &'static str) {
    (StatusCode::OK, "OK")
}
