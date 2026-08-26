//! Every statement that CHANGES a chronology event, and nothing else.
//!
//! ## ⚑ THIS MODULE IS THE ONE WRITE PATH'S FLOOR
//!
//! `INSERT INTO chronology_events` appears in this repository exactly once, in
//! [`insert_event`] below, and `UPDATE chronology_events` exactly once more per
//! kind of change. That is not tidiness — it is the invariant Phase C's design
//! asks for ("every write goes through ONE guarded write path"), and it is
//! PROVED rather than asserted: `chronology_one_write_path_tests` strips this
//! crate's comments and fails if a second event-writing statement exists
//! anywhere outside this file.
//!
//! The guard that decides WHO may call these and what gets recorded when they
//! do lives one layer up, in `services::chronology_guard`. This layer knows SQL
//! and nothing about authorisation: a repository that checked permissions would
//! be a second place to get permissions wrong.
//!
//! ## Phase A's note, kept because it came true
//!
//! Until 2026-08-26 this module's header said there was no HTTP path to any
//! function here, and that "if this module ever appears in an `api::` import
//! list before Phase C, the review question asks itself". Phase C is here, the
//! import list now includes it, and the question was asked and answered: the
//! api layer reaches these through the guard, never directly.
//!
//! Every function takes `impl sqlx::PgExecutor<'_>` so the caller can hand it a
//! `&mut PgConnection` inside a transaction. The seed writes all 22 events and
//! their links in ONE transaction; every Phase C write is one transaction that
//! also carries its history row, so a failure half way leaves neither a partial
//! event nor a change nobody can see.

use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use super::PipelineRepoError;

/// Everything one new event needs.
///
/// ## Rust Learning: a parameter struct instead of nine positional arguments
///
/// `insert_event(pool, "awad", date, "day", false, "title", None, attrs, "roman")`
/// has three `&str` in a row and two `Option`s; swapping any two would compile
/// and write the wrong row. Naming each field at the call site makes that class
/// of mistake impossible, and it keeps the function under the argument count
/// where a reader stops tracking positions.
#[derive(Debug, Clone)]
pub struct NewChronologyEvent<'a> {
    pub case_slug: &'a str,
    pub event_date: NaiveDate,
    pub date_precision: &'a str,
    pub approximate: bool,
    /// The phase slug. A real column with a foreign key, so a value that is not
    /// one of the four is refused by the database, not merely by a reviewer.
    pub phase: &'a str,
    pub title: &'a str,
    pub fact: Option<&'a str>,
    pub attributes: &'a serde_json::Value,
    pub created_by: &'a str,
}

/// Insert one event and return the id the database generated.
///
/// No `ON CONFLICT`: an event has no natural key — two real events can share a
/// date and a title — so there is nothing to conflict ON, and a re-run that
/// silently merged rows would be worse than one that visibly doubles them. The
/// one-shot's own guard is that it refuses to seed a case that already holds
/// events.
pub async fn insert_event(
    executor: impl sqlx::PgExecutor<'_>,
    event: &NewChronologyEvent<'_>,
) -> Result<Uuid, PipelineRepoError> {
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO chronology_events \
             (case_slug, event_date, date_precision, approximate, phase, title, \
              fact, attributes, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
    )
    .bind(event.case_slug)
    .bind(event.event_date)
    .bind(event.date_precision)
    .bind(event.approximate)
    .bind(event.phase)
    .bind(event.title)
    .bind(event.fact)
    .bind(event.attributes)
    .bind(event.created_by)
    .fetch_one(executor)
    .await?;
    Ok(id)
}

/// Everything one new link needs.
#[derive(Debug, Clone)]
pub struct NewChronologyLink<'a> {
    pub event_id: Uuid,
    pub target_type: &'a str,
    pub target_id: &'a str,
    pub label: Option<&'a str>,
    /// `None` is the honest value for every seeded link: the legacy JSON carried
    /// no pinpoints, and inventing one would be exactly the fabrication the
    /// date-precision track exists to prevent.
    pub pinpoint: Option<&'a str>,
    pub created_by: &'a str,
}

