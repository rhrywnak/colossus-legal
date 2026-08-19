//! Notes — Chuck, Marie and Roman writing to each other.
//!
//! One table, three places: the scenario (`question_id IS NULL`), one question,
//! and one attempt at a question. See the migration's header for why they are
//! one table and not three.
//!
//! ## Nothing here deletes
//!
//! [`strike_note`] is the whole of "take it back", and a struck note is still
//! rendered — struck through, with who struck it and when. A note somebody could
//! delete is a note nobody can rely on having been read, and these are notes
//! about a witness's testimony.
//!
//! ## CRITICAL — the pipeline pool
//!
//! Every table here lives in `colossus_legal_v2`.

use sqlx::PgPool;
use uuid::Uuid;

use super::PipelineRepoError;

/// One note, as every panel renders it.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct NoteRecord {
    pub id: Uuid,
    /// `None` = a note about the scenario.
    pub question_id: Option<Uuid>,
    /// `None` = a note about the question rather than about one attempt.
    pub answer_id: Option<Uuid>,
    pub author: String,
    pub text: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// `None` while the note stands.
    pub struck_at: Option<chrono::DateTime<chrono::Utc>>,
    pub struck_by: Option<String>,
}

/// One note on its way to the table.
///
/// A struct rather than five arguments: three of them are `Option<Uuid>` or
/// `&str`, and `insert_note(pool, a, Some(b), None, "Chuck", text)` is one
/// transposition away from filing a note about the wrong question.
#[derive(Debug, Clone, Copy)]
pub struct NewNote<'a> {
    pub scenario_id: Uuid,
    /// `None` for a scenario-level note.
    pub question_id: Option<Uuid>,
    /// `None` unless the note is about one attempt. A note on an attempt must
    /// also name its question — the table has a CHECK saying so.
    pub answer_id: Option<Uuid>,
    pub author: &'a str,
    pub text: &'a str,
}

/// Write one note. Returns its id.
pub async fn insert_note(pool: &PgPool, note: &NewNote<'_>) -> Result<Uuid, PipelineRepoError> {
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO practice_notes (scenario_id, question_id, answer_id, author, text) \
         VALUES ($1,$2,$3,$4,$5) RETURNING id",
    )
    .bind(note.scenario_id)
    .bind(note.question_id)
    .bind(note.answer_id)
    .bind(note.author)
    .bind(note.text)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

/// Every note on this scenario, oldest first — all three levels in one read.
///
/// ## Why one read and not three
///
/// The start card needs the scenario's, the review page needs one question's and
/// its attempts', and the deck payload needs a COUNT of the scenario's. Three
/// queries would be three round trips for a page that is one payload by design,
/// and the caller partitions the result by the two nullable columns — which is
/// the same partition the table's own CHECK already guarantees is meaningful.
pub async fn list_notes(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Vec<NoteRecord>, PipelineRepoError> {
    sqlx::query_as::<_, NoteRecord>(
        "SELECT id, question_id, answer_id, author, text, created_at, struck_at, struck_by \
         FROM practice_notes WHERE scenario_id = $1 ORDER BY created_at, id",
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// Strike one note through. Returns whether a row was touched.
///
/// Idempotent on purpose: striking an already-struck note keeps the FIRST
/// striking, because the moment somebody withdrew it is not something a second
/// press should move. The route can still tell "no such note" from "struck",
/// which is why this returns a boolean rather than nothing.
pub async fn strike_note(
    pool: &PgPool,
    note_id: Uuid,
    by: &str,
) -> Result<bool, PipelineRepoError> {
    let done = sqlx::query(
        "UPDATE practice_notes SET struck_at = NOW(), struck_by = $2 \
         WHERE id = $1 AND struck_at IS NULL",
    )
    .bind(note_id)
    .bind(by)
    .execute(pool)
    .await?;
    if done.rows_affected() == 1 {
        return Ok(true);
    }
    // Nothing was updated: either the note does not exist, or it was already
    // struck. Those are a 404 and a success, and the caller cannot tell them
    // apart without this second look.
    let exists: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM practice_notes WHERE id = $1")
        .bind(note_id)
        .fetch_optional(pool)
        .await?;
    Ok(exists.is_some())
}

/// The scenario a note belongs to, for the fence on the strike route.
pub async fn note_scenario(
    pool: &PgPool,
    note_id: Uuid,
) -> Result<Option<Uuid>, PipelineRepoError> {
    let row: Option<(Uuid,)> =
        sqlx::query_as("SELECT scenario_id FROM practice_notes WHERE id = $1")
            .bind(note_id)
            .fetch_optional(pool)
            .await?;
    Ok(row.map(|r| r.0))
}

/// One attempt at one question, as the review page stacks them.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AttemptRecord {
    pub id: Uuid,
    /// The question AS ASKED at that moment — not the deck's current text.
    pub question_text: String,
    pub answer_text: String,
    pub answered_at: chrono::DateTime<chrono::Utc>,
    pub mark: String,
    pub read_text: Option<String>,
    pub read_ok: Option<bool>,
    pub self_check: serde_json::Value,
    pub help_opened: bool,
    pub points_to: Option<serde_json::Value>,
}

/// Every attempt at one question, OLDEST first.
///
/// ## Domain note: oldest first here, newest first on screen
///
/// The numbering is what decides it. "attempt 1" must be her first attempt
/// however the list is sorted, so the numbers are assigned in the order they
/// happened and the caller reverses for display. Sorting newest-first here and
/// numbering from the top would make attempt 1 change its meaning every time she
/// answers again.
pub async fn attempts_for_question(
    pool: &PgPool,
    scenario_id: Uuid,
    question_id: Uuid,
) -> Result<Vec<AttemptRecord>, PipelineRepoError> {
    sqlx::query_as::<_, AttemptRecord>(
        "SELECT a.id, COALESCE(a.question_text, q.text) AS question_text, a.answer_text, \
                a.answered_at, a.mark, a.read_text, a.read_ok, a.self_check, \
                a.help_opened, a.points_to \
         FROM practice_answers a \
         JOIN practice_sessions s ON s.id = a.session_id \
         JOIN practice_questions q ON q.id = a.question_id \
         WHERE s.scenario_id = $1 AND a.question_id = $2 \
         ORDER BY a.answered_at, a.id",
    )
    .bind(scenario_id)
    .bind(question_id)
    .fetch_all(pool)
    .await
    .map_err(PipelineRepoError::from)
}
