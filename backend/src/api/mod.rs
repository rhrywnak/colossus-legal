use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post, put},
    Router,
};

use crate::state::AppState;

pub mod allegations;
pub mod claims;
pub mod documents;
pub mod import;
pub mod persons;
pub mod schema;

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
        .route("/documents", get(documents::list_documents))
        .route("/documents", post(documents::create_document))
        .route("/documents/:id", get(documents::get_document))
        .route("/documents/:id", put(documents::update_document))
        .route("/import/validate", post(import::validate_import))
        .route("/schema", get(schema::get_schema))
        .route("/persons", get(persons::list_persons))
        .route("/allegations", get(allegations::list_allegations))
}

async fn health_check(State(_state): State<AppState>) -> (StatusCode, &'static str) {
    (StatusCode::OK, "OK")
}
