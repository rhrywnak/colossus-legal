//! Saved-query list and run.

use axum::{routing::get, Router};

use crate::api::queries;
use crate::state::AppState;

/// Saved-query list and run.
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/queries", get(queries::list_queries))
        .route("/queries/:id/run", get(queries::run_query))
}
