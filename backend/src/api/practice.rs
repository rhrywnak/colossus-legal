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
use std::collections::HashSet;

use uuid::Uuid;

use super::practice_answers::{
    post_close_answer, post_help_opened, post_practice_answer, post_skip_question,
    put_question_flag,
};
use super::practice_sessions::{get_sitting_route, post_resume, post_start_over};

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
            session_scenario, sheet_rows, start_session, NewSitting,
        },
        practice_flow::{
            list_flagged, newest_open_session, open_session_count, row_statuses, session_queue_len,
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

    let read = read_deck_sources(&state, scenario_id).await?;

    let settings = state.settings.current();
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
            statuses: &read.statuses,
            open: read.open.as_ref(),
            now: chrono::Utc::now(),
        },
    );

    tracing::info!(
        slug = %slug,
        %scenario_id,
        questions = payload.questions.len(),
        points = payload.points.len(),
        answered_questions = read.statuses.len(),
        receipts = payload.receipts.len(),
        open_sessions = read.open_total,
        "served the practice deck"
    );
    Ok(Json(payload))
}

/// Everything one deck payload is read from, in one place.
///
/// Six reads, all against the pipeline pool, all fenced by the same scenario.
/// Gathered into a function so [`get_practice_deck`] stays the four steps it
/// reads as — fence the case, read the record, read the deck, say what was
/// served — rather than a straight run of six near-identical `map_err` blocks.
struct DeckRead {
    deck: Vec<crate::repositories::pipeline_repository::practice::PracticeQuestionRecord>,
    points: Vec<crate::repositories::pipeline_repository::practice::PracticePointRecord>,
    receipts: Vec<crate::repositories::pipeline_repository::practice::PracticePointReceipt>,
    last: Option<crate::repositories::pipeline_repository::practice::LastSessionRecord>,
    statuses: Vec<crate::repositories::pipeline_repository::practice_flow::RowStatusRecord>,
    open: Option<crate::repositories::pipeline_repository::practice_flow::OpenSessionRecord>,
    /// How many open sittings this scenario carries. Read, and LOGGED, before
    /// anything closes one: nothing closed an abandoned sitting before Section
    /// B, so a scenario can carry several — and an operator who only ever sees
    /// the newest has no way to discover how many there were.
    open_total: i64,
}

