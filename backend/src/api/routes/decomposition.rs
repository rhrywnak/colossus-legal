//! Decomposition intelligence: characterizations, per-allegation detail,
//! and rebuttals.

use axum::{routing::get, Router};

use crate::api::decomposition;
use crate::state::AppState;

/// Decomposition intelligence: characterizations, per-allegation detail,
/// and rebuttals.
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/allegations/:id/detail",
            get(decomposition::get_allegation_detail),
        )
        .route("/rebuttals", get(decomposition::list_rebuttals))
}
