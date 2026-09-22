//! "Discuss with AI" threads — one saved conversation per practice question.
//!
//! CC_TASK_QUESTION_CHAT_v1 §1. A question's thread is simply its rows in
//! `practice_discussions`, oldest first. Everyone writes the same thread; each
//! turn names its author, and each MODEL turn names the model that wrote it and
//! what the call cost.
//!
//! ## Nothing here deletes
//!
//! Like notes, a thread is a record of advice given about a witness's testimony.
//! There is no delete and no edit.
//!
//! ## CRITICAL — the pipeline pool
//!
//! `practice_discussions` lives in `colossus_legal_v2`.

use sqlx::PgPool;
use uuid::Uuid;

use super::PipelineRepoError;

// STRUCTURAL: the table's `role` CHECK vocabulary. A new role is a migration.
const ROLE_USER: &str = "user";
const ROLE_MODEL: &str = "model";

/// Who wrote a turn.
///
/// ## Rust Learning: an enum at the boundary, a string in the column
///
/// The column is TEXT with a CHECK; the code never passes a bare string, so a
/// typo like `"Model"` cannot reach the database and fail there as a constraint
/// violation — it fails to compile instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnRole {
    User,
    Model,
}

impl TurnRole {
    /// The stored code.
    pub fn code(self) -> &'static str {
        match self {
            TurnRole::User => ROLE_USER,
            TurnRole::Model => ROLE_MODEL,
        }
    }
}

/// One stored turn.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct DiscussionTurnRecord {
    pub id: Uuid,
    pub question_id: Uuid,
    pub author_user_id: String,
    pub author_name: String,
    /// `user` or `model` — see [`TurnRole::code`].
    pub role: String,
    /// The model that wrote a model turn; `None` on every user turn.
    pub model_id: Option<String>,
    pub text: String,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub ms: Option<i32>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A turn on its way to the table.
#[derive(Debug, Clone, Copy)]
pub struct NewTurn<'a> {
    pub question_id: Uuid,
    pub author_user_id: &'a str,
    pub author_name: &'a str,
    pub role: TurnRole,
    /// Required for a model turn, `None` for a user turn — the table's CHECK
    /// refuses the other two combinations.
    pub model_id: Option<&'a str>,
    pub text: &'a str,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub ms: Option<i32>,
}

const TURN_COLUMNS: &str = "id, question_id, author_user_id, author_name, role, model_id, text, \
     input_tokens, output_tokens, ms, created_at";

/// Store one turn and return it as stored.
///
/// # Errors
/// [`PipelineRepoError`] — including the table's CHECKs refusing a blank text or a
/// model turn with no model.
pub async fn insert_turn(
    pool: &PgPool,
    turn: &NewTurn<'_>,
) -> Result<DiscussionTurnRecord, PipelineRepoError> {
    let sql = format!(
        "INSERT INTO practice_discussions \
         (question_id, author_user_id, author_name, role, model_id, text, input_tokens, output_tokens, ms) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING {TURN_COLUMNS}"
    );
    sqlx::query_as::<_, DiscussionTurnRecord>(&sql)
        .bind(turn.question_id)
        .bind(turn.author_user_id)
        .bind(turn.author_name)
        .bind(turn.role.code())
        .bind(turn.model_id)
        .bind(turn.text)
        .bind(turn.input_tokens)
        .bind(turn.output_tokens)
        .bind(turn.ms)
        .fetch_one(pool)
        .await
        .map_err(PipelineRepoError::from)
}

/// One question's thread, oldest first (the id breaks a same-instant tie).
pub async fn list_thread(
    pool: &PgPool,
    question_id: Uuid,
) -> Result<Vec<DiscussionTurnRecord>, PipelineRepoError> {
    let sql = format!(
        "SELECT {TURN_COLUMNS} FROM practice_discussions \
         WHERE question_id = $1 ORDER BY created_at, id"
    );
    sqlx::query_as::<_, DiscussionTurnRecord>(&sql)
        .bind(question_id)
        .fetch_all(pool)
        .await
        .map_err(PipelineRepoError::from)
}

/// How many MODEL replies this question's thread already holds — the cap's count.
pub async fn model_turn_count(pool: &PgPool, question_id: Uuid) -> Result<i64, PipelineRepoError> {
    let row: (i64,) = sqlx::query_as(
        "SELECT count(*) FROM practice_discussions WHERE question_id = $1 AND role = $2",
    )
    .bind(question_id)
    .bind(ROLE_MODEL)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// The answer that stands for a question, with what the dock needs from it.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub struct StandingAnswer {
    pub answer_id: Uuid,
    pub answer_text: String,
    /// What she ticked to point to — `NULL` never opened, `[]` picked nothing.
    pub points_to: Option<serde_json::Value>,
    /// The latest analysis of this answer, if one was stored.
    pub read_text: Option<String>,
    /// With `read_abstain_reason`: whether that analysis FAILED (`read_failed`).
    pub read_error: Option<String>,
    pub read_abstain_reason: Option<String>,
}

impl StandingAnswer {
    /// Whether the stored analysis failed for a system reason — the dock then
    /// tells the model so, instead of passing Marie's failure sentence along.
    pub fn read_failed(&self) -> bool {
        crate::services::practice_read_outcome::is_failed_read(
            self.read_abstain_reason.as_deref(),
            self.read_error.as_deref(),
        )
    }
}

/// The current answer to a question, or `None` when nobody has answered.
///
/// Same "newest standing" rule as `practice_flow::current_answer_for`.
pub async fn standing_answer(
    pool: &PgPool,
    question_id: Uuid,
) -> Result<Option<StandingAnswer>, PipelineRepoError> {
    sqlx::query_as::<_, StandingAnswer>(
        "SELECT a.id AS answer_id, a.answer_text, a.points_to, a.read_text, \
                a.read_error, a.read_abstain_reason \
         FROM practice_answers a WHERE a.question_id = $1 \
         ORDER BY a.answered_at DESC, a.id DESC LIMIT 1",
    )
    .bind(question_id)
    .fetch_optional(pool)
    .await
    .map_err(PipelineRepoError::from)
}

#[cfg(test)]
#[path = "practice_discussions_live_tests.rs"]
mod live_tests;
