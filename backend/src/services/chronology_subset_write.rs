//! ⚑ THE ONE GUARDED WRITE PATH for timeline subsets (T1.3).
//!
//! Every write to `chronology_subsets`, `chronology_subset_events`,
//! `scenario_subsets` and `chronology_subset_history` goes through exactly one
//! function in this module, and every one of those functions owns its whole
//! transaction from `begin` to the seal. A handler's job is to fence the request
//! and judge it; it never opens a transaction of its own and never touches the
//! repository's write module.
//!
//! ## What each operation is, in one line
//!
//! | function | what a person did | history row |
//! |---|---|---|
//! | [`create`] | named a story and (maybe) filled it | `created` |
//! | [`rename`] | edited the name and/or description | `updated` |
//! | [`replace_events`] | pressed Save in the picker | `events_replaced` |
//! | [`soft_delete`] | pressed Delete | `deleted`, or none — see below |
//! | [`restore`] | pressed Undo | `restored`, or none — see below |
//! | [`attach`] | pointed a scenario at a story | none — see below |
//! | [`detach`] | stopped pointing | none — see below |
//!
//! Attach and detach write no subset history: they change the SCENARIO's fact
//! about the subset, not the subset's content. `chronology_subset_guard`'s
//! header states the choice; the T1 report records it as a decision.
//!
//! [`soft_delete`] and [`restore`] write no history row when the subset was
//! ALREADY in the state being asked for — they answer `Ok(None)` instead. That
//! is not the same silence: it says the act did not happen, so recording one
//! would be a false entry in a table nothing can correct. Both functions decide
//! it from the rows their own `UPDATE` matched, inside the transaction, which is
//! the only place the answer cannot go stale.
//!
//! ## Rust Learning: `async fn` that owns a transaction end to end
//!
//! Each function below takes `&PgPool`, opens a transaction, does its work and
//! hands the transaction to `seal_and_commit`, which consumes it. Nothing
//! partial can escape: an early `?` drops the `Transaction`, and sqlx rolls a
//! dropped transaction back. That is why there is no explicit rollback anywhere
//! in this file — the type system already wrote it.

use sqlx::PgPool;
use uuid::Uuid;

use crate::repositories::pipeline_repository::chronology_subset_write::{
    attach_subset_to_scenario, detach_subset_from_scenario, insert_subset, retain_subset_events,
    soft_delete_subset, touch_subset, undelete_subset, update_subset, upsert_subset_event,
    NewChronologySubset, NewSubsetEvent,
};
use crate::repositories::pipeline_repository::chronology_subsets::{
    ChronologySubsetRow, SubsetEventRefRow,
};
use crate::repositories::pipeline_repository::PipelineRepoError;
use crate::services::chronology_guard::ChronologyWriter;
use crate::services::chronology_subset_guard::{
    seal_and_commit, SubsetHistoryAction, SubsetSealError,
};
use crate::services::chronology_subset_validate::ValidSubsetEvent;

/// What every sealed operation in this module answers with: the subset as the
/// server now holds it, and its ordered references.
///
/// A named type rather than a bare tuple because three call sites destructure it
/// and `(row, events)` at the third one is a pair of positions a reader has to
/// remember.
#[derive(Debug, Clone)]
pub struct WrittenSubset {
    pub subset: ChronologySubsetRow,
    pub events: Vec<SubsetEventRefRow>,
}

/// Why a subset write could not be made.
///
/// Distinct from the pure [`SubsetWriteRefusal`][refusal] on purpose: that one
/// is about the REQUEST and is decided without a database, this one is about
/// what happened when the write was attempted. The API layer maps them to
/// different statuses.
///
/// [refusal]: crate::services::chronology_subset_validate::SubsetWriteRefusal
#[derive(Debug, thiserror::Error)]
pub enum SubsetWriteError {
    #[error("{source}")]
    Repo {
        #[source]
        source: PipelineRepoError,
    },

    #[error("{source}")]
    Seal {
        #[source]
        source: SubsetSealError,
    },

    /// The subset was live when it was read and is not now — somebody deleted it
    /// between the check and the write.
    ///
    /// Reported rather than sealed: a history row saying "edited" over a subset
    /// nobody edited would be a false record.
    #[error("timeline subset {subset_id} is deleted; restore it with Undo before changing it")]
    Deleted { subset_id: Uuid },
}

impl From<PipelineRepoError> for SubsetWriteError {
    fn from(source: PipelineRepoError) -> Self {
        SubsetWriteError::Repo { source }
    }
}

