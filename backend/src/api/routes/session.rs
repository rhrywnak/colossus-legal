//! Session / identity routes: who-am-I, known users, logout.
//!
//! Moved out of `api::mod` by the T1.0 split with its one handler
//! ([`me_with_tracking`]) beside it — the handler exists only to serve
//! `GET /me`, so leaving it behind would have split one thing across two files.

use axum::{extract::State, routing::get, Json, Router};

use crate::api::{logout, pipeline};
use crate::auth::{me_handler, AuthUser, MeResponse};
use crate::repositories::pipeline_repository::users as known_users;
use crate::state::AppState;

/// Session / identity routes: who-am-I, known users, logout.
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(me_with_tracking))
        .route("/users", get(pipeline::users::list_users_handler))
        .route("/logout", get(logout::logout))
}

/// Wrapper around `me_handler` that also records the user in `known_users`.
///
/// The upsert is fire-and-forget: it runs in a background task so it never
/// slows down or fails the `/api/me` response. This is the simplest way to
/// passively track users without adding middleware complexity.
///
/// ## Rust Learning: tokio::spawn for fire-and-forget
///
/// `tokio::spawn` launches a new async task on the runtime. The spawned
/// future runs independently — we don't `.await` the JoinHandle, so the
/// response returns immediately.
async fn me_with_tracking(user: AuthUser, State(state): State<AppState>) -> Json<MeResponse> {
    // Clone the values the background task needs before we move `user`.
    let pool = state.pipeline_pool.clone();
    let username = user.username.clone();
    let display_name = user.display_name.clone();
    let email = user.email.clone();

    tokio::spawn(async move {
        known_users::upsert_known_user(&pool, &username, &display_name, &email)
            .await
            // best-effort: passive user-tracking upsert in a detached task; a DB failure must never fail or delay the /api/me response.
            .ok();
    });

    me_handler(user).await
}
