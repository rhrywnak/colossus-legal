//! Writing one answer: opening the row, attaching its read, and settling it.
//!
//! Split from [`super::practice`] in T1 (2026-08-20), when that module reached
//! the 300-line limit (Rule 17) and this task needed to add a second write to it.
//! The seam is the one the table already draws: the sibling module READS the deck,
//! the points, the receipts and the sessions, and this one WRITES the log Chuck's
//! sheet is rendered from.
//!
//! ## Why one answer is now written in THREE steps
//!
//! It was two, and the reason for the second is unchanged: the read has to exist
//! before Marie can react to it, and her four boxes and her mark are both decided
//! afterwards. T1 adds the first:
//!
//! 1. [`insert_answer`] — her typed answer, the moment worth surviving a closed
//!    laptop. **Committed before the model is called.**
//! 2. [`attach_read`] — the read, when it arrives.
//! 3. [`close_answer`] — her mark and her four boxes, when she leaves the reveal.
//!
//! ## Domain note: why the answer is committed BEFORE the read is requested
//!
//! It used to be the reverse — the model was called, and her answer existed
//! nowhere until the call returned. Design §4 rule 4 inverts it, and the reason
//! is not hypothetical: the browser gives up after 90 seconds and the server
//! after 600, so a slow call could tell Marie her answer was not recorded while
//! the row was written anyway. She would answer again, and the log would show two
//! answers where she gave one. A tool whose whole claim is "your answers are
//! kept" cannot tell her a truth-shaped lie.
//!
//! The 90-versus-600 reconciliation itself is T2's, explicitly. This is the half
//! that makes her answer safe regardless of which number wins.

use sqlx::PgPool;
use uuid::Uuid;

use super::PipelineRepoError;

/// Everything one answer needs recording, before any read exists.
///
/// A struct rather than a dozen arguments: `insert_answer(pool, a, b, true,
/// false, None, …)` is unreadable at the call site and one transposition away
/// from logging the wrong booleans.
///
/// ## Domain note: no `read_*` field, and that is the T1 change
///
/// This struct used to carry eight of them. It carries none now, because the row
/// is opened before the model has been asked anything — see [`attach_read`],
/// which is the only way a read reaches this table.
#[derive(Debug, Clone)]
pub struct NewAnswer {
    pub session_id: Uuid,
    pub question_id: Uuid,
    pub answer_text: String,
    pub dont_recall: bool,
    pub self_check: serde_json::Value,
    /// The receipts she said she would point to, as the phrases she was shown.
    /// `None` = she never opened the control; `Some([])` = she opened it and
    /// picked nothing. Two different facts about the same answer, kept apart.
    pub points_to: Option<serde_json::Value>,
    /// The question AS ASKED, copied here at answer time. Chuck's sheet and the
    /// review page print this rather than joining the deck's current text: an
    /// answer is a moment, and a later edit must not re-write what she was asked.
    pub question_text: String,
    pub mark: String,
    /// Why this row has no read YET.
    ///
    /// ## Domain note: the state that did not exist before T1
    ///
    /// Every row used to have `read_error IS NOT NULL` whenever `read_text` was
    /// NULL — all four failure arms filled it, so "no read and no reason" was
    /// unreachable **[measured: 0 of 12 rows on DEV]**. Writing the row first
    /// makes that combination the shape of a read IN FLIGHT, and it would also be
    /// the shape of a process that died mid-read. Two operationally distinct
    /// states, one observable, which Standing Rule 1 forbids.
    ///
    /// So the insert writes a marker saying the read is in flight and
    /// [`attach_read`] clears it. A backend that dies between the two leaves a row
    /// that SAYS SO, in words, instead of a silent blank.
    pub read_error: Option<String>,
}

/// What one finished read writes back onto the row it belongs to.
///
/// Every field is nullable and every one of them is legitimately `None` on some
/// real outcome — an abstain has no parts, a judgement has no abstain reason, a
/// call that never returned has no tokens. The struct exists so those cannot be
/// passed in the wrong order.
#[derive(Debug, Clone, Default)]
pub struct AnswerRead {
    /// The single composed line the untouched frontend renders.
    pub read_text: Option<String>,
    pub read_ok: Option<bool>,
    /// The operator's reason, or `None` to CLEAR the in-flight marker on success.
    pub read_error: Option<String>,
    pub read_abstain_reason: Option<String>,
    pub read_call: Option<String>,
    pub read_why: Option<String>,
    pub read_pointers: Option<serde_json::Value>,
    pub read_keys: Option<serde_json::Value>,
    pub read_version: Option<String>,
    pub read_input_tokens: Option<i32>,
    pub read_output_tokens: Option<i32>,
    pub read_ms: Option<i32>,
    pub read_model: Option<String>,
    pub read_raw_reply: Option<String>,
    /// How many model calls this answer cost. `None` when none were made.
    ///
    /// Without it the accumulated token count cannot be read: a row saying 4,200
    /// input tokens is uninterpretable if nobody can tell one expensive call from
    /// two ordinary ones.
    pub read_attempts: Option<i16>,
    /// Which parts were stored OVER their ceiling, and what the ceiling was.
    ///
    /// `None` in the ordinary case. Stored rather than only logged so a WAVE of
    /// overruns — a model that changed, or a ceiling set wrong — is visible in the
    /// permanent record after the log window has rolled over.
    pub read_overruns: Option<serde_json::Value>,
}

