//! The War Room status card's Postgres reads — one query per fact family, for
//! every scenario of the case at once.
//!
//! CC_TASK_WAR_ROOM_v1. The dashboard renders one status card per scenario, and
//! the card reports three families of fact here: the last scan, the deck and how
//! much of it is answered, and what has changed on the deck since Marie last
//! answered it. (Talking points and watch items left the card with
//! CC_TASK_REVIEW_LOOP_v1; the viewer's own count lives in `review_cursor`.) Reading those per scenario
//! would be eleven round trips per family on every dashboard load; each function
//! here is ONE statement over all the case's scenario ids.
//!
//! The two evidence counts that are NOT here — candidates to rule and Matrix
//! linked — are not stored anywhere. They are computed from the graph and the
//! scan projection by the card queue's own read, and the dashboard reuses that
//! read (`services::war_room_evidence`) rather than re-deriving them in SQL.
//!
//! ## Rust Learning: `unnest($1::uuid[])` and why every query starts from it
//!
//! `WHERE scenario_id = ANY($1)` would return rows only for scenarios that HAVE
//! a deck, a scan, an answer. A scenario with none of them would simply be
//! missing from the result, and the caller would have to decide whether "absent"
//! meant "zero" or "the query forgot it" — the exact collapse Standing Rule 1
//! forbids. Starting FROM `unnest($1)` produces one row per id asked for, and the
//! `LEFT JOIN`s turn "nothing there" into an honest `0`. sqlx binds a Rust
//! `&[Uuid]` to a Postgres `uuid[]` directly, which is what makes the one bind
//! carry every scenario.

use sqlx::PgPool;
use uuid::Uuid;

use super::waiting_items::{waiting_counts, WaitingQuery, WaitingScope, WaitingSide};
use super::PipelineRepoError;

// STRUCTURAL: the `practice_questions.side` CHECK vocabulary
// (`side IN ('george', 'chuck')`, migration 20260817213319). Not a setting — a
// new side is a migration plus a code change, as that migration's header says.
// Chuck's side is named; the defense's is every other visible question, so a
// third side added to the CHECK would be counted as the defense's rather than
// silently dropped from both.
const SIDE_CHUCK: &str = "chuck";

