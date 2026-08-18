//! The reads and writes mockup v3 added: the flag on a question, and what one
//! sitting was.
//!
//! A sibling of [`super::practice`] rather than more of it: that module reached
//! Rule 17's 300-line limit when these arrived, and the seam is the honest one.
//! `practice` serves the DRILL — deal the deck, record an answer, close a
//! sitting. This serves what happens AROUND one: the complaint Marie files
//! against a question, and the shape of the evening she started.
//!
//! Same idioms as its sibling, and the same caveat: `sqlx::query` takes a
//! `&str`, so nothing here is checked against the schema at compile time. The
//! disk/code guard that catches a column name this code invents lives with the
//! sibling and reads BOTH files.

use sqlx::PgPool;
use uuid::Uuid;

use super::PipelineRepoError;

/// One flagged question, as the foot of Chuck's sheet prints it.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FlaggedQuestionRecord {
    pub side: String,
    pub text: String,
    pub sort_order: i32,
    /// Never `None` — the query only returns rows that carry one.
    pub flag_note: Option<String>,
}

/// Every flagged question in this scenario's deck, in deck order.
///
/// ## Domain note: the whole DECK, not this sitting's queue
///
/// The list is headed "Flagged before the session". A question she flagged and
/// then kept out of tonight is exactly the one Roman most needs to see — so
/// filtering to what was asked would hide the complaints that mattered most.
pub async fn list_flagged(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Vec<FlaggedQuestionRecord>, PipelineRepoError> {
    sqlx::query_as::<_, FlaggedQuestionRecord>(
        "SELECT side, text, sort_order, flag_note FROM practice_questions \
         WHERE scenario_id = $1 AND flag_note IS NOT NULL ORDER BY sort_order",
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// How many questions this sitting's stored queue holds.
///
/// `None` when the session carries no queue — every session started before flow
/// v1, and any started by a build that does not write one. The caller must treat
/// that as "unknown", never as zero: a sheet that claimed `Ended early.` because
/// it could not find a queue would be inventing a fact about her evening.
pub async fn session_queue_len(
    pool: &PgPool,
    session_id: Uuid,
) -> Result<Option<i32>, PipelineRepoError> {
    let row: Option<(Option<i32>,)> =
        sqlx::query_as("SELECT jsonb_array_length(queue) FROM practice_sessions WHERE id = $1")
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
    Ok(row.and_then(|r| r.0))
}

/// Store — or clear — Marie's flag on one question.
///
/// ## Domain note: why this writes the QUESTION and not the session
///
/// Roman's ruling of 2026-08-18: a flag outlives the sitting. It is Marie
/// telling Roman and Chuck that a question is wrong, and it stands until one of
/// them changes the deck. A note scoped to an evening would be gone before
/// either of them read it.
///
/// A blank note CLEARS the flag — all three columns together, so a row can never
/// carry a `flagged_at` with nothing flagged. `who` is stored rather than
/// derived at render because the answer to "who flagged this" must survive the
/// log window.
///
/// Returns whether a row was touched, so the route can tell "stored" from "no
/// such question" rather than reporting success for a write that hit nothing.
pub async fn set_flag(
    pool: &PgPool,
    question_id: Uuid,
    note: Option<&str>,
    who: &str,
) -> Result<bool, PipelineRepoError> {
    let done = sqlx::query(
        "UPDATE practice_questions \
         SET flag_note = $2, \
             flagged_at = CASE WHEN $2::text IS NULL THEN NULL ELSE NOW() END, \
             flagged_by = CASE WHEN $2::text IS NULL THEN NULL ELSE $3 END \
         WHERE id = $1",
    )
    .bind(question_id)
    .bind(note)
    .bind(who)
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}
