//! The practice drill's three tables, read and written.
//!
//! `practice_questions` (the deck), `practice_sessions` (one sitting) and
//! `practice_answers` (the log Chuck's sheet is rendered from). All three live in
//! `colossus_legal_v2` — see the migration's header for why.
//!
//! ## What this module deliberately cannot do
//!
//! There is no `delete_question`, no `update_question` and no "reseed". A stored
//! question is cited by `practice_answers.question_id` under `ON DELETE RESTRICT`,
//! and Chuck's sheet is the record of what Marie was actually asked. Editing the
//! deck is a page in v1 with its own audit; it is not a function some other
//! caller can reach today by accident.
//!
//! ## Rust Learning: `sqlx::FromRow` and runtime queries
//!
//! The record structs derive `FromRow`, which teaches sqlx how to build one from
//! a result row by matching COLUMN NAMES to field names. Combined with
//! `query_as::<_, Record>(sql)` this gives typed rows without the `query!` macro —
//! which matters here because the macro needs a live database (or a checked-in
//! offline cache) at COMPILE time, and these tables did not exist when the
//! container image that builds this code was last prepared.

use sqlx::PgPool;
use uuid::Uuid;

use super::PipelineRepoError;

/// One question from the deck, with everything its reveal screen renders.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PracticeQuestionRecord {
    pub id: Uuid,
    pub side: String,
    pub text: String,
    /// TACTIC_DECK_v1 card 1–7. `None` on a Chuck question — which has no tactic,
    /// as opposed to having one called "none".
    pub tactic: Option<i16>,
    pub braid_rows: Option<String>,
    pub source_kind: String,
    pub source_ref: Option<String>,
    pub receipt: Option<String>,
    pub watch_for: Option<String>,
    pub stronger: Option<String>,
    pub stronger_lean: Option<String>,
    pub pair_said: Option<String>,
    pub pair_admitted: Option<String>,
    pub sort_order: i32,
    /// Marie's one line saying what is wrong with this question, or `None`.
    /// Read with the deck because the start screen renders it on the row and
    /// Chuck's sheet lists it at the foot — one read serves both.
    pub flag_note: Option<String>,
}

/// One of Marie's three talking points, read live from the scenario record.
///
/// ## Domain note: why the points are NOT stored on the deck
///
/// They are hers, they already exist, and they are edited on the rehearsal page.
/// A copy on the deck would be a second truth that drifts the first time she
/// rewords one — and the wording she practises with has to be the wording she
/// took to Chuck.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PracticePointRecord {
    /// The number printed beside it: `item_index + 1`.
    pub position: i32,
    pub text: String,
    /// The authored exhibit phrase behind this point, or `None` when nobody has
    /// paired one. `None` renders the stored named-absence line, never a blank.
    pub exhibit: Option<String>,
}

/// A summary of the most recent ENDED session, for the start screen's line.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LastSessionRecord {
    pub ended_at: chrono::DateTime<chrono::Utc>,
    pub answered: i64,
    pub repeats: i64,
}

/// One row of Chuck's sheet.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PracticeSheetRow {
    pub side: String,
    pub braid_rows: Option<String>,
    pub tactic: Option<i16>,
    pub question: String,
    pub answer_text: String,
    pub mark: String,
    pub help_opened: bool,
}

/// Everything one answer needs recording. A struct rather than eleven arguments:
/// `insert_answer(pool, a, b, true, false, None, …)` is unreadable at the call
/// site and one transposition away from logging the wrong booleans.
#[derive(Debug, Clone)]
pub struct NewAnswer {
    pub session_id: Uuid,
    pub question_id: Uuid,
    pub answer_text: String,
    pub dont_recall: bool,
    pub read_text: Option<String>,
    pub read_ok: Option<bool>,
    pub read_error: Option<String>,
    pub read_input_tokens: Option<i32>,
    pub read_output_tokens: Option<i32>,
    pub read_ms: Option<i32>,
    pub read_model: Option<String>,
    /// What the model said when this build refused to show it. `None` otherwise.
    pub read_raw_reply: Option<String>,
    pub self_check: serde_json::Value,
    pub mark: String,
}

