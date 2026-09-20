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

/// The amber pill's number, for every scenario asked for (CC_GO_WAR_ROOM_v3).
///
/// ## Domain note: per question, and no sittings
///
/// A visible question counts when EITHER
/// - it has an answer and its newest change is later than that answer — she
///   answered words that have since changed; or
/// - it has no answer and its newest change is later than the newest answer
///   anywhere in the scenario — it arrived or moved after she last worked here.
///
/// "Its newest change" includes NOTES (CC_TASK_REVIEW_LOOP_v1 §3): an UNSTRUCK
/// note on the question, or on its CURRENT answer, is a change she has not read.
/// `GREATEST(chg.at, nt.at)` is the newer of the two — Postgres's `GREATEST`
/// ignores NULLs, so a question with only a note, or only a change, still has a
/// signal. A note on a superseded attempt never counts (GO v1 ruling 6), and a
/// struck note never counts — withdrawing it withdraws the work it asked for.
///
/// ## SQL note: `LEFT JOIN LATERAL`
///
/// An ordinary join's subquery cannot see the other tables in the FROM list.
/// `LATERAL` lets it — so the note subquery can read THIS row's `q.id` and
/// `cur.answer_id`, and "a note on the current answer" is expressible at all.
/// `LEFT … ON true` keeps a question with no notes (its `MAX` is NULL).
///
/// ## Domain note: her OWN notes do not badge her (ruled 2026-09-20)
///
/// `witness` is the `practice_witness_username` settings row, and a note that
/// login wrote never reaches this count. Until v2.1.15 it did: Marie wrote
/// Chuck a note on 19 September and her own tile came back reading "1 new or
/// changed for Marie", because this LATERAL had no author leg at all.
///
/// It filters by the count's OWNER, never by the signed-in viewer. The number
/// is one global fact by ruling (2026-09-17) and this build has no
/// is-the-reader-the-witness concept, so a viewer filter would give three
/// people three different truths about one deck. A message she wrote to
/// somebody else is not work waiting on her, whoever is looking at the tile.
///
/// The NULL case is named for the reason `review_cursor` names it: notes from
/// before 2026-08-19 carry `author_id = NULL`, `NULL <> 'docmarie'` is NULL,
/// and a NULL here drops the note from a `WHERE` — so an unattributed note
/// would silently stop counting. NULL means "not the witness", not "skip it".
///
/// Question EDITS are deliberately untouched. `chg` reads
/// `practice_deck_changes` with no author leg, so a question SHE moved or
/// reworded still badges her — the badge asks her to re-read a question whose
/// words have changed, and it is no less true when she changed them.
///
/// Chuck's queue is a different query in a different module
/// (`review_cursor::awaiting_review`), and it is untouched: her notes must keep
/// counting as work awaiting his review, which is the loop this closes the
/// other half of.
///
/// And nothing counts in a scenario with no answer at all (`latest IS NULL`): a
/// deck she has never opened is not "changed", however many `added` rows its
/// seeding wrote. `practice_sessions` is joined only to learn which scenario an
/// answer belongs to, exactly as `current_answers` does — never for `ended_at`,
/// because nothing in her one-page flow ever ends a sitting.
///
/// It resets itself one question at a time by the act she already performs:
/// answering.
pub async fn changed_counts(
    pool: &PgPool,
    scenario_ids: &[Uuid],
    witness: &str,
) -> Result<Vec<ChangedCountRow>, PipelineRepoError> {
    let sql = format!(
        "WITH {CURRENT_ANSWERS_CTE}, \
         latest AS ( \
            SELECT s.scenario_id, MAX(a.answered_at) AS at \
            FROM practice_answers a JOIN practice_sessions s ON s.id = a.session_id \
            WHERE s.scenario_id = ANY($1) GROUP BY s.scenario_id), \
         chg AS ( \
            SELECT question_id, MAX(changed_at) AS at FROM practice_deck_changes \
            WHERE scenario_id = ANY($1) GROUP BY question_id) \
         SELECT ids.scenario_id, \
                COUNT(q.id) FILTER (WHERE latest.at IS NOT NULL \
                    AND GREATEST(chg.at, nt.at) IS NOT NULL AND ( \
                    (cur.answered_at IS NOT NULL AND GREATEST(chg.at, nt.at) > cur.answered_at) OR \
                    (cur.answered_at IS NULL AND GREATEST(chg.at, nt.at) > latest.at))) AS changed \
         FROM unnest($1::uuid[]) AS ids(scenario_id) \
         LEFT JOIN latest ON latest.scenario_id = ids.scenario_id \
         LEFT JOIN practice_questions q \
                ON q.scenario_id = ids.scenario_id AND q.hidden_at IS NULL \
         LEFT JOIN cur ON cur.question_id = q.id \
         LEFT JOIN chg ON chg.question_id = q.id \
         LEFT JOIN LATERAL ( \
            SELECT MAX(n.created_at) AS at FROM practice_notes n \
            WHERE n.question_id = q.id AND n.struck_at IS NULL \
              AND (n.author_id IS NULL OR n.author_id <> $2) \
              AND (n.answer_id IS NULL OR n.answer_id = cur.answer_id)) nt ON true \
         GROUP BY ids.scenario_id"
    );
    sqlx::query_as::<_, ChangedCountRow>(&sql)
        .bind(scenario_ids)
        .bind(witness)
        .fetch_all(pool)
        .await
        .map_err(PipelineRepoError::from)
}

// `pub(crate)`: the review-loop proofs (`review_cursor`, and the notes file
// below) build their scenarios, questions and answers with these same helpers.
#[cfg(test)]
#[path = "war_room_status_live_tests.rs"]
pub(crate) mod live_tests;

#[cfg(test)]
#[path = "war_room_notes_live_tests.rs"]
mod notes_live_tests;
