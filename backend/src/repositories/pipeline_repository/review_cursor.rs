//! The review loop's read cursor — when each person last finished reviewing a deck.
//!
//! CC_TASK_REVIEW_LOOP_v1 §1–§2. One table, `practice_review_cursor`, holding one
//! row per (user, scenario), and the ONE derived count that reads it.
//!
//! ## Domain note: events and read-state are separate
//!
//! What happened on a deck is already recorded — every answer is a row in
//! `practice_answers`. This module stores nothing about those events. It stores
//! only the moment a reader last said "I have read up to here" (Slack's
//! `conversations.mark` pattern), and COUNTS the events newer than that moment at
//! read time. There is no stored counter to drift out of step with the answers,
//! and nothing to reset: pressing Done reviewing moves one timestamp, and every
//! count that depends on it follows on the next read.
//!
//! No row means everything is new. That is the honest reading: a person who has
//! never finished reviewing a deck has not reviewed any of it.
//!
//! ## CRITICAL — the pipeline pool
//!
//! `practice_review_cursor` lives in `colossus_legal_v2`: `&state.pipeline_pool`.

use sqlx::PgPool;
use uuid::Uuid;

use super::PipelineRepoError;

/// Move this user's cursor on this scenario to now. Returns the new mark.
///
/// ## Rust Learning: `INSERT … ON CONFLICT … DO UPDATE` (an upsert)
///
/// The primary key is `(user_id, scenario_id)`, so a second press would collide
/// with the first row. `ON CONFLICT (user_id, scenario_id) DO UPDATE` turns that
/// collision into an update of the existing row — ONE statement that is correct
/// whether or not a row exists, instead of a read-then-write pair that two quick
/// presses could interleave. Pressing it twice leaves one row: idempotent in
/// shape, and the mark simply moves to the later press.
///
/// `RETURNING looked_at` hands back the server's own clock, so the caller reports
/// the time the database recorded rather than a second `now()` of its own.
///
/// # Errors
/// [`PipelineRepoError`] for a failed statement — including the foreign key
/// refusing a scenario that does not exist.
pub async fn mark_reviewed(
    pool: &PgPool,
    user_id: &str,
    scenario_id: Uuid,
) -> Result<chrono::DateTime<chrono::Utc>, PipelineRepoError> {
    let row: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
        "INSERT INTO practice_review_cursor (user_id, scenario_id, looked_at) \
         VALUES ($1, $2, now()) \
         ON CONFLICT (user_id, scenario_id) DO UPDATE SET looked_at = EXCLUDED.looked_at \
         RETURNING looked_at",
    )
    .bind(user_id)
    .bind(scenario_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// How many answers are new for one viewer, per scenario.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct ViewerNewRow {
    pub scenario_id: Uuid,
    pub new_answers: i64,
}

/// Chuck's badge: answers by someone else since the viewer last finished reviewing.
///
/// A visible question counts when its CURRENT answer
/// - exists,
/// - is newer than the viewer's cursor (no cursor row = `-infinity`, so every
///   answer is newer), and
/// - was written by someone other than the viewer.
///
/// `viewer = None` (no signed-in user) is `0` for every scenario — decided in the
/// SQL by `$2::text IS NOT NULL`, so there is no Rust branch that could drift from
/// the query.
///
/// ## Domain note: `IS DISTINCT FROM`, not `<>`
///
/// Sessions from before 2026-08-19 carry `user_id = NULL`. With `<>`, `NULL <>
/// 'chuck'` is NULL — neither true nor false — and the FILTER would silently drop
/// those answers. `IS DISTINCT FROM` treats NULL as a value, so an unattributed
/// answer counts as "not you" (GO v1 ruling 1: a false "new" is honest; a silent
/// miss is not).
///
/// Starts `FROM unnest($1)` for the reason `war_room_status` gives: one row per
/// scenario asked for, zero included.
///
/// # Errors
/// [`PipelineRepoError`] for a failed statement.
pub async fn new_answers_for_viewer(
    pool: &PgPool,
    scenario_ids: &[Uuid],
    viewer: Option<&str>,
) -> Result<Vec<ViewerNewRow>, PipelineRepoError> {
    sqlx::query_as::<_, ViewerNewRow>(
        "WITH cur AS ( \
            SELECT DISTINCT ON (a.question_id) a.question_id, a.answered_at, s.user_id AS author_id \
            FROM practice_answers a JOIN practice_sessions s ON s.id = a.session_id \
            WHERE s.scenario_id = ANY($1) \
            ORDER BY a.question_id, a.answered_at DESC, a.id DESC), \
         mark AS ( \
            SELECT scenario_id, looked_at FROM practice_review_cursor \
            WHERE user_id = $2 AND scenario_id = ANY($1)) \
         SELECT ids.scenario_id, \
                COUNT(q.id) FILTER (WHERE $2::text IS NOT NULL \
                    AND cur.answered_at IS NOT NULL \
                    AND cur.answered_at > COALESCE(mark.looked_at, '-infinity'::timestamptz) \
                    AND cur.author_id IS DISTINCT FROM $2) AS new_answers \
         FROM unnest($1::uuid[]) AS ids(scenario_id) \
         LEFT JOIN mark ON mark.scenario_id = ids.scenario_id \
         LEFT JOIN practice_questions q \
                ON q.scenario_id = ids.scenario_id AND q.hidden_at IS NULL \
         LEFT JOIN cur ON cur.question_id = q.id \
         GROUP BY ids.scenario_id",
    )
    .bind(scenario_ids)
    .bind(viewer)
    .fetch_all(pool)
    .await
    .map_err(PipelineRepoError::from)
}

#[cfg(test)]
#[path = "review_cursor_live_tests.rs"]
mod live_tests;
