//! ⚑ THE SEAL for every timeline-subset write (T1.3).
//!
//! The sibling of `services::chronology_guard`, and the same three rules hold:
//!
//!  1. **It is not anonymous.** Reads are open — looking at a story is not
//!     privileged, exactly as looking at the chronology is not — but a write
//!     requires an authenticated user. The refusal is axum's: every write
//!     handler in `api::timeline_subsets` takes `user: AuthUser` rather than
//!     `Option<AuthUser>`, so an anonymous request is a 401 before a handler
//!     body runs, and a scan test proves no handler skips it.
//!  2. **It stamps the acting user**, through the same
//!     `services::practice_notes::attribution` helper every other stamped path
//!     in this codebase uses. Identity comes from the Authentik headers, never
//!     from a picker on a screen. [`ChronologyWriter`] is reused verbatim from
//!     the event guard rather than re-declared: two structs holding the same two
//!     strings is two places for the id and the display name to swap over.
//!  3. **It writes exactly one history row**, holding a full snapshot of the
//!     subset AFTER the write, INCLUDING ITS ORDERED EVENT LIST.
//!     [`seal_and_commit`] is the only way a subset transaction is committed, so
//!     a write that recorded no history is not something a handler can forget —
//!     it is something a handler cannot express.
//!
//! ## Rust Learning: why the seal takes the transaction BY VALUE
//!
//! `seal_and_commit(mut tx, …)` consumes the `Transaction`. That is what makes
//! rule 3 structural rather than advisory: a handler that wanted to commit
//! without history would have to call `tx.commit()` itself, and it no longer
//! owns a `tx` to call it on — the seal took it.
//!
//! ## ⚑ What is NOT sealed, and why that is a decision and not an omission
//!
//! Attaching a subset to a scenario, and detaching it, write no history row
//! here. They change `scenario_subsets`, which is the SCENARIO's fact about the
//! subset — not the subset's content, which is what this table snapshots. The
//! same reasoning makes a detach the one hard delete in the feature: a link is
//! not content. Stated as a choice in the T1 report, where it is also noted that
//! attachment therefore has no audit trail beyond the row itself.

use uuid::Uuid;

use crate::repositories::pipeline_repository::chronology_subset_write::insert_subset_history;
use crate::repositories::pipeline_repository::chronology_subsets::{
    get_subset_any_state, list_subset_event_ids, ChronologySubsetRow, SubsetEventRefRow,
};
use crate::repositories::pipeline_repository::PipelineRepoError;
use crate::services::chronology_guard::ChronologyWriter;

/// What happened to a subset, as `chronology_subset_history.action` stores it.
///
/// ## Rust Learning: a fieldless enum in front of a SQL CHECK
///
/// The column carries a `CHECK (action IN ('created','updated','events_replaced',
/// 'deleted','restored'))`. A `&str` parameter would compile for any word and
/// fail at the database with a constraint violation — a 500, at write time, for
/// a typo a compiler could have caught. The enum makes the five the only
/// spellable values, and `the_action_words_match_the_check` pins them against
/// the migration file so the two lists cannot drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubsetHistoryAction {
    Created,
    /// The name and/or the description changed.
    Updated,
    /// The picker's Save: the whole ordered event set was rewritten. ONE row,
    /// because a person did one thing.
    EventsReplaced,
    Deleted,
    Restored,
}

impl SubsetHistoryAction {
    /// The token the column stores.
    pub fn as_str(self) -> &'static str {
        match self {
            SubsetHistoryAction::Created => "created",
            SubsetHistoryAction::Updated => "updated",
            SubsetHistoryAction::EventsReplaced => "events_replaced",
            SubsetHistoryAction::Deleted => "deleted",
            SubsetHistoryAction::Restored => "restored",
        }
    }

    /// Every action, for the test that pins them against the migration.
    pub const ALL: &'static [SubsetHistoryAction] = &[
        SubsetHistoryAction::Created,
        SubsetHistoryAction::Updated,
        SubsetHistoryAction::EventsReplaced,
        SubsetHistoryAction::Deleted,
        SubsetHistoryAction::Restored,
    ];
}

