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

use crate::domain::settings::Settings;
use crate::repositories::pipeline_repository::waiting_items::{
    waiting_items, WaitingItemRow, WaitingQuery, WaitingScope, WaitingSide,
};
use crate::services::for_you::{query_side, side_for};
use crate::services::war_room_progress::{practice_witness, review_queue_reviewer};

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

/// What this deck holds UNREAD for the person asking (CC_TASK_FOR_YOU_v1 L3).
///
/// ## Domain note: the SAME predicate as the For you page
///
/// Not a second query about a similar question: `waiting_items`, narrowed to
/// one scenario, on the side `may_review` puts this reader on. So a mark on a
/// question's row and a row on the For you list are two readings of one fact,
/// and the deck cannot say something waits that the list does not show.
///
/// A reader who is neither a reviewer nor the witness has no side, so no query
/// runs and every row is unmarked — the same silence the list gives them.
///
/// # Errors
/// 500 (logged, naming the read) when the statement fails.
pub(super) async fn deck_waiting(
    state: &AppState,
    scenario_id: Uuid,
    user_id: &str,
    is_admin: bool,
) -> Result<Vec<WaitingItemRow>, AppError> {
    let settings = state.settings.current();
    let reviewers = review_queue_reviewer(&settings);
    let Some(side) = deck_side(&settings, user_id, is_admin) else {
        return Ok(Vec::new());
    };
    waiting_items(
        &state.pipeline_pool,
        &WaitingQuery {
            scenario_ids: &[scenario_id],
            viewer: user_id,
            reviewers,
            side,
            scope: WaitingScope::Unseen,
        },
        // No LIMIT: a mark missing from one row because a cap cut the list is
        // the silent failure this whole task exists to remove.
        None,
    )
    .await
    .map_err(|e| {
        repo_error(
            "waiting_items",
            format!("deck {scenario_id} for {user_id}: {e}"),
        )
    })
}

/// Which side's unread list this deck should be read on, or `None` for nobody.
///
/// Split out of [`deck_waiting`] so the decision can be tested without an
/// `AppState`: the branch that matters is the SHORT-CIRCUIT — a person who is
/// neither a reviewer nor the witness must cost no query at all, and a
/// regression there is a statement per deck row per page load for somebody
/// whose marks would all be empty anyway.
pub(super) fn deck_side(settings: &Settings, user_id: &str, is_admin: bool) -> Option<WaitingSide> {
    query_side(side_for(
        user_id,
        is_admin,
        review_queue_reviewer(settings),
        practice_witness(settings),
    ))
}

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
        awaiting_review = payload.review.awaiting,
        can_mark_reviewed = payload.review.can_mark_reviewed,
        // How many rows went out wearing a board-4 mark (CC_TASK_FOR_YOU_v1 L3).
        // Counted here so an operator can hold the number against the screen —
        // and so a `deck_waiting` that quietly returned nothing is visible as a
        // zero rather than as a deck that happens to look calm.
        waiting_marks = payload
            .questions
            .iter()
            .filter(|q| q.waiting.is_some())
            .count(),
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
    /// What is UNREAD on this deck for the person asking, on their own side —
    /// the board-4 mark's source (CC_TASK_FOR_YOU_v1 L3). Empty for somebody
    /// who is neither a reviewer nor the witness: no side, so nothing waits.
    pub(super) waiting: Vec<WaitingItemRow>,
    /// The review bar: the reviewer's backlog, and whether THIS user may clear it.
    pub(super) review: crate::dto::practice_review::DeckReviewDto,
}

/// Read all eight, or fail naming the read that did.
///
/// # Errors
/// 500 (logged, with the operation named) for any read that fails.
/// `is_admin` rides with `user_id` because the review bar asks BOTH: the button
/// is offered to a listed reviewer or to an administrator
/// (`services::review_permission`), and only the route has the caller's groups.
pub(super) async fn read_deck_sources(
    state: &AppState,
    scenario_id: Uuid,
    user_id: &str,
    is_admin: bool,
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
        waiting: deck_waiting(state, scenario_id, user_id, is_admin).await?,
        review: super::practice_review_cursor::deck_review(state, scenario_id, user_id, is_admin)
            .await?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::pipeline_repository::waiting_items::WaitingSide;

    /// The three readers of one deck, and the one that costs no query.
    ///
    /// `Settings::for_test()` carries the seeded bench (`cpenzien`) and witness
    /// (`docmarie`), so these assert against the values the migration ships.
    #[test]
    fn a_deck_is_read_on_the_side_its_reader_belongs_to() {
        let settings = Settings::for_test();
        assert_eq!(
            deck_side(&settings, "cpenzien", false),
            Some(WaitingSide::Reviewers),
            "a listed reviewer reads the reviewers' side"
        );
        assert_eq!(
            deck_side(&settings, "docmarie", false),
            Some(WaitingSide::Witness),
            "the witness reads her own"
        );
    }

    /// (M) An UNLISTED ADMINISTRATOR still reads the reviewers' side.
    ///
    /// The same rule the page itself obeys (`may_review` is permission, the
    /// stored list is display). If this asked the list instead, Roman would see
    /// no marks on any deck.
    #[test]
    fn an_unlisted_administrator_reads_the_reviewers_side() {
        assert_eq!(
            deck_side(&Settings::for_test(), "roman", true),
            Some(WaitingSide::Reviewers)
        );
    }

    /// (M) Somebody who is neither costs NO query — the short-circuit.
    ///
    /// `None` here is what makes `deck_waiting` return an empty list without
    /// touching the database. A regression to `Some(...)` would be one extra
    /// statement per deck load, to compute marks that are all empty by
    /// construction: the audience clauses exclude a stranger from both sides.
    #[test]
    fn a_stranger_costs_no_query() {
        assert_eq!(deck_side(&Settings::for_test(), "nobody", false), None);
    }
}
