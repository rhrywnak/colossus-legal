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
use super::practice_editor::{post_edit_question, post_hide_question, post_move_question};
use super::practice_editor_add::post_add_question;
use super::practice_notes::{get_question_review, post_note, post_strike_note};
use super::practice_sessions::{get_sitting_route, post_end_session, post_resume, post_start_over};

use crate::{
    auth::AuthUser,
    domain::scenario_code::scenario_code,
    dto::practice::{PracticeDeckPayload, StartSessionRequest, StartSessionResponse},
    error::AppError,
    repositories::pipeline_repository::{
        get_scenario,
        practice::{
            last_ended_session, list_deck, list_point_receipts, list_points, start_session,
            NewSitting,
        },
        practice_editor::{changes_since, last_answered_at},
        practice_flow::{newest_open_session, open_session_count, row_statuses},
        practice_notes::list_notes,
    },
    services::{
        practice_changes::{badged, changed_box},
        practice_editor_options::attach_options,
        practice_notes::{new_since, scenario_notes},
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
        .route(
            "/practice/questions/:question_id/hidden",
            post(post_hide_question),
        )
        .route(
            "/cases/:slug/scenarios/:scenario_id/practice/questions",
            post(post_add_question),
        )
        .route(
            "/cases/:slug/scenarios/:scenario_id/practice/questions/:question_id",
            get(get_question_review),
        )
        .route(
            "/cases/:slug/scenarios/:scenario_id/practice/notes",
            post(post_note),
        )
        .route("/practice/notes/:note_id/strike", post(post_strike_note))
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
            statuses: &read.statuses,
            open: read.open.as_ref(),
            now: chrono::Utc::now(),
            badged: &badged(&news.changes, &read.answered_at),
            notes: scenario_notes(&settings, &read.notes),
            changed: changed_box(
                &settings,
                &read.deck_for_changes,
                &news.changes,
                news.fresh_notes,
                news.newest_note_author.as_deref(),
            ),
            attach_options: attach,
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
        changes_since_last = news.changes.len(),
        notes = payload.notes.len(),
        "served the practice deck"
    );
    Ok(Json(payload))
}

/// What has happened to this scenario since her last finished sitting.
struct WhatChanged {
    changes: Vec<crate::repositories::pipeline_repository::practice_editor::DeckChangeRecord>,
    fresh_notes: usize,
    newest_note_author: Option<String>,
}

/// Read the deck changes and count the notes that arrived since her last sitting.
///
/// ## Domain note: measured from the last ENDED session
///
/// The one she is in right now is not a sitting she has finished, and measuring
/// from it would empty the box the moment she pressed Start — which is exactly
/// when she has not yet read any of it.
async fn read_what_changed(
    state: &AppState,
    scenario_id: Uuid,
    read: &DeckRead,
) -> Result<WhatChanged, AppError> {
    let since = read.last.as_ref().map(|s| s.ended_at);
    let changes = changes_since(&state.pipeline_pool, scenario_id, since)
        .await
        .map_err(|e| repo_error("changes_since", e))?;
    let (fresh_notes, newest_note_author) = new_since(&read.notes, since);
    Ok(WhatChanged {
        changes,
        fresh_notes,
        newest_note_author: newest_note_author.map(str::to_string),
    })
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
    /// The deck again, kept whole for the two readers that need POSITIONS in
    /// it — the change list's `Q3` and the add form's picker. `deck` itself is
    /// consumed by the payload, and cloning once here is cheaper than the two
    /// extra reads the alternative would cost.
    deck_for_changes:
        Vec<crate::repositories::pipeline_repository::practice::PracticeQuestionRecord>,
    /// Every note on the scenario, all three levels. Partitioned by the caller.
    notes: Vec<crate::repositories::pipeline_repository::practice_notes::NoteRecord>,
    /// When each question was last answered, for the `changed` badge.
    answered_at: Vec<(Uuid, chrono::DateTime<chrono::Utc>)>,
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
    let deck = list_deck(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| repo_error("list_deck", e))?;
    Ok(DeckRead {
        deck_for_changes: deck.clone(),
        deck,
        notes: list_notes(&state.pipeline_pool, scenario_id)
            .await
            .map_err(|e| repo_error("list_notes", e))?,
        answered_at: last_answered_at(&state.pipeline_pool, scenario_id)
            .await
            .map_err(|e| repo_error("last_answered_at", e))?,
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
#[cfg(test)]
#[path = "practice_tests.rs"]
mod tests;