/// The scenario's most recent scan, or `None` when it has never been scanned.
///
/// Only the day now: the card's scan line is date-only, and the model and the
/// relevant counts live on the scenario page (CC_TASK_REVIEW_LOOP_v1 §5).
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct LastScanRow {
    pub scenario_id: Uuid,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

/// The newest scan of each scenario that has one.
///
/// One row per SCANNED scenario — deliberately not `unnest`-first: a scenario
/// with no run is `None` on the card ("Scan: never run"), not a row of zeroes,
/// and the absence is the fact. The caller turns a missing id into `None`.
///
/// ## Why any status, and newest by `started_at`
///
/// It mirrors `GET …/scan-runs`, whose `runs[0]` is the scan header's "last scan"
/// line: newest first, whatever became of it.
pub async fn last_scans(
    pool: &PgPool,
    scenario_ids: &[Uuid],
) -> Result<Vec<LastScanRow>, PipelineRepoError> {
    sqlx::query_as::<_, LastScanRow>(
        "SELECT DISTINCT ON (r.scenario_id) r.scenario_id, r.started_at \
         FROM scan_runs r \
         WHERE r.scenario_id = ANY($1) \
         ORDER BY r.scenario_id, r.started_at DESC, r.run_id DESC",
    )
    .bind(scenario_ids)
    .fetch_all(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// The deck and how much of it is answered, for one scenario.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct DeckCountsRow {
    pub scenario_id: Uuid,
    pub questions: i64,
    /// The day the deck last changed — `max(updated_at)` over the whole deck, the
    /// practice page's `deck_as_of` rule. `None` for a scenario with no deck.
    pub built_on: Option<chrono::DateTime<chrono::Utc>>,
    pub answered: i64,
    pub chuck_answered: i64,
    pub chuck_total: i64,
    pub defense_answered: i64,
    pub defense_total: i64,
}

/// The current answer's id and time for each question, over every scenario asked for.
///
/// The same `DISTINCT ON` as `practice_flow::current_answers` — every user's
/// answers, newest standing — so "answered" here and "Answered on …" on the deck
/// row are one fact. Shared by the two deck queries below; the id is what lets a
/// note on the CURRENT answer be told from a note on a superseded one.
const CURRENT_ANSWERS_CTE: &str = "cur AS ( \
        SELECT DISTINCT ON (a.question_id) a.question_id, a.id AS answer_id, a.answered_at \
        FROM practice_answers a JOIN practice_sessions s ON s.id = a.session_id \
        WHERE s.scenario_id = ANY($1) \
        ORDER BY a.question_id, a.answered_at DESC, a.id DESC)";

/// Deck size, deck date and the answered split, for every scenario asked for.
///
/// Hidden questions (`hidden_at IS NOT NULL`) are excluded from every COUNT.
/// `built_on` is not a count; it follows `deck_as_of`, which reads the whole deck.
pub async fn deck_counts(
    pool: &PgPool,
    scenario_ids: &[Uuid],
) -> Result<Vec<DeckCountsRow>, PipelineRepoError> {
    let sql = format!(
        "WITH {CURRENT_ANSWERS_CTE} \
         SELECT ids.scenario_id, \
                COUNT(q.id) FILTER (WHERE q.hidden_at IS NULL) AS questions, \
                MAX(q.updated_at) AS built_on, \
                COUNT(q.id) FILTER (WHERE q.hidden_at IS NULL AND cur.answered_at IS NOT NULL) \
                    AS answered, \
                COUNT(q.id) FILTER (WHERE q.hidden_at IS NULL AND q.side = $2 \
                                      AND cur.answered_at IS NOT NULL) AS chuck_answered, \
                COUNT(q.id) FILTER (WHERE q.hidden_at IS NULL AND q.side = $2) AS chuck_total, \
                COUNT(q.id) FILTER (WHERE q.hidden_at IS NULL AND q.side <> $2 \
                                      AND cur.answered_at IS NOT NULL) AS defense_answered, \
                COUNT(q.id) FILTER (WHERE q.hidden_at IS NULL AND q.side <> $2) AS defense_total \
         FROM unnest($1::uuid[]) AS ids(scenario_id) \
         LEFT JOIN practice_questions q ON q.scenario_id = ids.scenario_id \
         LEFT JOIN cur ON cur.question_id = q.id \
         GROUP BY ids.scenario_id"
    );
    sqlx::query_as::<_, DeckCountsRow>(&sql)
        .bind(scenario_ids)
        .bind(SIDE_CHUCK)
        .fetch_all(pool)
        .await
        .map_err(PipelineRepoError::from)
}

/// How many visible questions are new or changed for Marie, per scenario.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct ChangedCountRow {
    pub scenario_id: Uuid,
    pub changed: i64,
}

/// The amber pill's number, for every scenario asked for — now DERIVED from the
/// per-item seen record (CC_TASK_FOR_YOU_v1 L2).
///
/// ## What it used to be, and what replaced it
///
/// A question counted when its newest change or unstruck note was later than
/// the witness's own last answer — her answering was the read mark, and nothing
/// was stored. That could say a deck held something new and never WHICH
/// question, and it could not be cleared by reading: only by answering again.
///
/// It is now the same predicate the For You page reads, asked per deck about
/// HER: the items on the witness's side (a listed reviewer's note, a reworded
/// question) that she has not seen. One query behind the tile, the page and the
/// deck bar, so a count can never send her to a list that shows her nothing.
///
/// ## Domain note: the OWNER, not the viewer
///
/// `witness` is the `practice_witness_username` row, and it is both whose side
/// is counted and whose seen record clears it. Whoever is LOOKING at the war
/// room does not enter into it — the tile reports her backlog to anybody,
/// which is the ruling of 2026-09-17 carried forward unchanged.
///
/// `reviewers` is the stored `practice_reviewer_usernames` list. It decides
/// which notes are a REVIEWER's and therefore hers to read (ruling R2); it
/// never decides who may see this number.
///
/// ## What changed for a reader, stated plainly
///
/// Her own notes still never badge her — an item never waits for its own
/// author. A question SHE reworded no longer badges her either, which it used
/// to: the ruling of 2026-09-22 (Q3) excludes an item's author on every kind,
/// and "Marie changed this — waiting for Marie" was never a sentence worth
/// printing. And a change that touched something other than the question's
/// TEXT — Chuck's `stronger` or `tactic` notes on it — no longer counts at all.
///
pub async fn changed_counts(
    pool: &PgPool,
    scenario_ids: &[Uuid],
    witness: &str,
    reviewers: &[String],
) -> Result<Vec<ChangedCountRow>, PipelineRepoError> {
    let rows = waiting_counts(
        pool,
        &WaitingQuery {
            scenario_ids,
            // The count's OWNER is also the person it is counted FOR: this
            // number is "new or changed for Marie", and the seen record that
            // clears it is hers. Whoever is LOOKING at the war room does not
            // enter into it — the tile reports her backlog to anybody.
            viewer: witness,
            reviewers,
            side: WaitingSide::Witness,
            scope: WaitingScope::Unseen,
        },
    )
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| ChangedCountRow {
            scenario_id: row.scenario_id,
            changed: row.waiting,
        })
        .collect())
}

// `pub(crate)`: the review-loop proofs (`review_cursor`, and the notes file
// below) build their scenarios, questions and answers with these same helpers.
#[cfg(test)]
#[path = "war_room_status_live_tests.rs"]
pub(crate) mod live_tests;

#[cfg(test)]
#[path = "war_room_notes_live_tests.rs"]
mod notes_live_tests;
