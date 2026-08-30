//! Every statement that CHANGES a timeline subset, and nothing else.
//!
//! ## ⚑ THIS MODULE IS THE ONE WRITE PATH'S FLOOR
//!
//! `INSERT INTO chronology_subsets`, `UPDATE chronology_subsets`,
//! `INSERT INTO chronology_subset_events`, `DELETE FROM
//! chronology_subset_events`, `INSERT INTO chronology_subset_history`,
//! `INSERT INTO scenario_subsets` and `DELETE FROM scenario_subsets` each appear
//! in this crate exactly once, and every one of those appearances is below.
//! That is not tidiness — it is what T1.3 asks for ("every write is ONE call
//! into a service that is the only INSERT/UPDATE path for the three tables") —
//! and it is PROVED rather than asserted:
//! `services::chronology_subset_write::tests` strips this crate's comments and
//! fails if a second subset-writing statement exists anywhere outside this file.
//!
//! The guard that decides WHO may call these and what gets recorded when they
//! do lives one layer up, in `services::chronology_subset_guard`. This layer
//! knows SQL and nothing about authorisation: a repository that checked
//! permissions would be a second place to get permissions wrong.
//!
//! Every function takes `impl sqlx::PgExecutor<'_>` so the caller can hand it a
//! `&mut PgConnection` inside a transaction — which every caller does, because
//! every subset write is one transaction that also carries its history row.

use uuid::Uuid;

use super::PipelineRepoError;

/// Everything one new subset needs.
///
/// ## Rust Learning: a parameter struct instead of five positional arguments
///
/// Four of the five are `&str`, and swapping any two would compile and write the
/// wrong row — a subset named after its own description, stamped with its case
/// slug. Naming each field at the call site makes that class of mistake
/// impossible.
#[derive(Debug, Clone)]
pub struct NewChronologySubset<'a> {
    pub case_slug: &'a str,
    pub name: &'a str,
    pub description: &'a str,
    /// The Authentik username. Written to `created_by` AND `updated_by`: a row
    /// nobody has edited was last touched by whoever made it, and leaving
    /// `updated_by` empty would make "never edited" and "edited by nobody" the
    /// same value.
    pub by_id: &'a str,
}

/// Insert one subset and return the id the database generated.
///
/// No `ON CONFLICT`: the live-name uniqueness is a PARTIAL index, and this
/// codebase answers a name clash with a 409 the author reads rather than with a
/// silent no-op. The service checks first; the index is the backstop.
pub async fn insert_subset(
    executor: impl sqlx::PgExecutor<'_>,
    subset: &NewChronologySubset<'_>,
) -> Result<Uuid, PipelineRepoError> {
    let id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO chronology_subsets (case_slug, name, description, created_by, updated_by) \
         VALUES ($1, $2, $3, $4, $4) RETURNING id",
    )
    .bind(subset.case_slug)
    .bind(subset.name)
    .bind(subset.description)
    .bind(subset.by_id)
    .fetch_one(executor)
    .await?;
    Ok(id)
}