/// Open the row for one answered question. Returns its id.
///
/// The read columns are left NULL and `read_error` carries the in-flight marker
/// — see [`NewAnswer::read_error`].
pub async fn insert_answer(pool: &PgPool, answer: &NewAnswer) -> Result<Uuid, PipelineRepoError> {
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO practice_answers \
         (session_id, question_id, answer_text, dont_recall, read_error, \
          self_check, points_to, question_text, mark) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id",
    )
    .bind(answer.session_id)
    .bind(answer.question_id)
    .bind(&answer.answer_text)
    .bind(answer.dont_recall)
    .bind(answer.read_error.as_deref())
    .bind(&answer.self_check)
    .bind(&answer.points_to)
    .bind(&answer.question_text)
    .bind(&answer.mark)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Attach one finished read to an answer already on disk.
///
/// Returns whether a row was touched, so the caller can tell "recorded" from "no
/// such answer" rather than reporting success for a write that hit nothing.
///
/// ## Why this may NOT fail the request
///
/// Her answer is already committed. A 500 here would tell Marie the answer was
/// lost when it is on disk — the precise lie the two-write shape exists to
/// prevent. The caller logs this failure and shows the same "no read" surface as
/// every other read failure; see `api::practice_answers`.
///
/// ## Domain note: this writes ONLY read columns
///
/// It names no column that records something a human did. `answer_text`,
/// `points_to`, `self_check`, `mark` and `question_text` are untouchable here by
/// construction, which is what keeps the hard constraint — no existing answer
/// rewritten — a property of the SQL rather than a promise in a comment.
pub async fn attach_read(
    pool: &PgPool,
    answer_id: Uuid,
    read: &AnswerRead,
) -> Result<bool, PipelineRepoError> {
    let done = sqlx::query(
        "UPDATE practice_answers SET \
           read_text = $2, read_ok = $3, read_error = $4, read_abstain_reason = $5, \
           read_call = $6, read_why = $7, read_pointers = $8, read_keys = $9, \
           read_version = $10, read_input_tokens = $11, read_output_tokens = $12, \
           read_ms = $13, read_model = $14, read_raw_reply = $15, \
           read_attempts = $16, read_overruns = $17 \
         WHERE id = $1",
    )
    .bind(answer_id)
    .bind(read.read_text.as_deref())
    .bind(read.read_ok)
    .bind(read.read_error.as_deref())
    .bind(read.read_abstain_reason.as_deref())
    .bind(read.read_call.as_deref())
    .bind(read.read_why.as_deref())
    .bind(&read.read_pointers)
    .bind(&read.read_keys)
    .bind(read.read_version.as_deref())
    .bind(read.read_input_tokens)
    .bind(read.read_output_tokens)
    .bind(read.read_ms)
    .bind(read.read_model.as_deref())
    .bind(read.read_raw_reply.as_deref())
    .bind(read.read_attempts)
    .bind(&read.read_overruns)
    .execute(pool)
    .await?;
    Ok(done.rows_affected() == 1)
}

/// Record that she opened the stronger-answer drawer.
///
/// Returns whether a row was touched, so the route can tell "recorded" from "no
/// such answer" rather than reporting success for a write that hit nothing.
pub async fn mark_help_opened(pool: &PgPool, answer_id: Uuid) -> Result<bool, PipelineRepoError> {
    let done = sqlx::query("UPDATE practice_answers SET help_opened = TRUE WHERE id = $1")
        .bind(answer_id)
        .execute(pool)
        .await?;
    Ok(done.rows_affected() == 1)
}

/// Settle an answer when Marie leaves the reveal: her mark, and her four boxes.
///
/// Both are decided AFTER she has read the reveal, which is why they are not part
/// of [`insert_answer`] — see this module's header for the three steps.
///
/// Returns whether a row was touched, so the route can tell "settled" from "no
/// such answer" rather than reporting success for a write that hit nothing.
pub async fn close_answer(
    pool: &PgPool,
    answer_id: Uuid,
    mark: &str,
    self_check: &serde_json::Value,
) -> Result<bool, PipelineRepoError> {
    let done = sqlx::query("UPDATE practice_answers SET mark = $2, self_check = $3 WHERE id = $1")
        .bind(answer_id)
        .bind(mark)
        .bind(self_check)
        .execute(pool)
        .await?;
    Ok(done.rows_affected() == 1)
}

// The write half's guarantees are pinned in a SIBLING file, not here.
//
// ## Why the tests are not in this module
//
// Two guards in this tree read SOURCE TEXT rather than types: `sql_invariants`
// scans every file for statement/bind arity, and `practice_sql_shape` parses the
// covered repository files for column lists. A test that spells `INSERT INTO
// practice_answers` in a helper is, to both of them, a statement — one with no
// column list and no binds. Putting these tests beside the code they check made
// three real guards fail on a phantom.
//
// The sibling is outside the SQL cover, so its strings are strings. This is the
// same reasoning `sql_invariants` gives for excluding its own source.
#[cfg(test)]
#[path = "practice_answers_write_tests.rs"]
mod tests;