/// Read all six, or fail naming the read that did.
///
/// # Errors
/// 500 (logged, with the operation named) for any read that fails.
async fn read_deck_sources(state: &AppState, scenario_id: Uuid) -> Result<DeckRead, AppError> {
    Ok(DeckRead {
        deck: list_deck(&state.pipeline_pool, scenario_id)
            .await
            .map_err(|e| repo_error("list_deck", e))?,
        points: list_points(&state.pipeline_pool, scenario_id)
            .await
            .map_err(|e| repo_error("list_points", e))?,
        receipts: list_point_receipts(&state.pipeline_pool, scenario_id)
            .await
            .map_err(|e| repo_error("list_point_receipts", e))?,
        last: last_ended_session(&state.pipeline_pool, scenario_id)
            .await
            .map_err(|e| repo_error("last_ended_session", e))?,
        statuses: row_statuses(&state.pipeline_pool, scenario_id)
            .await
            .map_err(|e| repo_error("row_statuses", e))?,
        open: newest_open_session(&state.pipeline_pool, scenario_id)
            .await
            .map_err(|e| repo_error("newest_open_session", e))?,
        open_total: open_session_count(&state.pipeline_pool, scenario_id)
            .await
            .map_err(|e| repo_error("open_session_count", e))?,
    })
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

    check_sitting(&state, scenario_id, &body).await?;

    let queue = serde_json::json!(body.queue);
    let skipped_today = serde_json::json!(body.skipped_today);
    let session_id = start_session(
        &state.pipeline_pool,
        scenario_id,
        &body.who,
        &NewSitting {
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

/// Read everything Chuck's sheet is composed from, and compose it.
///
/// Split from [`post_end_session`] so that handler is the three steps it reads
/// as — find the session, close it, show the sheet — while the four reads and
/// the one comparison that BUILD the sheet sit together, where the reason each
/// is needed can be argued in one place.
///
/// # Errors
/// 404 when the scenario behind the session has gone; 500 (logged, with the
/// operation named) for any read that fails.
async fn compose_sheet(
    state: &AppState,
    session_id: Uuid,
    scenario_id: Uuid,
) -> Result<PracticeSheetPayload, AppError> {
    let record = get_scenario(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| repo_error("get_scenario", e))?
        .ok_or_else(|| AppError::NotFound {
            message: format!("scenario {scenario_id} not found"),
        })?;
    let rows = sheet_rows(&state.pipeline_pool, session_id)
        .await
        .map_err(|e| repo_error("sheet_rows", e))?;
    // The whole deck's flags, not this sitting's: a question she flagged AND
    // kept out of tonight is the one Roman most needs to see.
    let flagged = list_flagged(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| repo_error("list_flagged", e))?;

    // "Ended early" is a comparison against the queue she STARTED with, which is
    // why the queue is stored on the session. Unknown (no stored queue) is
    // reported as NOT early — the sheet never claims a fact it cannot source.
    let queue_len = session_queue_len(&state.pipeline_pool, session_id)
        .await
        .map_err(|e| repo_error("session_queue_len", e))?;
    let ended_early = queue_len.is_some_and(|n| rows.len() < n.max(0) as usize);

    let settings = state.settings.current();
    Ok(sheet_payload(
        &settings,
        &scenario_code(record.code_ordinal),
        chrono::Utc::now(),
        rows,
        ended_early,
        &flagged,
    ))
}

/// Refuse a sitting whose side is unknown or whose questions are not this
/// scenario's.
///
/// Split from the handler so that function stays the four steps it reads as
/// (fence the case, check the sitting, store it, say so) — and because these are
/// the two refusals that are about what a CLIENT sent, which is a different
/// subject from recording a sitting.
///
/// # Errors
/// 400 with the offending field and value, both times.
async fn check_sitting(
    state: &AppState,
    scenario_id: Uuid,
    body: &StartSessionRequest,
) -> Result<(), AppError> {
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

    // FENCE: every id the browser sent must belong to THIS scenario's deck.
    //
    // The queue is composed on screen, so without this a client could open a
    // sitting whose queue named another scenario's questions — and Chuck's
    // sheet would then carry a question Marie was never asked, with nothing on
    // the page looking wrong. Same reasoning as `fence_answer`, applied at the
    // moment the sitting is recorded rather than one answer at a time.
    let deck = list_deck(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| repo_error("list_deck", e))?;
    let known: HashSet<Uuid> = deck.iter().map(|q| q.id).collect();
    if let Some(stray) = fence_queue(&body.queue, &body.skipped_today, &known) {
        return Err(AppError::BadRequest {
            message: "every question in the sitting must be in this scenario's deck".to_string(),
            details: serde_json::json!({ "field": "queue", "value": stray.to_string() }),
        });
    }
    Ok(())
}

/// The first id in the sitting that this scenario's deck does not contain.
///
/// ## Why this fence exists
///
/// The queue and today's skips are both composed in the BROWSER — the order is
/// the drill, and the screen is what knows it. That makes them client input.
/// Without this check a sitting could be opened whose queue named another
/// scenario's questions, and Chuck's sheet would then carry a question Marie was
/// never asked, with nothing on the page looking wrong. Same reasoning as
/// [`super::practice_answers`]'s per-answer fence, applied once at the moment
/// the sitting is recorded.
///
/// `skipped_today` is fenced too, and deliberately: it is written to the row as
/// the record of what she was offered, so a foreign id there is a lie in the
/// record even though it deals no question.
///
/// Returns `None` when everything belongs — which is also the answer for an
/// empty sitting, because a sitting that deals nothing names nothing foreign.
pub(super) fn fence_queue<'a>(
    queue: &'a [Uuid],
    skipped_today: &'a [Uuid],
    known: &HashSet<Uuid>,
) -> Option<&'a Uuid> {
    queue
        .iter()
        .chain(skipped_today.iter())
        .find(|id| !known.contains(id))
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

    let payload = compose_sheet(&state, session_id, scenario_id).await?;

    tracing::info!(
        %session_id,
        rows = payload.rows.len(),
        flagged = payload.flagged.len(),
        "practice session ended"
    );
    Ok(Json(payload))
}

#[cfg(test)]
#[path = "practice_tests.rs"]
mod tests;
