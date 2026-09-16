//! The War Room status card's Postgres reads — one query per fact family, for
//! every scenario of the case at once.
//!
//! CC_TASK_WAR_ROOM_v1. The dashboard renders one status card per scenario, and
//! the card reports five families of fact: the last scan, the prep (talking
//! points and watch items), the deck and how much of it is answered, and what has
//! changed on the deck since Marie last answered it. Reading those per scenario
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
//! a deck, a scan, a talking point. A scenario with none of them would simply be
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
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct LastScanRow {
    pub scenario_id: Uuid,
    /// The model's display name, resolved exactly as the scan header resolves it:
    /// the active, scan-eligible catalogue row's `display_name`, else the raw id.
    pub model_name: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub relevant: i32,
    /// `candidates_total`; a pre-background legacy run carries none, and then
    /// the count it actually judged stands in for it.
    pub total: i32,
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
        "SELECT DISTINCT ON (r.scenario_id) \
                r.scenario_id, \
                COALESCE(m.display_name, r.model_id) AS model_name, \
                r.started_at, \
                r.relevant_count AS relevant, \
                COALESCE(r.candidates_total, r.candidates_judged) AS total \
         FROM scan_runs r \
         LEFT JOIN llm_models m \
                ON m.id = r.model_id AND m.is_active = true AND m.scan_eligible = true \
         WHERE r.scenario_id = ANY($1) \
         ORDER BY r.scenario_id, r.started_at DESC, r.run_id DESC",
    )
    .bind(scenario_ids)
    .fetch_all(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// Talking points and watch items for one scenario.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct PrepCountsRow {
    pub scenario_id: Uuid,
    pub talking_points: i64,
    pub watch_items: i64,
}

/// Talking points and watch items for every scenario asked for.
///
/// `watch_list_kind` is `HumanFactKind::WatchList.code()`, passed in so this
/// module does not spell the vocabulary a second time.
///
/// ## Domain note: talking points are the OLDEST response's items
///
/// A scenario has one `scenario_responses` row by design; the read the panel uses
/// (`sole_response_for_scenario`) takes the oldest when there are more and warns.
/// The subquery picks the same row, by the same `ORDER BY created_at, id`.
pub async fn prep_counts(
    pool: &PgPool,
    scenario_ids: &[Uuid],
    watch_list_kind: &str,
) -> Result<Vec<PrepCountsRow>, PipelineRepoError> {
    sqlx::query_as::<_, PrepCountsRow>(
        "SELECT ids.scenario_id, \
                (SELECT COUNT(*) FROM response_items i \
                  WHERE i.response_id = (SELECT r.id FROM scenario_responses r \
                                          WHERE r.scenario_id = ids.scenario_id \
                                          ORDER BY r.created_at, r.id LIMIT 1) \
                ) AS talking_points, \
                (SELECT COUNT(*) FROM scenario_human_facts f \
                  WHERE f.scenario_id = ids.scenario_id AND f.kind = $2 \
                ) AS watch_items \
         FROM unnest($1::uuid[]) AS ids(scenario_id)",
    )
    .bind(scenario_ids)
    .bind(watch_list_kind)
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

/// The current answer's time for each question, over every scenario asked for.
///
/// The same `DISTINCT ON` as `practice_flow::current_answers` — every user's
/// answers, newest standing — so "answered" here and "Answered on …" on the deck
/// row are one fact. Shared by the two deck queries below.
const CURRENT_ANSWERS_CTE: &str = "cur AS ( \
        SELECT DISTINCT ON (a.question_id) a.question_id, a.answered_at \
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
                COUNT(q.id) FILTER (WHERE latest.at IS NOT NULL AND chg.at IS NOT NULL AND ( \
                    (cur.answered_at IS NOT NULL AND chg.at > cur.answered_at) OR \
                    (cur.answered_at IS NULL AND chg.at > latest.at))) AS changed \
         FROM unnest($1::uuid[]) AS ids(scenario_id) \
         LEFT JOIN latest ON latest.scenario_id = ids.scenario_id \
         LEFT JOIN practice_questions q \
                ON q.scenario_id = ids.scenario_id AND q.hidden_at IS NULL \
         LEFT JOIN cur ON cur.question_id = q.id \
         LEFT JOIN chg ON chg.question_id = q.id \
         GROUP BY ids.scenario_id"
    );
    sqlx::query_as::<_, ChangedCountRow>(&sql)
        .bind(scenario_ids)
        .fetch_all(pool)
        .await
        .map_err(PipelineRepoError::from)
}

#[cfg(test)]
#[path = "war_room_status_live_tests.rs"]
mod live_tests;
