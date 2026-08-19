//! The deck editor's writes, and the record every one of them leaves.
//!
//! Part B of CC_TASK_PRACTICE_V1_CHUCK_REVIEW_v1. Chuck re-orders, re-words,
//! hides and adds; Marie is told what changed. This module is both halves,
//! because they are one act: nothing here changes a question without also
//! writing the row that says who changed it and what it was before.
//!
//! ## Why every function takes `changed_by`
//!
//! There is one login. "Editing as Chuck" is the honest substitute for the
//! account separation this build does not have, and it is only worth anything if
//! it is IMPOSSIBLE to make a change without it. So it is a parameter on every
//! write rather than a field somebody remembers to set.
//!
//! ## What this module cannot do
//!
//! Delete. `practice_answers.question_id` is `ON DELETE RESTRICT` and Chuck's
//! sheet is the record of what Marie was actually asked. [`set_hidden`] is the
//! whole of "take it out", and it is reversible.
//!
//! ## CRITICAL — the pipeline pool
//!
//! Every table here lives in `colossus_legal_v2`.

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::PipelineRepoError;

/// One recorded edit, as Marie's "what changed" list and Chuck's sheet read it.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DeckChangeRecord {
    pub question_id: Uuid,
    pub change_kind: String,
    pub field: Option<String>,
    pub after_value: Option<String>,
    pub changed_by: String,
    pub changed_at: chrono::DateTime<chrono::Utc>,
}

/// What one edit is, on its way to the log.
///
/// A struct rather than six arguments: four of them are `Option<String>` and a
/// transposition would record the new text as the old one — which is the one
/// error this table exists to make impossible.
#[derive(Debug, Clone)]
pub struct NewChange<'a> {
    pub scenario_id: Uuid,
    pub question_id: Uuid,
    /// `added` | `reworded` | `edited` | `moved` | `hidden` | `unhidden`.
    pub change_kind: &'a str,
    /// Which field an `edited` change touched. `None` on the other kinds.
    pub field: Option<&'a str>,
    pub before_value: Option<&'a str>,
    pub after_value: Option<&'a str>,
    pub changed_by: &'a str,
}