/// Insert one link.
///
/// `ON CONFLICT DO NOTHING` on the natural key: linking the same target to the
/// same event twice is not an error, it is the same fact stated twice, and the
/// second statement should be a no-op rather than a constraint violation that
/// aborts a 22-event transaction.
pub async fn insert_link(
    executor: impl sqlx::PgExecutor<'_>,
    link: &NewChronologyLink<'_>,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "INSERT INTO chronology_event_links \
             (event_id, target_type, target_id, label, pinpoint, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (event_id, target_type, target_id) DO NOTHING",
    )
    .bind(link.event_id)
    .bind(link.target_type)
    .bind(link.target_id)
    .bind(link.label)
    .bind(link.pinpoint)
    .bind(link.created_by)
    .execute(executor)
    .await?;
    Ok(())
}

/// How many link rows one case's live events carry. Part of the seed's proof.
pub async fn count_links(
    executor: impl sqlx::PgExecutor<'_>,
    case_slug: &str,
) -> Result<i64, PipelineRepoError> {
    let n = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM chronology_event_links l \
         JOIN chronology_events e ON e.id = l.event_id \
         WHERE e.case_slug = $1 AND e.deleted_at IS NULL",
    )
    .bind(case_slug)
    .fetch_one(executor)
    .await?;
    Ok(n)
}

/// The mutable body of one event — everything an edit may change.
///
/// ## Rust Learning: two parameter structs for one table, on purpose
///
/// [`NewChronologyEvent`] above carries `case_slug` and `created_by`; this one
/// carries neither, because neither is editable. A row's case does not change
/// (an event does not migrate between lawsuits) and its author does not change
/// (that is what `updated_by` is for). Sharing one struct between insert and
/// update would mean an `Option` on each of those two fields and a comment
/// explaining that they are ignored on the update path — which is a runtime
/// convention where the compiler could have had a rule. Two small structs, and
/// the type system says which fields an edit is even able to name.
#[derive(Debug, Clone)]
pub struct EventEdit<'a> {
    pub event_date: NaiveDate,
    pub date_precision: &'a str,
    pub approximate: bool,
    /// The phase slug. Validated against `chronology_phases` BEFORE this runs —
    /// see `services::chronology_validate` — so a foreign-key violation here
    /// means a phase was deleted between the check and the write, not that a
    /// human typed something wrong.
    pub phase: &'a str,
    pub title: &'a str,
    pub fact: Option<&'a str>,
    pub attributes: &'a serde_json::Value,
}

