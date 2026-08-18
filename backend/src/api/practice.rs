//! HTTP routes for Marie's practice drill (PRACTICE_SESSION_DESIGN_v1 §4–§5).
//!
//! - `GET  /cases/:slug/scenarios/:id/practice`          → the deck, in one read
//! - `POST /cases/:slug/scenarios/:id/practice/sessions` → open a sitting
//! - `POST /practice/answers`                            → record one answer + the read
//! - `POST /practice/answers/:id/help`                   → she opened the drawer
//! - `POST /practice/sessions/:id/end`                   → close it, return Chuck's sheet
//!
//! ## Why the deck is ONE request and the session is four states of one page
//!
//! A witness moving from a question to its reveal must never wait on a network,
//! and must never see a screen fail between them. So S0–S3 are one page holding
//! one payload; the only calls made mid-session are the writes, and each is small.
//!
//! ## Why the answer route is NOT case-scoped
//!
//! `session_id` and `answer_id` are server-minted handles the browser only ever
//! learns by having opened the session. Adding a case slug to those paths would
//! be ceremony rather than a fence — and the fence that matters IS enforced: an
//! answer's question must belong to its session's scenario, which
//! [`record_answer`] checks before writing.
//!
//! ## CRITICAL — the pipeline pool
//!
//! Every table here lives in `colossus_legal_v2`: `&state.pipeline_pool`.

use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use uuid::Uuid;

use super::practice_answers::{
    post_close_answer, post_help_opened, post_practice_answer, put_question_flag,
};

use crate::{
    auth::AuthUser,
    domain::scenario_code::scenario_code,
    dto::practice::{
        PracticeDeckPayload, PracticeSheetPayload, StartSessionRequest, StartSessionResponse,
    },
    error::AppError,
    repositories::pipeline_repository::{
        get_scenario,
        practice::{
            end_session, last_ended_session, list_deck, list_point_receipts, list_points,
            session_queue_len, session_scenario, sheet_rows, start_session,
        },
    },
    services::{
        practice_page::{deck_payload, DeckSources},
        practice_sheet::sheet_payload,
    },
    state::AppState,
};

use super::scenario_facts::{ensure_scenario_in_case, parse_scenario_id};

/// This module's routes, declared beside their handlers.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/cases/:slug/scenarios/:scenario_id/practice",
            get(get_practice_deck),
        )
        .route(
            "/cases/:slug/scenarios/:scenario_id/practice/sessions",
            post(post_practice_session),
        )
        .route("/practice/answers", post(post_practice_answer))
        .route("/practice/answers/:answer_id/help", post(post_help_opened))
        .route(
            "/practice/answers/:answer_id/close",
            post(post_close_answer),
        )
        .route("/practice/sessions/:session_id/end", post(post_end_session))
        // PUT and not POST: writing the same note twice leaves the same row, and
        // clearing is the same call with nothing in it. That is idempotent, which
        // is what PUT means.
        .route(
            "/practice/questions/:question_id/flag",
            put(put_question_flag),
        )
}

/// Turn a repository failure into a 500 that says nothing, having logged
/// everything. The operation name is the part a log reader needs.
pub(super) fn repo_error(operation: &'static str, error: impl std::fmt::Display) -> AppError {
    tracing::error!(operation, error = %error, "practice: database call failed");
    AppError::Internal {
        message: "the practice drill could not reach its store".to_string(),
    }
}

