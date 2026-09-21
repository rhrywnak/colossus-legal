//! The question chat's routes (CC_TASK_CHAT_ENGINE_v1). Thin handlers — the work
//! is `services::chat_question_*`.
//!
//! - `GET  /chat/question/:question_id/threads` — the switcher and header
//! - `GET  /chat/question/:question_id/threads/:username` — one person's thread
//! - `POST /chat/question/:question_id/threads/:username/messages` — send; the reply
//!   STREAMS back as server-sent events. Only the thread's owner may send (403).
//! - `POST /chat/question/:question_id/threads/:username/read` — move your read mark
//! - `GET  /chat/question/:question_id/earlier` — the old dock's shared thread,
//!   read-only (ADDENDUM_1). It has NO write verb: a POST is a 405 by construction.
//!
//! Any signed-in user may read every thread (Roman's ruling, 2026-09-21).
//!
//! ## Rust Learning: `axum::response::sse::Sse`
//!
//! `Sse::new(stream)` turns a `Stream` of `Event`s into a `text/event-stream`
//! response that flushes each event as it is produced. The stream here is the
//! receiving end of the channel the turn's task writes to, mapped event by
//! event. `KeepAlive` adds a comment line on a quiet stream so a proxy does not
//! decide the connection is dead while the model is thinking.

use std::convert::Infallible;

use axum::{
    extract::{Path, State},
    http::{header, HeaderName, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use futures::StreamExt;
use tokio_stream::wrappers::UnboundedReceiverStream;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    dto::chat_discussion::{
        ChatThreadsPayload, MarkReadRequest, SendMessageRequest, ThreadPayload,
    },
    error::AppError,
    repositories::pipeline_repository::chat_discussions::{
        find_discussion, mark_read, ANCHOR_QUESTION,
    },
    services::{
        chat_question_error::{store, ChatRunError},
        chat_question_run::prepare_turn,
        chat_question_stream::spawn_turn,
        chat_question_threads::{earlier_payload, thread_payload, threads_payload},
    },
    state::AppState,
};

/// Tells nginx-style proxies (and Traefik's buffering middleware, if enabled)
/// not to hold the stream back. CONST: HTTP protocol vocabulary.
const NO_BUFFERING: (&str, &str) = ("x-accel-buffering", "no");

/// The five routes, merged into the API router.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/chat/question/:question_id/threads", get(get_threads))
        .route(
            "/chat/question/:question_id/threads/:username",
            get(get_thread),
        )
        .route(
            "/chat/question/:question_id/threads/:username/messages",
            post(post_message),
        )
        .route(
            "/chat/question/:question_id/threads/:username/read",
            post(post_read),
        )
        .route("/chat/question/:question_id/earlier", get(get_earlier))
}

async fn get_threads(
    user: AuthUser,
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
) -> Result<Json<ChatThreadsPayload>, AppError> {
    threads_payload(&state, question_id, &user.username)
        .await
        .map(Json)
        .map_err(|e| e.into_app_error(question_id, &user.username))
}

async fn get_thread(
    user: AuthUser,
    State(state): State<AppState>,
    Path((question_id, username)): Path<(Uuid, String)>,
) -> Result<Json<ThreadPayload>, AppError> {
    thread_payload(&state, question_id, &username, &user.username)
        .await
        .map(Json)
        .map_err(|e| e.into_app_error(question_id, &user.username))
}

async fn get_earlier(
    user: AuthUser,
    State(state): State<AppState>,
    Path(question_id): Path<Uuid>,
) -> Result<Json<ThreadPayload>, AppError> {
    earlier_payload(&state, question_id)
        .await
        .map(Json)
        .map_err(|e| e.into_app_error(question_id, &user.username))
}

/// Send one message. Every refusal happens BEFORE the stream opens, as a plain
/// status; once it opens, failures arrive as a `failed` event.
async fn post_message(
    user: AuthUser,
    State(state): State<AppState>,
    Path((question_id, username)): Path<(Uuid, String)>,
    Json(body): Json<SendMessageRequest>,
) -> Result<Response, AppError> {
    let turn = prepare_turn(&state, question_id, &username, &user.username, &body.text)
        .await
        .map_err(|e| e.into_app_error(question_id, &user.username))?;
    tracing::info!(%question_id, by = %user.username, seq = turn.user_seq, "question chat: message stored, reply streaming");
    let events = UnboundedReceiverStream::new(spawn_turn(state.clone(), turn)).map(|e| {
        let event = Event::default()
            .event(e.name())
            .json_data(e.data())
            .unwrap_or_else(|err| {
                // `json_data` fails only on a serialization error, which a Value
                // cannot produce; named anyway so the stream says what went wrong.
                Event::default()
                    .event("failed")
                    .data(format!("{{\"failure\":\"failed\",\"detail\":\"{err}\"}}"))
            });
        Ok::<Event, Infallible>(event)
    });
    Ok((
        [
            (HeaderName::from_static(NO_BUFFERING.0), NO_BUFFERING.1),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        Sse::new(events).keep_alive(KeepAlive::default()),
    )
        .into_response())
}

/// Move the viewer's read mark on a thread forward (never back).
async fn post_read(
    user: AuthUser,
    State(state): State<AppState>,
    Path((question_id, username)): Path<(Uuid, String)>,
    Json(body): Json<MarkReadRequest>,
) -> Result<StatusCode, AppError> {
    let result = async {
        let found = find_discussion(
            &state.pipeline_pool,
            ANCHOR_QUESTION,
            &question_id.to_string(),
            &username,
        )
        .await
        .map_err(store("find_discussion", question_id))?;
        // A thread that does not exist yet has nothing to mark — not an error.
        if let Some(d) = found {
            mark_read(&state.pipeline_pool, d.id, &user.username, body.seq)
                .await
                .map_err(store("mark_read", question_id))?;
        }
        Ok::<(), ChatRunError>(())
    }
    .await;
    result
        .map(|()| StatusCode::NO_CONTENT)
        .map_err(|e| e.into_app_error(question_id, &user.username))
}
