//! Writing and retiring the attributed notes on a chronology event (design R8).
//!
//! Split from `chronology_write` for Rule 17 — that module sits at ~215
//! production lines against a 300 limit and owns the EVENT's statements. The
//! seam is honest as well as arithmetical: everything there changes the dated
//! fact itself and is signed into the event's history; a note is a separate
//! attributed row with its own author and its own life, and R8's whole point is
//! that three writers never share one field.
//!
//! ## CRITICAL — the pipeline pool
//!
//! `chronology_event_notes` lives in `colossus_legal_v2`, so every call here
//! takes the PIPELINE pool, never `pg_pool`.
//!
//! ## Notes soft-delete, like everything else in the chronology
//!
//! Nothing is removed. `deleted_at` is set and the read side filters it out, so
//! a note's author stays readable forever and a deletion is recoverable in the
//! same sense an event's is.

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::PipelineRepoError;

/// Insert one note and return the id the database generated.
///
/// `created_by` is not an `Option` here even though the column is nullable: the
/// column allows NULL because the SEED wrote rows before any human touched them,
/// and every note this function will ever write comes from an authenticated
/// request. A parameter that cannot be absent is better expressed as one that
/// is not optional than as one documented not to be.
pub async fn insert_note(
    executor: impl sqlx::PgExecutor<'_>,
    event_id: Uuid,
    note: &str,
    created_by: &str,
) -> Result<Uuid, PipelineRepoError> {
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO chronology_event_notes (event_id, note, created_by) \
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(event_id)
    .bind(note)
    .bind(created_by)
    .fetch_one(executor)
    .await?;
    Ok(id)
}

/// One note as it stands, deleted or not — what the delete path checks first.
///
/// The read module's `list_notes_for_event` never returns a deleted note, which
/// is right for a page. Deciding whether somebody may delete a note needs the
/// row whatever state it is in, and needs its author, so this is its own read
/// with its own type.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChronologyNoteStateRow {
    pub id: Uuid,
    pub event_id: Uuid,
    /// Who signed it. `Option` because the column is, and because a note whose
    /// author is NULL must NOT be deletable by whoever happens to ask — see
    /// [`note_is_deletable_by`].
    pub created_by: Option<String>,
    /// NULL = live.
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Read one note whatever state it is in, or `None` when there is no such id.
pub async fn get_note_any_state(
    executor: impl sqlx::PgExecutor<'_>,
    id: Uuid,
) -> Result<Option<ChronologyNoteStateRow>, PipelineRepoError> {
    let row = sqlx::query_as::<_, ChronologyNoteStateRow>(
        "SELECT id, event_id, created_by, deleted_at FROM chronology_event_notes WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(executor)
    .await?;
    Ok(row)
}

/// May `username` delete this note?
///
/// # Domain note — R8's attributed-notes model, stated as a rule
///
/// R2 makes the three authors equal on EVENTS: anyone may add, edit or delete
/// any event, and history makes every act attributable. Notes are deliberately
/// narrower, and the design says why: a note is a signed remark, not a shared
/// field, so "the author may delete their own note". Chuck's note about the
/// certified copy is Chuck's to withdraw.
///
/// ## Why a NULL author is deletable by nobody
///
/// A note with no `created_by` was written by no session this build can name.
/// Treating that as "anyone may delete it" would make an unsigned row the one
/// row with the weakest protection, which is exactly backwards — an unattributed
/// remark is the one nobody can prove is theirs. It stays until somebody decides
/// what to do with it, and today no such row exists.
///
/// Pure — no I/O, so the rule is testable without a database.
pub fn note_is_deletable_by(note: &ChronologyNoteStateRow, username: &str) -> bool {
    note.created_by.as_deref() == Some(username)
}

/// Soft-delete one live note. Returns the rows changed.
///
/// `deleted_at IS NULL` in the WHERE means deleting an already-deleted note
/// reports `0` rather than re-stamping a row and pretending something happened.
/// The author check is the CALLER's — see [`note_is_deletable_by`] — because a
/// refusal has to name which rule it broke, and a query that simply matched
/// nothing could not tell "not yours" from "not there".
pub async fn soft_delete_note(
    executor: impl sqlx::PgExecutor<'_>,
    id: Uuid,
) -> Result<u64, PipelineRepoError> {
    let done = sqlx::query(
        "UPDATE chronology_event_notes SET deleted_at = NOW() \
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .execute(executor)
    .await?;
    Ok(done.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(created_by: Option<&str>) -> ChronologyNoteStateRow {
        ChronologyNoteStateRow {
            id: Uuid::nil(),
            event_id: Uuid::nil(),
            created_by: created_by.map(str::to_string),
            deleted_at: None,
        }
    }

    #[test]
    fn an_author_may_delete_their_own_note() {
        assert!(note_is_deletable_by(&note(Some("chuck")), "chuck"));
    }

    #[test]
    fn one_author_may_not_delete_anothers_note() {
        // R8's line, and the one place the chronology is NOT equal-for-all: R2
        // makes every author equal on EVENTS, and notes are signed remarks.
        assert!(!note_is_deletable_by(&note(Some("chuck")), "marie"));
    }

    #[test]
    fn an_unsigned_note_is_deletable_by_nobody() {
        // Including by somebody whose username is the empty string, which is
        // what a naive `created_by.unwrap_or_default() == username` would have
        // let through.
        assert!(!note_is_deletable_by(&note(None), "roman"));
        assert!(!note_is_deletable_by(&note(None), ""));
    }

    #[test]
    fn the_check_is_exact_and_not_a_prefix() {
        // "marie" must not open "marie.awad"'s notes, in either direction.
        assert!(!note_is_deletable_by(&note(Some("marie.awad")), "marie"));
        assert!(!note_is_deletable_by(&note(Some("marie")), "marie.awad"));
    }
}
