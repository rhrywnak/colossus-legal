//! Claim CRUD plus the motion-claims read.

use axum::{
    routing::{get, post, put},
    Router,
};

use crate::api::claims;
use crate::state::AppState;

/// Claim CRUD plus the motion-claims read.
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/claims", get(claims::list_claims))
        .route("/claims/:id", get(claims::get_claim))
        .route("/claims", post(claims::create_claim))
        .route("/claims/:id", put(claims::update_claim))
        .route("/motion-claims", get(claims::list_motion_claims))
}