/// The subset as it stood after a write, in the shape history stores.
///
/// ## ⚑ Why the EVENT LIST is in the snapshot
///
/// A subset's content is not its name — it is which events, in what order, with
/// what notes. A snapshot that held only the columns of `chronology_subsets`
/// would record every rename perfectly and record nothing at all about the
/// reorder that is the whole feature. So the ordered references ride in, and
/// "which twelve, in what order, on that day" stays answerable from history
/// alone.
///
/// The events are the REFERENCES, not the events: ids, positions and notes. A
/// snapshot that copied titles would be a copy of the chronology frozen in a
/// history row — the exact thing design §4 forbids, arriving through the back
/// door.
///
/// Pure — no I/O, so a test can assert the shape without a database.
pub fn snapshot_of(row: &ChronologySubsetRow, events: &[SubsetEventRefRow]) -> serde_json::Value {
    serde_json::json!({
        "id": row.id,
        "case_slug": row.case_slug,
        "name": row.name,
        "description": row.description,
        "created_by": row.created_by,
        "created_at": row.created_at,
        "updated_by": row.updated_by,
        "updated_at": row.updated_at,
        "deleted_at": row.deleted_at,
        "events": events
            .iter()
            .map(|e| serde_json::json!({
                "event_id": e.event_id,
                "position": e.position,
                "note": e.note,
            }))
            .collect::<Vec<_>>(),
    })
}

/// Why a sealed subset write could not be completed.
#[derive(Debug, thiserror::Error)]
pub enum SubsetSealError {
    /// The database refused something during the seal.
    #[error("{source}")]
    Repo {
        #[source]
        source: PipelineRepoError,
    },

    /// The subset vanished between the write and the snapshot.
    ///
    /// Inside one transaction this cannot happen, which is exactly why it is an
    /// error rather than an `Option` the caller shrugs at: reaching it means an
    /// assumption this module is built on has stopped holding, and the operator
    /// needs the id in the log, not a quiet empty response.
    #[error("timeline subset {subset_id} could not be read back inside its own write transaction")]
    Vanished { subset_id: Uuid },
}

impl From<PipelineRepoError> for SubsetSealError {
    fn from(source: PipelineRepoError) -> Self {
        SubsetSealError::Repo { source }
    }
}

/// Close a write: snapshot the subset and its ordered events, append ONE history
/// row, commit.
///
/// This is the only place a subset write transaction is committed, and the only
/// caller of `insert_subset_history`. Together those two facts are what make
/// "one history row per write" a property of the code rather than a habit of
/// whoever wrote the last handler.
///
/// Returns the subset and its references as they stand after the write — deleted
/// or not — which is what every write endpoint answers with, so a surface never
/// has to guess the new state from a status code.
///
/// ## Rust Learning: `&mut *tx` inside a consumed transaction
///
/// `tx` is owned here, and sqlx's queries want an executor. `&mut *tx`
/// re-borrows the transaction as a `&mut PgConnection` for the duration of one
/// query, leaving `tx` usable afterwards — which is what lets the same function
/// read, write and then `commit()`.
///
/// # Errors
/// [`SubsetSealError::Repo`] for any database failure, [`SubsetSealError::Vanished`]
/// if the subset cannot be read back. Either way the transaction is dropped
/// without a commit, so the write it was sealing is rolled back with it — a
/// change nobody could see the history of is never left behind.
pub async fn seal_and_commit(
    mut tx: sqlx::Transaction<'_, sqlx::Postgres>,
    subset_id: Uuid,
    action: SubsetHistoryAction,
    writer: &ChronologyWriter,
) -> Result<(ChronologySubsetRow, Vec<SubsetEventRefRow>), SubsetSealError> {
    let row = get_subset_any_state(&mut *tx, subset_id)
        .await?
        .ok_or(SubsetSealError::Vanished { subset_id })?;
    let events = list_subset_event_ids(&mut *tx, subset_id).await?;
    let snapshot = snapshot_of(&row, &events);
    insert_subset_history(
        &mut *tx,
        subset_id,
        action.as_str(),
        &snapshot,
        &writer.by_id,
    )
    .await?;
    tx.commit().await.map_err(PipelineRepoError::from)?;
    Ok((row, events))
}

#[cfg(test)]
#[path = "chronology_subset_guard_tests.rs"]
mod tests;
