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

use super::practice_answers::{
    post_close_answer, post_help_opened, post_practice_answer, post_skip_question,
};
use super::practice_deck_read::{
    log_served, read_deck_sources, read_what_changed, scenario_record,
};
use super::practice_editor::{post_edit_question, post_hide_question, post_move_question};
use super::practice_editor_add::post_add_question;
use super::practice_fences::check_sitting;
use super::practice_flag::put_question_flag;
use super::practice_sessions::{get_sitting_route, post_end_session, post_resume, post_start_over};

use crate::{
    auth::AuthUser,
    domain::scenario_code::scenario_code,
    dto::practice::{PracticeDeckPayload, StartSessionRequest, StartSessionResponse},
    error::AppError,
    repositories::pipeline_repository::practice::{start_session, NewSitting},
    services::{
        practice_editor_options::attach_options,
        practice_notes::attribution,
        practice_page::{deck_payload, DeckSources},
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
            "/cases/:slug/scenarios/:scenario_id/practice/answers",
            get(super::practice_one_page::get_practice_answers),
        )
        .route(
            "/cases/:slug/scenarios/:scenario_id/practice/answer-session",
            post(super::practice_one_page::post_answer_session),
        )
        .route(
            "/practice/questions/:question_id/answers",
            get(super::practice_one_page::get_question_answers),
        )
        .route(
            "/cases/:slug/scenarios/:scenario_id/practice/sessions",
            post(post_practice_session),
        )
        .route("/practice/answers", post(post_practice_answer))
        .route("/practice/answers/skip", post(post_skip_question))
        .route("/practice/answers/:answer_id/help", post(post_help_opened))
        .route(
            "/practice/answers/:answer_id/close",
            post(post_close_answer),
        )
        .route("/practice/sessions/:session_id/end", post(post_end_session))
        .route("/practice/sessions/:session_id", get(get_sitting_route))
        .route("/practice/sessions/:session_id/resume", post(post_resume))
        .route(
            "/practice/sessions/:session_id/start-over",
            post(post_start_over),
        )
        // PUT and not POST: writing the same note twice leaves the same row, and
        // clearing is the same call with nothing in it. That is idempotent, which
        // is what PUT means.
        .route(
            "/practice/questions/:question_id/flag",
            put(put_question_flag),
        )
        .merge(part_b_routes())
        .merge(review_loop_routes())
}

/// The review loop's four routes (CC_TASK_REVIEW_LOOP_v1): Chuck's notes to
/// Marie, and the one act that moves his read cursor. Handlers live in
/// `practice_notes_routes` and `practice_review_cursor`.
fn review_loop_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/practice/answers/:answer_id/notes",
            post(super::practice_notes_routes::post_answer_note),
        )
        .route(
            "/practice/questions/:question_id/notes",
            post(super::practice_notes_routes::post_question_note),
        )
        // PUT: striking twice keeps the first striking — idempotent.
        .route(
            "/practice/notes/:note_id/strike",
            put(super::practice_notes_routes::put_strike_note),
        )
        .route(
            "/cases/:slug/scenarios/:scenario_id/practice/review-cursor",
            put(super::practice_review_cursor::put_review_cursor),
        )
}

/// Part B's seven routes, declared together.
///
/// Split from [`routes`] so neither passes Rule 18, and grouped rather than
/// scattered because they arrived as one task: the deck editor, the notes, and
/// the review page. The four editor writes address a question by its own
/// server-minted id, for the same reason the answer routes do; the three that
/// CREATE or READ something scenario-shaped are case- and scenario-scoped,
/// because each has to be told where to look.
fn part_b_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/practice/questions/:question_id/edit",
            post(post_edit_question),
        )
        .route(
            "/practice/questions/:question_id/move",
            post(post_move_question),
        )
        // The drag's endpoint. A sibling of `/move` rather than a flag on it —
        // one step and "put it here" are different operations, and the DTO says
        // so rather than a comment.
        .route(
            "/practice/questions/:question_id/reorder",
            post(super::practice_reorder::post_reorder_question),
        )
        .route(
            "/practice/questions/:question_id/hidden",
            post(post_hide_question),
        )
        .route(
            "/cases/:slug/scenarios/:scenario_id/practice/questions",
            post(post_add_question),
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
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
) -> Result<Json<PracticeDeckPayload>, AppError> {
    let scenario_id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, scenario_id, &slug).await?;

    let record = scenario_record(&state, scenario_id).await?;

    let settings = state.settings.current();
    // Every per-user read on this page is keyed to the signed-in user, and every
    // "today" is compared in the case's own zone. See `row_statuses`.
    let (user_id, _) = attribution(&user);
    let read = read_deck_sources(
        &state,
        scenario_id,
        &user_id,
        &settings.practice_read.case_timezone,
    )
    .await?;
    let news = read_what_changed(&state, scenario_id, &read).await?;
    // Composed BEFORE the payload takes ownership of the points. The add form's
    // picker and the reveal's point list are the same three rows read twice, and
    // one read is what the payload promises.
    let attach = attach_options(&settings, &read.deck_for_changes, &read.points);

    let payload = deck_payload(
        &settings,
        DeckSources {
            scenario_id,
            code: scenario_code(record.code_ordinal),
            title: record.name,
            deck: read.deck,
            points: read.points,
            receipts: &read.receipts,
            last: read.last.as_ref(),
            current: &read.current,
            open: read.open.as_ref(),
            attach_options: attach,
            notes: &read.notes,
            new_since_you_reviewed: read.new_since_you_reviewed,
        },
    );

    log_served(
        &slug,
        scenario_id,
        &payload,
        read.current.len(),
        read.open_total,
        news.changes.len(),
    );
    Ok(Json(payload))
}

/// Open a session. The client keeps the id for the rest of the sitting.
pub async fn post_practice_session(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
    Json(body): Json<StartSessionRequest>,
) -> Result<Json<StartSessionResponse>, AppError> {
    let scenario_id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, scenario_id, &slug).await?;

    check_sitting(&state, scenario_id, &body).await?;
    // The sitting belongs to whoever opened it. See `NewSitting` for why.
    let (user_id, user_name) = attribution(&user);

    let queue = serde_json::json!(body.queue);
    let skipped_today = serde_json::json!(body.skipped_today);
    let session_id = start_session(
        &state.pipeline_pool,
        scenario_id,
        &body.who,
        &NewSitting {
            user_id: &user_id,
            user_name: &user_name,
            count: body.count,
            queue: &queue,
            skipped_today: &skipped_today,
        },
    )
    .await
    .map_err(|e| repo_error("start_session", e))?;

    tracing::info!(
        %scenario_id,
        %session_id,
        who = %body.who,
        dealt = body.queue.len(),
        skipped_today = body.skipped_today.len(),
        "practice session started"
    );
    Ok(Json(StartSessionResponse { session_id }))
}

#[cfg(test)]
#[path = "practice_tests.rs"]
mod tests;