/// Apply an edit to one LIVE event, stamping who did it.
///
/// Returns how many rows changed: `0` means there is no live event with that
/// id — deleted, or never existed. The caller turns that into a 404 rather than
/// this layer deciding what absence means (the same split `get_event` makes).
///
/// `deleted_at IS NULL` is part of the WHERE on purpose: editing a deleted event
/// would resurrect its content without resurrecting the row, and the next reader
/// would see a history of edits to something that is not on the page.
pub async fn update_event(
    executor: impl sqlx::PgExecutor<'_>,
    id: Uuid,
    edit: &EventEdit<'_>,
    updated_by: &str,
) -> Result<u64, PipelineRepoError> {
    let done = sqlx::query(
        "UPDATE chronology_events SET \
             event_date = $2, date_precision = $3, approximate = $4, phase = $5, \
             title = $6, fact = $7, attributes = $8, updated_by = $9, updated_at = NOW() \
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .bind(edit.event_date)
    .bind(edit.date_precision)
    .bind(edit.approximate)
    .bind(edit.phase)
    .bind(edit.title)
    .bind(edit.fact)
    .bind(edit.attributes)
    .bind(updated_by)
    .execute(executor)
    .await?;
    Ok(done.rows_affected())
}

/// Soft-delete one live event (design R10). Nothing is ever removed.
///
/// Returns the rows changed, so a second delete of the same event reports `0`
/// rather than pretending to have done something. `deleted_at IS NULL` in the
/// WHERE is what makes the operation idempotent AND honest at the same time.
pub async fn soft_delete_event(
    executor: impl sqlx::PgExecutor<'_>,
    id: Uuid,
    deleted_by: &str,
) -> Result<u64, PipelineRepoError> {
    let done = sqlx::query(
        "UPDATE chronology_events \
         SET deleted_at = NOW(), updated_by = $2, updated_at = NOW() \
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .bind(deleted_by)
    .execute(executor)
    .await?;
    Ok(done.rows_affected())
}

/// Clear the soft delete — the Undo line's whole implementation (design R10).
///
/// `deleted_at IS NOT NULL` mirrors the delete's guard: undoing an event that
/// was never deleted reports `0` instead of silently stamping `updated_by` on a
/// row nobody touched.
pub async fn undelete_event(
    executor: impl sqlx::PgExecutor<'_>,
    id: Uuid,
    restored_by: &str,
) -> Result<u64, PipelineRepoError> {
    let done = sqlx::query(
        "UPDATE chronology_events \
         SET deleted_at = NULL, updated_by = $2, updated_at = NOW() \
         WHERE id = $1 AND deleted_at IS NOT NULL",
    )
    .bind(id)
    .bind(restored_by)
    .execute(executor)
    .await?;
    Ok(done.rows_affected())
}

/// One event as it stands, DELETED OR NOT.
///
/// ## Why this exists beside `chronology::get_event`
///
/// That read is the page's, and its module promises never to hand a caller a
/// deleted row. This one is the write path's, and it must see a deleted row for
/// two reasons: the history snapshot after a delete has to record the row it
/// just deleted, and the response to a delete carries the event so the surface
/// can draw the undo line in its place.
///
/// The extra column is the whole difference, and it is why this is a separate
/// type rather than a flag on the other: a caller holding one of these has
/// `deleted_at` in hand and cannot forget to look at it.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChronologyEventStateRow {
    pub id: Uuid,
    pub case_slug: String,
    pub event_date: NaiveDate,
    pub date_precision: String,
    pub approximate: bool,
    pub phase: String,
    pub title: String,
    pub fact: Option<String>,
    pub attributes: serde_json::Value,
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_by: Option<String>,
    pub updated_at: DateTime<Utc>,
    /// NULL = live. Meaningful to every caller of this function.
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Read one event whatever state it is in, or `None` if there is no such id.
pub async fn get_event_any_state(
    executor: impl sqlx::PgExecutor<'_>,
    id: Uuid,
) -> Result<Option<ChronologyEventStateRow>, PipelineRepoError> {
    let row = sqlx::query_as::<_, ChronologyEventStateRow>(
        "SELECT id, case_slug, event_date, date_precision, approximate, phase, title, \
                fact, attributes, created_by, created_at, updated_by, updated_at, deleted_at \
         FROM chronology_events WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(executor)
    .await?;
    Ok(row)
}

/// Append ONE history row. The only statement in this crate that writes history.
///
/// ## ⚑ Append-only means append-only
///
/// There is no `update_history` and no `delete_history`, here or anywhere, and
/// `chronology_one_write_path_tests` fails if one appears. That is what makes an
/// event's history evidence rather than a field: a delete is recoverable and
/// attributable (design R10) precisely because the record of it cannot be
/// tidied away by the person who made it.
///
/// `action` is one of the four the table's CHECK allows. It is typed at the
/// layer above (`services::chronology_guard::HistoryAction`), so a caller here
/// cannot invent a fifth without the database refusing it.
pub async fn insert_history(
    executor: impl sqlx::PgExecutor<'_>,
    event_id: Uuid,
    action: &str,
    snapshot: &serde_json::Value,
    changed_by: &str,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "INSERT INTO chronology_event_history (event_id, action, snapshot, changed_by) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(event_id)
    .bind(action)
    .bind(snapshot)
    .bind(changed_by)
    .execute(executor)
    .await?;
    Ok(())
}

/// Remove one link, addressed by the natural key a human actually picked.
///
/// A HARD delete, and the only one in the chronology — deliberately. A link is
/// not a fact about the case, it is a POINTER to where a fact is written, and an
/// un-pointing leaves the event and the document both untouched. Soft-deleting
/// pointers would mean every read of every event's links carried a
/// `deleted_at IS NULL` for a row nobody would ever want back. The act is still
/// attributable: the event's history carries a snapshot after the change, and
/// snapshots are never rewritten.
///
/// Returns the rows removed, so removing a link that is not there reports `0`.
pub async fn delete_link(
    executor: impl sqlx::PgExecutor<'_>,
    event_id: Uuid,
    target_type: &str,
    target_id: &str,
) -> Result<u64, PipelineRepoError> {
    let done = sqlx::query(
        "DELETE FROM chronology_event_links \
         WHERE event_id = $1 AND target_type = $2 AND target_id = $3",
    )
    .bind(event_id)
    .bind(target_type)
    .bind(target_id)
    .execute(executor)
    .await?;
    Ok(done.rows_affected())
}