/// The scenario's deck, in the order it is dealt.
pub async fn list_deck(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Vec<PracticeQuestionRecord>, PipelineRepoError> {
    sqlx::query_as::<_, PracticeQuestionRecord>(
        "SELECT id, side, text, tactic, braid_rows, source_kind, source_ref, receipt, \
                watch_for, stronger, stronger_lean, pair_said, pair_admitted, sort_order, \
                flag_note \
         FROM practice_questions WHERE scenario_id = $1 ORDER BY sort_order",
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// One question by id, for the answer path.
pub async fn get_question(
    pool: &PgPool,
    question_id: Uuid,
) -> Result<Option<PracticeQuestionRecord>, PipelineRepoError> {
    sqlx::query_as::<_, PracticeQuestionRecord>(
        "SELECT id, side, text, tactic, braid_rows, source_kind, source_ref, receipt, \
                watch_for, stronger, stronger_lean, pair_said, pair_admitted, sort_order, \
                flag_note \
         FROM practice_questions WHERE id = $1",
    )
    .bind(question_id)
    .fetch_optional(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// One seeded stand-in receipt, keyed to the point it backs by printed position.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PracticePointReceipt {
    pub position: i32,
    pub text: String,
}

/// The seeded receipts for this scenario's points.
///
/// Separate from [`list_points`] rather than joined into it because they are
/// different KINDS of fact: one is the scenario's own record, the other is a
/// stand-in the deck carries until the pairing editor exists. Keeping the two
/// reads apart is what lets the caller state the precedence out loud instead of
/// burying it in a `COALESCE`.
pub async fn list_point_receipts(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Vec<PracticePointReceipt>, PipelineRepoError> {
    sqlx::query_as::<_, PracticePointReceipt>(
        "SELECT position, text FROM practice_point_receipts \
         WHERE scenario_id = $1 ORDER BY position",
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// The scenario's talking points, with the exhibit phrase a human paired.
///
/// `MIN(note)` collapses the m:n link to the one phrase the point shows. A point
/// with two paired exhibits is not a state this surface can render — the
/// rehearsal page has the same shape — and taking the first is the same choice
/// made there rather than a new one invented here.
pub async fn list_points(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Vec<PracticePointRecord>, PipelineRepoError> {
    sqlx::query_as::<_, PracticePointRecord>(
        "SELECT (i.item_index + 1) AS position, i.text, MIN(f.note) AS exhibit \
         FROM scenario_responses r \
         JOIN response_items i ON i.response_id = r.id \
         LEFT JOIN response_item_fact_refs f ON f.response_item_id = i.id \
         WHERE r.scenario_id = $1 \
         GROUP BY i.item_index, i.text \
         ORDER BY i.item_index",
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// The most recently ENDED session for this scenario, and its counts.
///
/// Only ended sessions: a sitting she walked away from is not a session she did,
/// and reporting it as one would put a number on the start screen that says she
/// practised when she did not.
pub async fn last_ended_session(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Option<LastSessionRecord>, PipelineRepoError> {
    sqlx::query_as::<_, LastSessionRecord>(
        "SELECT s.ended_at, \
                COUNT(a.id) AS answered, \
                COUNT(a.id) FILTER (WHERE a.mark = 'repeat') AS repeats \
         FROM practice_sessions s \
         LEFT JOIN practice_answers a ON a.session_id = s.id \
         WHERE s.scenario_id = $1 AND s.ended_at IS NOT NULL \
         GROUP BY s.id, s.ended_at ORDER BY s.ended_at DESC LIMIT 1",
    )
    .bind(scenario_id)
    .fetch_optional(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// Open a session. Returns the id the answers will cite.
pub async fn start_session(
    pool: &PgPool,
    scenario_id: Uuid,
    who: &str,
    sitting: &NewSitting<'_>,
) -> Result<Uuid, PipelineRepoError> {
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO practice_sessions \
         (scenario_id, who, count, queue, skipped_today) \
         VALUES ($1,$2,$3,$4,$5) RETURNING id",
    )
    .bind(scenario_id)
    .bind(who)
    .bind(sitting.count)
    .bind(sitting.queue)
    .bind(sitting.skipped_today)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// What the sitting was, at the moment it opened.
///
/// A struct rather than three more positional arguments: two of the three are
/// `serde_json::Value` holding arrays of the same shape, and
/// `start_session(pool, id, who, a, b, c)` is one transposition away from
/// recording the questions she kept OUT as the ones she was dealt.
#[derive(Debug, Clone, Copy)]
pub struct NewSitting<'a> {
    /// What she chose off the count pills, or `None` when nothing chose.
    pub count: Option<i32>,
    /// The dealt question ids, in order, as a JSON array.
    pub queue: &'a serde_json::Value,
    /// The ids she kept out on the start screen, as a JSON array.
    pub skipped_today: &'a serde_json::Value,
}

/// The scenario a session belongs to, or `None` if there is no such session.
pub async fn session_scenario(
    pool: &PgPool,
    session_id: Uuid,
) -> Result<Option<Uuid>, PipelineRepoError> {
    let row: Option<(Uuid,)> =
        sqlx::query_as("SELECT scenario_id FROM practice_sessions WHERE id = $1")
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|r| r.0))
}

/// Close a session so the start screen can report it. Idempotent: a session
/// already ended keeps its first `ended_at`, because the moment she finished is
/// not something a second request should move.
pub async fn end_session(pool: &PgPool, session_id: Uuid) -> Result<(), PipelineRepoError> {
    sqlx::query("UPDATE practice_sessions SET ended_at = NOW() WHERE id = $1 AND ended_at IS NULL")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Record one answered question. Returns the answer's id, which the drawer's
/// help flag later addresses.
pub async fn insert_answer(pool: &PgPool, answer: &NewAnswer) -> Result<Uuid, PipelineRepoError> {
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO practice_answers \
         (session_id, question_id, answer_text, dont_recall, read_text, read_ok, read_error, \
          read_input_tokens, read_output_tokens, read_ms, read_model, read_raw_reply, \
          self_check, mark) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14) RETURNING id",
    )
    .bind(answer.session_id)
    .bind(answer.question_id)
    .bind(&answer.answer_text)
    .bind(answer.dont_recall)
    .bind(answer.read_text.as_deref())
    .bind(answer.read_ok)
    .bind(answer.read_error.as_deref())
    .bind(answer.read_input_tokens)
    .bind(answer.read_output_tokens)
    .bind(answer.read_ms)
    .bind(answer.read_model.as_deref())
    .bind(answer.read_raw_reply.as_deref())
    .bind(&answer.self_check)
    .bind(&answer.mark)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
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

/// Settle an answer when Marie leaves the reveal: her four boxes, and the mark.
///
/// ## Why the answer is written in TWO steps and not one
///
/// The read has to exist before she can react to it, and her four boxes and her
/// "ask me this one again later" both happen AFTER she has read it. One write
/// would mean either recording her answer before the model saw it (losing the
/// read) or holding her typed answer in the browser until she pressed a button
/// (losing it if she walked away). So the row is created when she answers —
/// which is the moment worth surviving a closed laptop — and settled when she
/// moves on.
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

/// The session's answers, in the order she gave them — Chuck's sheet.
pub async fn sheet_rows(
    pool: &PgPool,
    session_id: Uuid,
) -> Result<Vec<PracticeSheetRow>, PipelineRepoError> {
    sqlx::query_as::<_, PracticeSheetRow>(
        "SELECT q.side, q.braid_rows, q.tactic, q.text AS question, \
                a.answer_text, a.mark, a.help_opened \
         FROM practice_answers a JOIN practice_questions q ON q.id = a.question_id \
         WHERE a.session_id = $1 ORDER BY a.answered_at, a.id",
    )
    .bind(session_id)
    .fetch_all(pool)
    .await
    .map_err(PipelineRepoError::from)
}

// The SQL in this file is opaque to the compiler (`sqlx::query`, not `query!`),
// so a column name that no migration creates is neither a build error nor a
// unit-test failure — it is a runtime "column … does not exist" on the first
// real request, which is how 2026-08-18 happened. The sibling module reads this
// file and the migrations off disk and asserts they agree (Rule 21), the same
// guard `scan_runs.rs` has carried since the 2026-07-19 defect of that family.
#[cfg(test)]
#[path = "practice_sql_shape.rs"]
mod sql_shape;

#[cfg(test)]
#[path = "practice_schema_tests.rs"]
mod schema_tests;