/// Record one edit. Always called inside the same transaction as the edit.
pub async fn log_change(
    tx: &mut Transaction<'_, Postgres>,
    change: &NewChange<'_>,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "INSERT INTO practice_deck_changes \
         (scenario_id, question_id, change_kind, field, before_value, after_value, changed_by) \
         VALUES ($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(change.scenario_id)
    .bind(change.question_id)
    .bind(change.change_kind)
    .bind(change.field)
    .bind(change.before_value)
    .bind(change.after_value)
    .bind(change.changed_by)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Set one editable field on one question.
///
/// ## Why the column name is chosen from a fixed list by the CALLER
///
/// `format!`-ing a column into SQL is injection unless the value can only be one
/// of a few known words. [`super::super::super::api::practice_editor`] maps the
/// request's field name through an exhaustive `match` before this is reached, so
/// what arrives is a `&'static str` this crate wrote — never a client's string.
/// Stated here because the safety lives at the call site, which is exactly where
/// a reader of THIS function cannot see it.
pub async fn set_field(
    tx: &mut Transaction<'_, Postgres>,
    question_id: Uuid,
    column: &'static str,
    value: Option<&str>,
) -> Result<(), PipelineRepoError> {
    let sql =
        format!("UPDATE practice_questions SET {column} = $2, updated_at = NOW() WHERE id = $1");
    sqlx::query(&sql)
        .bind(question_id)
        .bind(value)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Set one question's tactic. Separate from [`set_field`] because it is the only
/// editable column that is not TEXT, and binding an `Option<i16>` through the
/// same function would mean a second generic parameter for one caller.
pub async fn set_tactic(
    tx: &mut Transaction<'_, Postgres>,
    question_id: Uuid,
    tactic: Option<i16>,
) -> Result<(), PipelineRepoError> {
    sqlx::query("UPDATE practice_questions SET tactic = $2, updated_at = NOW() WHERE id = $1")
        .bind(question_id)
        .bind(tactic)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Swap two questions' `sort_order`, which is what ▲ and ▼ do.
///
/// ## Rust Learning: why this is THREE statements and not one
///
/// `sort_order` is `UNIQUE (scenario_id, sort_order)`, so writing A's number
/// onto B before moving A would collide mid-statement. Parking one row on a
/// number nothing can hold — negative, which the deck never uses — is the
/// ordinary way round it, and the transaction means nobody ever sees the park.
pub async fn swap_sort_order(
    tx: &mut Transaction<'_, Postgres>,
    first: Uuid,
    second: Uuid,
) -> Result<(), PipelineRepoError> {
    let a: (i32,) = sqlx::query_as("SELECT sort_order FROM practice_questions WHERE id = $1")
        .bind(first)
        .fetch_one(&mut **tx)
        .await?;
    let b: (i32,) = sqlx::query_as("SELECT sort_order FROM practice_questions WHERE id = $1")
        .bind(second)
        .fetch_one(&mut **tx)
        .await?;

    for (id, order) in [(first, -1), (second, a.0), (first, b.0)] {
        sqlx::query(
            "UPDATE practice_questions SET sort_order = $2, updated_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .bind(order)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

/// Hide a question, or put it back. Never a delete.
pub async fn set_hidden(
    tx: &mut Transaction<'_, Postgres>,
    question_id: Uuid,
    by: Option<&str>,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "UPDATE practice_questions \
         SET hidden_at = CASE WHEN $2::text IS NULL THEN NULL ELSE NOW() END, \
             hidden_by = $2, \
             updated_at = NOW() \
         WHERE id = $1",
    )
    .bind(question_id)
    .bind(by)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// The next free `sort_order` in a scenario's deck.
///
/// A new question goes at the END and is then moved with the arrows. Inserting
/// it in the middle would renumber every row below it — several `moved` rows in
/// the change log for one act nobody performed.
pub async fn next_sort_order(
    tx: &mut Transaction<'_, Postgres>,
    scenario_id: Uuid,
) -> Result<i32, PipelineRepoError> {
    let row: (Option<i32>,) =
        sqlx::query_as("SELECT MAX(sort_order) FROM practice_questions WHERE scenario_id = $1")
            .bind(scenario_id)
            .fetch_one(&mut **tx)
            .await?;
    Ok(row.0.unwrap_or(0) + 1)
}

/// Everything a hand-added question carries. See [`insert_question`].
#[derive(Debug, Clone)]
pub struct NewQuestion<'a> {
    pub scenario_id: Uuid,
    pub side: &'a str,
    pub kind: &'a str,
    pub text: &'a str,
    pub tactic: Option<i16>,
    pub follows_key: Option<&'a str>,
    pub watch_for: Option<&'a str>,
    pub source_kind: &'a str,
    pub source_ref: Option<&'a str>,
    pub receipt: Option<&'a str>,
    pub sort_order: i32,
    pub created_by: &'a str,
}

/// Insert a question somebody typed on the page. Returns its id.
///
/// ## Domain note: no `deck_key`
///
/// The key is the deck FILE's handle, and this question is not in the file. A
/// key invented here would collide with the next one the architect writes, and
/// `--update` would then reconcile the wrong two rows. It stays NULL until
/// somebody puts the question in the file and gives it one.
pub async fn insert_question(
    tx: &mut Transaction<'_, Postgres>,
    question: &NewQuestion<'_>,
) -> Result<Uuid, PipelineRepoError> {
    let row: (Uuid,) = sqlx::query_as(
        "INSERT INTO practice_questions \
         (scenario_id, side, kind, text, tactic, follows_key, watch_for, source_kind, \
          source_ref, receipt, sort_order, created_by) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) RETURNING id",
    )
    .bind(question.scenario_id)
    .bind(question.side)
    .bind(question.kind)
    .bind(question.text)
    .bind(question.tactic)
    .bind(question.follows_key)
    .bind(question.watch_for)
    .bind(question.source_kind)
    .bind(question.source_ref)
    .bind(question.receipt)
    .bind(question.sort_order)
    .bind(question.created_by)
    .fetch_one(&mut **tx)
    .await?;
    Ok(row.0)
}

/// Every change to this scenario's deck since one instant, newest first.
///
/// `None` for `since` returns the whole history — which is what a scenario with
/// no ended sitting behind it needs, and it is not a special case: a witness who
/// has never sat down has had everything change since she last did.
pub async fn changes_since(
    pool: &PgPool,
    scenario_id: Uuid,
    since: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<Vec<DeckChangeRecord>, PipelineRepoError> {
    sqlx::query_as::<_, DeckChangeRecord>(
        "SELECT question_id, change_kind, field, after_value, changed_by, changed_at \
         FROM practice_deck_changes \
         WHERE scenario_id = $1 AND ($2::timestamptz IS NULL OR changed_at > $2) \
         ORDER BY changed_at DESC, id DESC",
    )
    .bind(scenario_id)
    .bind(since)
    .fetch_all(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// The changes made on ONE calendar day, oldest first — the foot of Chuck's
/// sheet.
///
/// Compared in UTC, like every other "day" on these tables. See
/// `services::practice_status::same_day` for what that costs and why nobody has
/// fixed it yet.
pub async fn changes_on_day(
    pool: &PgPool,
    scenario_id: Uuid,
    day: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<DeckChangeRecord>, PipelineRepoError> {
    sqlx::query_as::<_, DeckChangeRecord>(
        "SELECT question_id, change_kind, field, after_value, changed_by, changed_at \
         FROM practice_deck_changes \
         WHERE scenario_id = $1 AND changed_at::date = $2::date \
         ORDER BY changed_at, id",
    )
    .bind(scenario_id)
    .bind(day)
    .fetch_all(pool)
    .await
    .map_err(PipelineRepoError::from)
}

/// When each question was last answered, for the `changed` badge.
///
/// The badge stands until she has answered a question once AFTER the change, so
/// the comparison needs both instants. Returned as a list rather than joined
/// into the change query because the two are asked for different reasons and one
/// of them is needed even when nothing has changed.
pub async fn last_answered_at(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Vec<(Uuid, chrono::DateTime<chrono::Utc>)>, PipelineRepoError> {
    let rows: Vec<(Uuid, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
        "SELECT a.question_id, MAX(a.answered_at) \
         FROM practice_answers a JOIN practice_sessions s ON s.id = a.session_id \
         WHERE s.scenario_id = $1 GROUP BY a.question_id",
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