/// Rename and/or re-describe one live subset. Returns rows changed.
///
/// ## Why `COALESCE` and not two statements
///
/// An absent field means "leave it alone" (see `UpdateSubsetRequest`), and
/// `COALESCE($2, name)` says exactly that in one statement: a `NULL` parameter
/// keeps the stored value. Two statements would have been two chances to forget
/// `updated_by`, and a rename that did not restamp the row is a change with no
/// author.
///
/// `deleted_at IS NULL` in the predicate is what makes editing a deleted subset
/// return zero rather than quietly reviving it; the caller turns that into the
/// 409 that names Undo.
pub async fn update_subset(
    executor: impl sqlx::PgExecutor<'_>,
    id: Uuid,
    name: Option<&str>,
    description: Option<&str>,
    by_id: &str,
) -> Result<u64, PipelineRepoError> {
    let result = sqlx::query(
        "UPDATE chronology_subsets \
            SET name = COALESCE($2, name), \
                description = COALESCE($3, description), \
                updated_by = $4, updated_at = NOW() \
          WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(by_id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected())
}

/// SOFT-delete one subset (chronology R10). Returns rows changed.
pub async fn soft_delete_subset(
    executor: impl sqlx::PgExecutor<'_>,
    id: Uuid,
    by_id: &str,
) -> Result<u64, PipelineRepoError> {
    let result = sqlx::query(
        "UPDATE chronology_subsets \
            SET deleted_at = NOW(), updated_by = $2, updated_at = NOW() \
          WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .bind(by_id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected())
}

/// Restore one soft-deleted subset — the Undo. Returns rows changed.
pub async fn undelete_subset(
    executor: impl sqlx::PgExecutor<'_>,
    id: Uuid,
    by_id: &str,
) -> Result<u64, PipelineRepoError> {
    let result = sqlx::query(
        "UPDATE chronology_subsets \
            SET deleted_at = NULL, updated_by = $2, updated_at = NOW() \
          WHERE id = $1 AND deleted_at IS NOT NULL",
    )
    .bind(id)
    .bind(by_id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected())
}

/// Bump one subset's `updated_by` / `updated_at` without touching its fields.
///
/// The events-replace path calls this: the ordered set is the subset's content,
/// so changing it is an edit of the subset even though no column on
/// `chronology_subsets` changes. Without this, the picker's Save would leave the
/// list showing a stale "last touched by" — a write with no visible author,
/// which is the attribution half of Standing Rule 1.
pub async fn touch_subset(
    executor: impl sqlx::PgExecutor<'_>,
    id: Uuid,
    by_id: &str,
) -> Result<u64, PipelineRepoError> {
    let result = sqlx::query(
        "UPDATE chronology_subsets SET updated_by = $2, updated_at = NOW() \
          WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .bind(by_id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected())
}

/// Everything one event reference needs.
#[derive(Debug, Clone, Copy)]
pub struct NewSubsetEvent<'a> {
    pub subset_id: Uuid,
    pub event_id: Uuid,
    pub position: i32,
    pub note: &'a str,
    pub added_by: &'a str,
}

/// Add one event to a subset, or move/re-note one that is already there.
///
/// ## ⚑ Why `ON CONFLICT` names the PRIMARY KEY and not the position
///
/// `(subset_id, event_id)` is the primary key and is IMMEDIATE, so Postgres can
/// use it as the upsert's arbiter. `(subset_id, position)` is `DEFERRABLE
/// INITIALLY DEFERRED` — which is what lets a reorder pass through a moment
/// where two rows share a number — and a deferred constraint cannot be an
/// arbiter. The two roles are why the table carries both.
///
/// `added_by` / `added_at` are NOT restamped on conflict: they record when this
/// event first entered this story, which is a different fact from when the story
/// was last reordered. The subset's own `updated_by` carries the latter, via
/// [`touch_subset`].
pub async fn upsert_subset_event(
    executor: impl sqlx::PgExecutor<'_>,
    event: &NewSubsetEvent<'_>,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "INSERT INTO chronology_subset_events \
             (subset_id, event_id, position, note, added_by) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (subset_id, event_id) \
         DO UPDATE SET position = EXCLUDED.position, note = EXCLUDED.note",
    )
    .bind(event.subset_id)
    .bind(event.event_id)
    .bind(event.position)
    .bind(event.note)
    .bind(event.added_by)
    .execute(executor)
    .await?;
    Ok(())
}

/// Remove from one subset every reference NOT in `keep`. Returns rows removed.
///
/// A hard delete, and the one place in this feature where a hard delete is
/// right: an event's membership of a story is not content, it is a pointer the
/// author is presently drawing. The event is untouched, the subset is untouched,
/// and the history row this write lands carries the ordered list as it stood
/// before and after — so the removal is recoverable by reading, which is what
/// soft delete buys everywhere else.
///
/// `keep` empty removes them all, which is the honest meaning of "the picker
/// saved with nothing ticked".
pub async fn retain_subset_events(
    executor: impl sqlx::PgExecutor<'_>,
    subset_id: Uuid,
    keep: &[Uuid],
) -> Result<u64, PipelineRepoError> {
    let result = sqlx::query(
        "DELETE FROM chronology_subset_events \
          WHERE subset_id = $1 AND NOT (event_id = ANY($2))",
    )
    .bind(subset_id)
    .bind(keep)
    .execute(executor)
    .await?;
    Ok(result.rows_affected())
}

/// Append ONE history row. The only statement in this crate that writes subset
/// history.
///
/// ## ⚑ Append-only means append-only
///
/// There is no `update_subset_history` and no `delete_subset_history`, here or
/// anywhere. That is what makes a subset's history evidence rather than a field:
/// a delete is attributable and a reorder is reconstructible precisely because
/// the record of it cannot be tidied away by the person who made it.
///
/// `action` is one of the five the table's CHECK allows. It is typed at the
/// layer above (`services::chronology_subset_guard::SubsetHistoryAction`), so a
/// caller cannot spell one that the database will refuse.
pub async fn insert_subset_history(
    executor: impl sqlx::PgExecutor<'_>,
    subset_id: Uuid,
    action: &str,
    snapshot: &serde_json::Value,
    changed_by: &str,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "INSERT INTO chronology_subset_history (subset_id, action, snapshot, changed_by) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(subset_id)
    .bind(action)
    .bind(snapshot)
    .bind(changed_by)
    .execute(executor)
    .await?;
    Ok(())
}

/// Attach one subset to one scenario at the given position.
///
/// No `ON CONFLICT`: attaching what is already attached is a 409 the reader
/// sees, not a silent no-op, because the button that sent it is showing them a
/// list that is out of date. The service checks first; the primary key is the
/// backstop.
pub async fn attach_subset_to_scenario(
    executor: impl sqlx::PgExecutor<'_>,
    scenario_id: Uuid,
    subset_id: Uuid,
    position: i32,
    attached_by: &str,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "INSERT INTO scenario_subsets (scenario_id, subset_id, position, attached_by) \
         VALUES ($1, $2, $3, $4)",
    )
    .bind(scenario_id)
    .bind(subset_id)
    .bind(position)
    .bind(attached_by)
    .execute(executor)
    .await?;
    Ok(())
}

/// Detach one subset from one scenario. Returns rows removed.
///
/// ## ⚑ THE ONE HARD DELETE IN THIS FEATURE, and why
///
/// A link is not content. Detaching removes a pointer from a scenario to a
/// story; the story, its events, its notes and its whole history are untouched
/// and one click from being re-attached. There is nothing here that a soft
/// delete would preserve — no words somebody wrote, no attribution that would be
/// lost — so a `deleted_at` on this table would be a column that only ever made
/// the reads harder to write.
pub async fn detach_subset_from_scenario(
    executor: impl sqlx::PgExecutor<'_>,
    scenario_id: Uuid,
    subset_id: Uuid,
) -> Result<u64, PipelineRepoError> {
    let result =
        sqlx::query("DELETE FROM scenario_subsets WHERE scenario_id = $1 AND subset_id = $2")
            .bind(scenario_id)
            .bind(subset_id)
            .execute(executor)
            .await?;
    Ok(result.rows_affected())
}