impl From<SubsetSealError> for SubsetWriteError {
    fn from(source: SubsetSealError) -> Self {
        SubsetWriteError::Seal { source }
    }
}

/// Create one subset, and the ordered references it arrived with.
///
/// ONE transaction and ONE history row, so "name a story and pick its twelve
/// events" is one act in the record rather than two — which is what the picker
/// does in one press.
pub async fn create(
    pool: &PgPool,
    case_slug: &str,
    name: &str,
    description: &str,
    events: &[ValidSubsetEvent],
    writer: &ChronologyWriter,
) -> Result<WrittenSubset, SubsetWriteError> {
    let mut tx = pool.begin().await.map_err(PipelineRepoError::from)?;
    let id = insert_subset(
        &mut *tx,
        &NewChronologySubset {
            case_slug,
            name,
            description,
            by_id: &writer.by_id,
        },
    )
    .await?;
    write_refs(&mut tx, id, events, &writer.by_id).await?;
    let (subset, events) = seal_and_commit(tx, id, SubsetHistoryAction::Created, writer).await?;
    Ok(WrittenSubset { subset, events })
}

/// Edit one live subset's name and/or description.
///
/// A `None` leaves the stored value alone — the repository's `COALESCE` says so
/// in SQL, and the wire shape says so in `UpdateSubsetRequest`. A request with
/// both absent is a legal no-op that still lands a history row, because somebody
/// pressed Save and the record is of acts, not of diffs.
pub async fn rename(
    pool: &PgPool,
    subset_id: Uuid,
    name: Option<&str>,
    description: Option<&str>,
    writer: &ChronologyWriter,
) -> Result<WrittenSubset, SubsetWriteError> {
    let mut tx = pool.begin().await.map_err(PipelineRepoError::from)?;
    let changed = update_subset(&mut *tx, subset_id, name, description, &writer.by_id).await?;
    if changed == 0 {
        return Err(SubsetWriteError::Deleted { subset_id });
    }
    let (subset, events) =
        seal_and_commit(tx, subset_id, SubsetHistoryAction::Updated, writer).await?;
    Ok(WrittenSubset { subset, events })
}

/// REPLACE one subset's ordered event set — the picker's Save.
///
/// ## Why replace and not add/remove/reorder
///
/// T1.3: "there is no per-row add/remove endpoint (one write, one snapshot)".
/// The picker is a screen where somebody ticks, unticks and drags for a minute
/// and then presses Save once. Modelling that as a stream of small writes would
/// put a dozen history rows behind one human act, and would leave the subset
/// legal-but-wrong at every intermediate step — the exact state a reader on
/// another screen would happen to load.
///
/// Rows not in `events` are removed; the rest are upserted, keeping their
/// original `added_by` / `added_at`. The deferred position constraint is what
/// lets the upserts run in any order without a shuffle — see the migration.
pub async fn replace_events(
    pool: &PgPool,
    subset_id: Uuid,
    events: &[ValidSubsetEvent],
    writer: &ChronologyWriter,
) -> Result<WrittenSubset, SubsetWriteError> {
    let mut tx = pool.begin().await.map_err(PipelineRepoError::from)?;
    // The subset's own row is restamped first, and its zero-rows answer is what
    // refuses a replace onto a deleted subset — one statement doing the fence
    // and the attribution, rather than a separate SELECT that could go stale
    // between the read and the write.
    let touched = touch_subset(&mut *tx, subset_id, &writer.by_id).await?;
    if touched == 0 {
        return Err(SubsetWriteError::Deleted { subset_id });
    }
    let keep: Vec<Uuid> = events.iter().map(|e| e.event_id).collect();
    retain_subset_events(&mut *tx, subset_id, &keep).await?;
    write_refs(&mut tx, subset_id, events, &writer.by_id).await?;
    let (subset, events) =
        seal_and_commit(tx, subset_id, SubsetHistoryAction::EventsReplaced, writer).await?;
    Ok(WrittenSubset { subset, events })
}