/// The whole page, in one read.
///
/// An empty deck is a 200 with no questions — the page says "no practice deck
/// yet — seed it" in the store's words. A scenario that does not exist, or one
/// reached through the wrong case, is a 404. Two states, two observables.
pub async fn get_practice_deck(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
) -> Result<Json<PracticeDeckPayload>, AppError> {
    let scenario_id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, scenario_id, &slug).await?;

    let record = get_scenario(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| repo_error("get_scenario", e))?
        .ok_or_else(|| AppError::NotFound {
            message: format!("scenario {scenario_id} not found"),
        })?;

    let deck = list_deck(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| repo_error("list_deck", e))?;
    let points = list_points(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| repo_error("list_points", e))?;
    let receipts = list_point_receipts(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| repo_error("list_point_receipts", e))?;
    let last = last_ended_session(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| repo_error("last_ended_session", e))?;

    let settings = state.settings.current();
    let payload = deck_payload(
        &settings,
        DeckSources {
            scenario_id,
            code: scenario_code(record.code_ordinal),
            title: record.name,
            deck,
            points,
            receipts: &receipts,
            last: last.as_ref(),
        },
    );

    tracing::info!(
        slug = %slug,
        %scenario_id,
        questions = payload.questions.len(),
        points = payload.points.len(),
        "served the practice deck"
    );
    Ok(Json(payload))
}

/// Open a session. The client keeps the id for the rest of the sitting.
pub async fn post_practice_session(
    _user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
    Json(body): Json<StartSessionRequest>,
) -> Result<Json<StartSessionResponse>, AppError> {
    let scenario_id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, scenario_id, &slug).await?;

    // The column has a CHECK, but a CHECK violation is a 500 with a constraint
    // name in it. Refusing here makes a bad `who` a 400 that says which values
    // exist — the difference between a client bug found in review and one found
    // in a log at midnight.
    if !matches!(body.who.as_str(), "george" | "chuck" | "mixed") {
        return Err(AppError::BadRequest {
            message: "who must be george, chuck or mixed".to_string(),
            details: serde_json::json!({ "field": "who", "value": body.who }),
        });
    }

    let session_id = start_session(&state.pipeline_pool, scenario_id, &body.who)
        .await
        .map_err(|e| repo_error("start_session", e))?;

    tracing::info!(%scenario_id, %session_id, who = %body.who, "practice session started");
    Ok(Json(StartSessionResponse { session_id }))
}

/// Record one answer, and ask the model for its one sentence.
///
/// ## Why the read cannot fail this request
///
/// Her answer is worth recording whatever the model does. A failed read is
/// stored as `read_text = NULL` with the reason in `read_error`; the screen shows
/// the stored "no system read this time" line and every other box stands. That is
/// the design's own instruction, and it is also the only behaviour that does not
/// throw away a witness's typed sentence because a vendor was slow.
/// Close the session and return Chuck's sheet.
pub async fn post_end_session(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> Result<Json<PracticeSheetPayload>, AppError> {
    let scenario_id = session_scenario(&state.pipeline_pool, session_id)
        .await
        .map_err(|e| repo_error("session_scenario", e))?
        .ok_or_else(|| AppError::NotFound {
            message: format!("practice session {session_id} not found"),
        })?;

    end_session(&state.pipeline_pool, session_id)
        .await
        .map_err(|e| repo_error("end_session", e))?;

    let record = get_scenario(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| repo_error("get_scenario", e))?
        .ok_or_else(|| AppError::NotFound {
            message: format!("scenario {scenario_id} not found"),
        })?;
    let rows = sheet_rows(&state.pipeline_pool, session_id)
        .await
        .map_err(|e| repo_error("sheet_rows", e))?;

    // "Ended early" is a comparison against the queue she STARTED with, which is
    // why the queue is stored on the session. Unknown (no stored queue) is
    // reported as NOT early — the sheet never claims a fact it cannot source.
    let queue_len = session_queue_len(&state.pipeline_pool, session_id)
        .await
        .map_err(|e| repo_error("session_queue_len", e))?;
    let ended_early = queue_len.is_some_and(|n| rows.len() < n.max(0) as usize);

    let settings = state.settings.current();
    let payload = sheet_payload(
        &settings,
        &scenario_code(record.code_ordinal),
        chrono::Utc::now(),
        rows,
        ended_early,
    );

    tracing::info!(%session_id, rows = payload.rows.len(), "practice session ended");
    Ok(Json(payload))
}

#[cfg(test)]
#[path = "practice_tests.rs"]
mod tests;
