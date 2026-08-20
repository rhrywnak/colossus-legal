//! What happens to a question hidden while a sitting still had it queued.
//!
//! ## The state nothing covered until 2026-08-19
//!
//! A sitting stores the ids it will deal (`practice_sessions.queue`) at the
//! moment it opens. Chuck can hide a question from the deck at any time after
//! that. So a live sitting can hold, in its queue, a question that no longer
//! exists as far as the deck is concerned — and when the sitting reached it,
//! .402 asked it anyway.
//!
//! None of the three existing marks describes what happened. She did not answer
//! it. She did not `skip` it — that is HER act, and this was Chuck's. It was not
//! `fine`. So the hotfix migration widened the vocabulary to a fourth value,
//! `hidden`, and these two functions are the whole of its lifecycle:
//!
//! - `hidden_in_queue` tells the sitting which of its queued ids to walk past;
//! - `record_hidden_marks` writes one row per walked-past question when the
//!   sitting ENDS, so Chuck's sheet says what happened rather than being one row
//!   short with no explanation.
//!
//! ## Why the row is written at END and not when she reaches it
//!
//! Reaching a question is a client-side observation on a GET. Writing a row for
//! it there would mean a read that mutates, and a reload that wrote twice.
//! Ending a sitting is an explicit POST that happens exactly once, and the
//! `NOT EXISTS` clause below makes a second one a no-op — so the sheet is
//! complete and the count of answers is honest, whichever way the sitting ended.

use sqlx::PgPool;
use uuid::Uuid;

use super::PipelineRepoError;

/// The queued ids that are hidden now — the ones the sitting must walk past.
///
/// ## Rust Learning: `Vec<(Uuid,)>` and the one-column tuple
///
/// `query_as` decodes a row into anything implementing `FromRow`, and sqlx
/// implements it for tuples. A ONE-column result therefore decodes into a
/// one-element tuple — `(Uuid,)`, with the trailing comma that distinguishes a
/// tuple from a parenthesised expression. `.0` takes the column out.
pub async fn hidden_in_queue(
    pool: &PgPool,
    session_id: Uuid,
) -> Result<Vec<Uuid>, PipelineRepoError> {
    // `jsonb_array_elements_text` unnests the stored queue into rows; the join
    // is what turns "an id the session remembers" into "a question that is
    // hidden today". A session with no queue produces no rows, which is the
    // right answer and not an error.
    let rows: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT q.id \
           FROM practice_sessions s \
           CROSS JOIN LATERAL jsonb_array_elements_text(s.queue) AS e(qid) \
           JOIN practice_questions q ON q.id = e.qid::uuid \
          WHERE s.id = $1 AND q.hidden_at IS NOT NULL",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

/// Write one `hidden` row per queued question that was never asked.
///
/// Returns how many rows were written, so the caller can log a number rather
/// than "did something". Zero is the normal case and is not an error.
///
/// ## Why `question_text` is copied here too
///
/// Every other answer row stores the question as it was WORDED at the time
/// (Part B), so a later re-wording cannot rewrite a printed sheet. A row written
/// here is no different: it names the question Chuck hid, in the words it had
/// when the sitting was dealt.
pub async fn record_hidden_marks(
    pool: &PgPool,
    session_id: Uuid,
) -> Result<u64, PipelineRepoError> {
    let result = sqlx::query(
        "INSERT INTO practice_answers \
             (session_id, question_id, answer_text, dont_recall, question_text, mark) \
         SELECT s.id, q.id, '', FALSE, q.text, 'hidden' \
           FROM practice_sessions s \
           CROSS JOIN LATERAL jsonb_array_elements_text(s.queue) AS e(qid) \
           JOIN practice_questions q ON q.id = e.qid::uuid \
          WHERE s.id = $1 \
            AND q.hidden_at IS NOT NULL \
            AND NOT EXISTS ( \
                  SELECT 1 FROM practice_answers a \
                   WHERE a.session_id = s.id AND a.question_id = q.id)",
    )
    .bind(session_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}