/// SOFT-delete one subset (chronology R10). No confirm dialog anywhere: the undo
/// line that replaces the row in place IS the safety.
///
/// Detaches nothing. The `scenario_subsets` rows stay exactly as they were, and
/// the scenario reads simply do not see a deleted subset — so an Undo brings the
/// attachment back with it, which a detach-on-delete could not have done.
///
/// ## Rust Learning: `Option` in the OK position, as a THIRD answer
///
/// The return is `Result<Option<WrittenSubset>, _>`, which spells three
/// outcomes where two would not be enough: `Err` is "it broke", `Ok(Some(..))`
/// is "the delete happened and its history row was written", and `Ok(None)` is
/// "there was nothing to delete, so nothing was recorded". Collapsing that
/// third case into the second is exactly what would let a false `deleted` row
/// reach an append-only table — see the next paragraph.
///
/// ## ⚑ Why the zero-rows check is INSIDE the transaction
///
/// `delete_subset` already refuses an already-deleted subset before it calls
/// here — but that check runs on its own connection, before this transaction
/// opens. Between the two, another request can delete the same subset. The
/// `UPDATE ... WHERE deleted_at IS NULL` then matches nothing, while
/// [`seal_and_commit`] — which reads the row back with `get_subset_any_state`,
/// ignoring delete state — would still append a `deleted` history row for a
/// delete THIS request did not perform. History is a record of acts, and it has
/// no correction path: there is no `delete_subset_history` and there must not
/// be one. [`rename`] and [`replace_events`] close the same race the same way;
/// this is that guard, not a new idea.
pub async fn soft_delete(
    pool: &PgPool,
    subset_id: Uuid,
    writer: &ChronologyWriter,
) -> Result<Option<WrittenSubset>, SubsetWriteError> {
    let mut tx = pool.begin().await.map_err(PipelineRepoError::from)?;
    let deleted = soft_delete_subset(&mut *tx, subset_id, &writer.by_id).await?;
    if deleted == 0 {
        // Dropping `tx` rolls back — this module's header explains why there is
        // no explicit rollback anywhere in this file. No history row is written,
        // and `None` is how the caller learns the act did not happen.
        return Ok(None);
    }
    let (subset, events) =
        seal_and_commit(tx, subset_id, SubsetHistoryAction::Deleted, writer).await?;
    Ok(Some(WrittenSubset { subset, events }))
}

/// Restore one soft-deleted subset — the Undo.
///
/// `Ok(None)` when the subset was already live, for the reason [`soft_delete`]
/// gives at length: a `restored` row for a restore that did not happen is the
/// same false record, and the same race produces it.
pub async fn restore(
    pool: &PgPool,
    subset_id: Uuid,
    writer: &ChronologyWriter,
) -> Result<Option<WrittenSubset>, SubsetWriteError> {
    let mut tx = pool.begin().await.map_err(PipelineRepoError::from)?;
    let restored = undelete_subset(&mut *tx, subset_id, &writer.by_id).await?;
    if restored == 0 {
        return Ok(None);
    }
    let (subset, events) =
        seal_and_commit(tx, subset_id, SubsetHistoryAction::Restored, writer).await?;
    Ok(Some(WrittenSubset { subset, events }))
}

/// Attach one subset to one scenario, appended at `position`.
///
/// No transaction and no seal, and both absences are deliberate: this is one
/// INSERT of one link row, and a link is not subset content (see the guard's
/// header). It lives in this module anyway, because "the only INSERT path for
/// the three tables" has to mean all three.
pub async fn attach(
    pool: &PgPool,
    scenario_id: Uuid,
    subset_id: Uuid,
    position: i32,
    writer: &ChronologyWriter,
) -> Result<(), SubsetWriteError> {
    attach_subset_to_scenario(pool, scenario_id, subset_id, position, &writer.by_id).await?;
    Ok(())
}

/// Detach one subset from one scenario. Returns rows removed, so the caller can
/// tell "it was attached and now is not" from "it was never attached".
pub async fn detach(
    pool: &PgPool,
    scenario_id: Uuid,
    subset_id: Uuid,
) -> Result<u64, SubsetWriteError> {
    Ok(detach_subset_from_scenario(pool, scenario_id, subset_id).await?)
}

/// Write one ordered set of references inside an open transaction.
///
/// Shared by [`create`] and [`replace_events`] because they do the same thing
/// with the same rules, and a second copy would be the place one of them
/// eventually forgot to stamp `added_by`.
///
/// ## Rust Learning: `&mut Transaction` here, by value at the seal
///
/// This helper BORROWS the transaction, so the caller keeps ownership and can
/// still hand it to `seal_and_commit` afterwards. The seal takes it by value
/// precisely because nothing may follow it — the difference between the two
/// signatures is the difference between "does some work in this transaction" and
/// "ends this transaction".
async fn write_refs(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    subset_id: Uuid,
    events: &[ValidSubsetEvent],
    by_id: &str,
) -> Result<(), SubsetWriteError> {
    for event in events {
        upsert_subset_event(
            &mut **tx,
            &NewSubsetEvent {
                subset_id,
                event_id: event.event_id,
                position: event.position,
                note: &event.note,
                added_by: by_id,
            },
        )
        .await?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "chronology_subset_write_tests.rs"]
mod tests;
