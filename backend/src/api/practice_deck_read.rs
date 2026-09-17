//! The practice deck's reads, gathered — split from `api::practice` under
//! Rule 17 when the review loop's two reads (every note, and the viewer's
//! unreviewed count) took that file past 300 lines.
//!
//! Everything here serves ONE handler, `practice::get_practice_deck`: the
//! scenario row, the eight family reads, what changed since her last sitting, and
//! the log line that says what was served.
//!
//! ## CRITICAL — the pipeline pool
//!
//! Every table here lives in `colossus_legal_v2`: `&state.pipeline_pool`.

use uuid::Uuid;

use crate::{
    dto::practice::PracticeDeckPayload,
    error::AppError,
    repositories::pipeline_repository::{
        get_scenario,
        practice::{last_ended_session, list_deck, list_point_receipts, list_points},
        practice_editor::changes_since,
        practice_flow::{current_answers, newest_open_session, open_session_count},
        practice_notes::list_notes,
    },
    state::AppState,
};

use super::practice::repo_error;

/// The scenario row behind a deck, or a 404 naming it.
///
/// Split from [`get_practice_deck`] under Rule 18 (the review loop's reads took
/// it past fifty lines), with [`log_served`] below.
pub(super) async fn scenario_record(
    state: &AppState,
    scenario_id: Uuid,
) -> Result<crate::repositories::pipeline_repository::scenario_store::ScenarioRecord, AppError> {
    get_scenario(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| repo_error("get_scenario", format!("scenario {scenario_id}: {e}")))?
        .ok_or_else(|| AppError::NotFound {
            message: format!("scenario {scenario_id} not found"),
        })
}

/// Say what one deck read served — counts an operator can hold against the screen.
pub(super) fn log_served(
    slug: &str,
    scenario_id: Uuid,
    payload: &PracticeDeckPayload,
    answered_questions: usize,
    open_sessions: i64,
    changes_since_last: usize,
) {
    tracing::info!(
        slug = %slug,
        %scenario_id,
        questions = payload.questions.len(),
        points = payload.points.len(),
        answered_questions,
        receipts = payload.receipts.len(),
        open_sessions,
        changes_since_last,
        new_since_you_reviewed = payload.new_since_you_reviewed,
        "served the practice deck"
    );
}

/// What has happened to this scenario since her last finished sitting.
pub(super) struct WhatChanged {
    pub(super) changes:
        Vec<crate::repositories::pipeline_repository::practice_editor::DeckChangeRecord>,
}

/// Read the deck changes and count the notes that arrived since her last sitting.
///
/// ## Domain note: measured from the last ENDED session
///
/// The one she is in right now is not a sitting she has finished, and measuring
/// from it would empty the box the moment she pressed Start — which is exactly
/// when she has not yet read any of it.
pub(super) async fn read_what_changed(
    state: &AppState,
    scenario_id: Uuid,
    read: &DeckRead,
) -> Result<WhatChanged, AppError> {
    let since = read.last.as_ref().map(|s| s.ended_at);
    let changes = changes_since(&state.pipeline_pool, scenario_id, since)
        .await
        .map_err(|e| repo_error("changes_since", e))?;
    Ok(WhatChanged { changes })
}

/// Everything one deck payload is read from, in one place.
///
/// Eight reads, all against the pipeline pool, all fenced by the same scenario.
/// Gathered into a function so [`get_practice_deck`] stays the four steps it
/// reads as — fence the case, read the record, read the deck, say what was
/// served — rather than a straight run of eight near-identical `map_err` blocks.
pub(super) struct DeckRead {
    pub(super) deck:
        Vec<crate::repositories::pipeline_repository::practice::PracticeQuestionRecord>,
    pub(super) points: Vec<crate::repositories::pipeline_repository::practice::PracticePointRecord>,
    pub(super) receipts:
        Vec<crate::repositories::pipeline_repository::practice::PracticePointReceipt>,
    pub(super) last: Option<crate::repositories::pipeline_repository::practice::LastSessionRecord>,
    /// The answer that stands for each question now, for the row's `Answered on
    /// …` line. Scenario-wide, unlike `statuses` — the one-page deck row is read
    /// by two people and an answer belongs to the question, not to the reader.
    pub(super) current:
        Vec<crate::repositories::pipeline_repository::practice_flow::CurrentAnswerRecord>,
    pub(super) open:
        Option<crate::repositories::pipeline_repository::practice_flow::OpenSessionRecord>,
    /// The deck again, kept whole for the two readers that need POSITIONS in
    /// it — the change list's `Q3` and the add form's picker. `deck` itself is
    /// consumed by the payload, and cloning once here is cheaper than the two
    /// extra reads the alternative would cost.
    pub(super) deck_for_changes:
        Vec<crate::repositories::pipeline_repository::practice::PracticeQuestionRecord>,
    /// How many open sittings this scenario carries. Read, and LOGGED, before
    /// anything closes one: nothing closed an abandoned sitting before Section
    /// B, so a scenario can carry several — and an operator who only ever sees
    /// the newest has no way to discover how many there were.
    pub(super) open_total: i64,
    /// Every note on the scenario — each row shows its own (CC_TASK_REVIEW_LOOP_v1).
    pub(super) notes: Vec<crate::repositories::pipeline_repository::practice_notes::NoteRecord>,
    /// The review bar's number for THIS viewer, derived from their cursor.
    pub(super) new_since_you_reviewed: u32,
}

/// Read all eight, or fail naming the read that did.
///
/// # Errors
/// 500 (logged, with the operation named) for any read that fails.
pub(super) async fn read_deck_sources(
    state: &AppState,
    scenario_id: Uuid,
    user_id: &str,
    timezone: &str,
) -> Result<DeckRead, AppError> {
    let deck = list_deck(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| repo_error("list_deck", e))?;
    Ok(DeckRead {
        deck_for_changes: deck.clone(),
        deck,
        points: list_points(&state.pipeline_pool, scenario_id)
            .await
            .map_err(|e| repo_error("list_points", e))?,
        receipts: list_point_receipts(&state.pipeline_pool, scenario_id)
            .await
            .map_err(|e| repo_error("list_point_receipts", e))?,
        last: last_ended_session(&state.pipeline_pool, scenario_id)
            .await
            .map_err(|e| repo_error("last_ended_session", e))?,
        current: current_answers(&state.pipeline_pool, scenario_id)
            .await
            .map_err(|e| repo_error("current_answers", e))?,
        open: newest_open_session(&state.pipeline_pool, scenario_id, user_id, timezone)
            .await
            .map_err(|e| repo_error("newest_open_session", e))?,
        open_total: open_session_count(&state.pipeline_pool, scenario_id)
            .await
            .map_err(|e| repo_error("open_session_count", e))?,
        notes: list_notes(&state.pipeline_pool, scenario_id)
            .await
            .map_err(|e| repo_error("list_notes", format!("scenario {scenario_id}: {e}")))?,
        new_since_you_reviewed: super::practice_review_cursor::viewer_new_count(
            state,
            scenario_id,
            user_id,
        )
        .await?,
    })
}
