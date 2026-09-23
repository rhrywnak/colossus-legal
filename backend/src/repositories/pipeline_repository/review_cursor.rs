//! The review loop's read cursor — when each person last finished reviewing a deck.
//!
//! CC_TASK_REVIEW_LOOP_v1 §1–§2. One table, `practice_review_cursor`, holding one
//! row per (user, scenario), and the ONE derived queue that reads it — against
//! the REVIEWER's row only, since CC_TASK_SIMPLE_COUNTS_v1.
//!
//! ## Domain note: events and read-state are separate
//!
//! What happened on a deck is already recorded — every answer is a row in
//! `practice_answers`, every note a row in `practice_notes`. This module stores
//! nothing about those events. It stores
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

/// The review queue for one scenario: how many answers wait, and since when.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct AwaitingReviewRow {
    pub scenario_id: Uuid,
    /// Answers the reviewer has not reviewed.
    pub awaiting: i64,
    /// When the OLDEST of those was written; `None` when nothing waits.
    pub oldest: Option<chrono::DateTime<chrono::Utc>>,
}

/// The reviewers' queue: answers by someone else since the BENCH last pressed
/// Done reviewing (CC_TASK_SIMPLE_COUNTS_v1; a bench since CC_TASK_REVIEW_PAGE_v1).
///
/// A visible question counts when EITHER of two things is true of it.
///
/// Its CURRENT answer
/// - exists,
/// - is newer than the SHARED cursor (no cursor row = `-infinity`, so every
///   answer is newer), and
/// - was written by someone who is not on the reviewer bench.
///
/// Or a NOTE on it (or on that current answer)
/// - still stands — not struck,
/// - was written by someone who is not on the bench, and
/// - is newer than the shared cursor.
///
/// The second leg arrived with CC_TASK_DEFECT_SWEEP_v1 (defect 5): until then
/// the queue counted answers only, so a note Marie left on a question Chuck had
/// already reviewed reached nobody. See the `NOTE_WAITING` predicate below.
///
/// ## Domain note: one queue, the same for every viewer
///
/// Never the signed-in user. Until v2.1.10 this counted against the VIEWER,
/// which gave three people three different truths about one deck.
///
/// ## Domain note: ONE SHARED CURSOR over EVERY PRESS (ruled 2026-09-22, R1)
///
/// The `mark` CTE takes `MAX(looked_at)` across every cursor row on the
/// scenario — it no longer filters by the shown list. "Any permitted press
/// clears the count for everyone" is the rule, and WHO IS PERMITTED is decided
/// once, at the route (`services::review_permission::may_review`, enforced by
/// `api::practice_review_cursor::put_review_cursor` with a 403). Every row in
/// this table was therefore written by somebody allowed to write it.
///
/// It had to change: permission became "listed OR administrator" when Roman took
/// himself off the war room's display list to stop it naming him and lost the
/// button with it. An admin's press wrote a row this CTE did not read, so the
/// queue never cleared and his review was silently ignored — the exact failure
/// the task forbade.
///
/// ## Domain note: the EXCLUSION legs still read the shown list (R2)
///
/// `$1` below is still the `practice_reviewer_usernames` row, and it still
/// decides whose work does not wait. SQL cannot see Authentik groups, so an
/// unlisted administrator's own answer or note DOES wait until a Done is
/// pressed — including by himself, which he may always do. Ruled 2026-09-22:
/// simple and never stale, over a query that tries to guess at group membership.
/// An answer or a note written by any LISTED reviewer no longer waits (2026-09-19):
/// left as one name, the second reviewer's own note counted as work waiting for
/// the second reviewer.
///
/// ## Domain note: `IS DISTINCT FROM`, not `<>`
///
/// Sessions from before 2026-08-19 carry `user_id = NULL`. With `<>`, `NULL <>
/// 'cpenzien'` is NULL — neither true nor false — and the FILTER would silently
/// drop those answers. `IS DISTINCT FROM` treats NULL as a value, so an
/// unattributed answer counts as "not the reviewer".
///
/// ## SQL note: the date rides the same pass
///
/// `MIN(…) FILTER (…)` uses the SAME predicate as the count, so the summary card's
/// "oldest waiting since" can never name an event the count excluded — and it
/// costs no second query. What it minimises is the moment that MADE the question
/// wait, which is the answer's stamp, the note's, or the earlier of the two when
/// both legs fire.
///
/// Starts `FROM unnest($1)` for the reason `war_room_status` gives: one row per
/// scenario asked for, zero included.
///
/// # Errors
/// [`PipelineRepoError`] for a failed statement.
pub async fn awaiting_review(
    pool: &PgPool,
    scenario_ids: &[Uuid],
    reviewers: &[String],
) -> Result<Vec<AwaitingReviewRow>, PipelineRepoError> {
    let waiting = format!("(({ANSWER_WAITING}) OR ({NOTE_WAITING}))");
    // The moment that made this question wait. `LEAST` ignores NULLs in
    // Postgres, so a question waiting on only one of the two legs yields that
    // leg's own timestamp, and one waiting on both yields the earlier — which is
    // what "waiting since" means to somebody reading the summary card.
    let waiting_since = format!(
        "LEAST(CASE WHEN {ANSWER_WAITING} THEN cur.answered_at END, \
               CASE WHEN {NOTE_WAITING} THEN note.at END)"
    );
    sqlx::query_as::<_, AwaitingReviewRow>(&format!(
        "WITH cur AS ( \
            SELECT DISTINCT ON (a.question_id) a.question_id, a.id AS answer_id, \
                   a.answered_at, s.user_id AS author_id \
            FROM practice_answers a JOIN practice_sessions s ON s.id = a.session_id \
            WHERE s.scenario_id = ANY($1) \
            ORDER BY a.question_id, a.answered_at DESC, a.id DESC), \
         mark AS ( \
            SELECT scenario_id, MAX(looked_at) AS looked_at \
            FROM practice_review_cursor \
            WHERE scenario_id = ANY($1) \
            GROUP BY scenario_id) \
         SELECT ids.scenario_id, \
                COUNT(q.id) FILTER (WHERE {waiting}) AS awaiting, \
                MIN({waiting_since}) FILTER (WHERE {waiting}) AS oldest \
         FROM unnest($1::uuid[]) AS ids(scenario_id) \
         LEFT JOIN mark ON mark.scenario_id = ids.scenario_id \
         LEFT JOIN practice_questions q \
                ON q.scenario_id = ids.scenario_id AND q.hidden_at IS NULL \
         LEFT JOIN cur ON cur.question_id = q.id \
         LEFT JOIN LATERAL ( \
            SELECT MAX(n.created_at) AS at FROM practice_notes n \
             WHERE n.question_id = q.id \
               AND n.struck_at IS NULL \
               AND (n.author_id IS NULL OR NOT (n.author_id = ANY($2))) \
               AND (n.answer_id IS NULL OR n.answer_id = cur.answer_id)) note ON true \
         GROUP BY ids.scenario_id"
    ))
    .bind(scenario_ids)
    // ## Rust Learning: binding a slice as a Postgres array
    //
    // sqlx encodes `&[String]` as `text[]`, which is what `= ANY($2)` reads. One
    // bind, three predicate sites, and no SQL built by string concatenation —
    // the alternative (an `IN` list assembled in Rust) would put user-supplied
    // logins into the statement text.
    .bind(reviewers)
    .fetch_all(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// An ANSWER awaiting review — see [`awaiting_review`]'s SQL note.
// STRUCTURAL: this IS the definition of an answer awaiting review — it exists,
// it post-dates the SHARED cursor, and nobody on the reviewer bench wrote it.
// Not a threshold or a limit: changing it changes what the review queue MEANS,
// which is a ruling and a code change, never a deployment value. Shared so the
// count and the oldest date cannot disagree.
//
// ⚑ The bench leg is written out rather than spelled `<> ALL($2)`, for the
// reason the old `IS DISTINCT FROM` was: sessions from before 2026-08-19 carry
// `author_id = NULL`, and every three-valued comparison against NULL — `<> ALL`
// included — yields NULL, which the FILTER drops. An unattributed answer must
// count as "not a reviewer", so the NULL case is named.
const ANSWER_WAITING: &str = "cur.answered_at IS NOT NULL \
    AND cur.answered_at > COALESCE(mark.looked_at, '-infinity'::timestamptz) \
    AND (cur.author_id IS NULL OR NOT (cur.author_id = ANY($2)))";

/// A NOTE awaiting review (CC_TASK_DEFECT_SWEEP_v1 defect 5).
///
/// ## Domain note: the other half of a loop that only ran one way
///
/// Marie's "new or changed" count has folded in Chuck's unstruck notes since
/// Part B (`war_room_status`, the same LATERAL shape) — so Chuck→Marie worked
/// and Marie→Chuck did not. A note she leaves on a question he has already
/// reviewed reached nobody: his queue counted answers only, and the note sat
/// there until somebody happened to open that row.
///
/// The three legs are the bench's own, one for one: the note still STANDS
/// (striking it withdraws it, exactly as it does on her side), somebody OTHER
/// than a listed reviewer wrote it, and it post-dates the shared cursor.
/// `author_id` rather than `author` because the cursor and the sessions key on
/// the login; the NULL case is named explicitly because rows written before
/// 2026-08-19 carry no author id at all, and a NULL there means "not a
/// reviewer", not "unknown, skip it".
// STRUCTURAL: this IS the definition of a note awaiting review. Same standing as
// ANSWER_WAITING above, and changed only by a ruling.
const NOTE_WAITING: &str = "note.at IS NOT NULL \
    AND note.at > COALESCE(mark.looked_at, '-infinity'::timestamptz)";

#[cfg(test)]
#[path = "review_cursor_live_tests.rs"]
mod live_tests;

#[cfg(test)]
#[path = "review_cursor_notes_live_tests.rs"]
mod notes_live_tests;
