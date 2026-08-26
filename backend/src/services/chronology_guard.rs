//! ⚑ THE ONE GUARDED WRITE PATH for the case chronology (Phase C, §C1).
//!
//! Phase C's instruction states three rules about every chronology write, and
//! this module is where all three are true in one place:
//!
//!  1. **It is not anonymous.** Reads stay open — looking at the chronology is
//!     not privileged (Phase A) — but a write requires an authenticated user.
//!     The refusal itself is axum's: every write handler takes `user: AuthUser`
//!     rather than `Option<AuthUser>`, so an anonymous request is a 401 before
//!     a handler body runs. [`open_write`] is what a handler does NEXT, and
//!     `api::timeline_write_guard_tests` proves no handler skips either half.
//!  2. **It stamps the acting user**, through the existing
//!     `services::practice_notes::attribution` helper — the tenth-plus stamped
//!     path in this codebase, not the first. Identity comes from the Authentik
//!     headers, never from a picker on a screen.
//!  3. **It writes exactly one history row**, holding a full snapshot of the
//!     event AFTER the write. [`seal_and_commit`] is the only way a chronology
//!     transaction is committed, so a write that recorded no history is not
//!     something a handler can forget — it is something a handler cannot
//!     express.
//!
//! ## Rust Learning: why the seal takes the transaction BY VALUE
//!
//! `seal_and_commit(mut tx, …)` consumes the `Transaction`. That is what makes
//! rule 3 structural rather than advisory: a handler that wanted to commit
//! without history would have to call `tx.commit()` itself, and it no longer
//! owns a `tx` to call it on — the seal took it. A `&mut Transaction` would have
//! left the handler holding the thing it must not use, and the rule would be
//! back to being a comment somebody has to read.
//!
//! ## Editing rights, and the one line that would narrow them
//!
//! Design R2: all three named users are equal, so enforcement here is
//! "authenticated", not role-gated. `colossus-auth` already exposes
//! `require_edit`, and gating this path on `legal_editor` would be one call
//! added to [`open_write`] — see the Phase C report's NEXT.

use uuid::Uuid;

use crate::auth::AuthUser;
use crate::repositories::pipeline_repository::chronology_write::{
    get_event_any_state, insert_history, ChronologyEventStateRow,
};
use crate::repositories::pipeline_repository::PipelineRepoError;
use crate::services::practice_notes::attribution;

/// What happened to an event, as `chronology_event_history.action` stores it.
///
/// ## Rust Learning: a fieldless enum in front of a SQL CHECK
///
/// The column carries a `CHECK (action IN ('created','updated','deleted',
/// 'restored'))`. A `&str` parameter would compile for any word and fail at the
/// database with a constraint violation — a 500, at write time, for a typo a
/// compiler could have caught. The enum makes the four the only spellable
/// values, and `the_action_words_match_the_check` pins them against the
/// migration file so the two lists cannot drift.
///
/// The DISPLAY word is not here. `updated` reads as "edited" on screen, and that
/// word is a stored settings row (`chronology_history_updated_label`) like every
/// other word this product speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryAction {
    Created,
    Updated,
    Deleted,
    Restored,
}

impl HistoryAction {
    /// The token the column stores.
    pub fn as_str(self) -> &'static str {
        match self {
            HistoryAction::Created => "created",
            HistoryAction::Updated => "updated",
            HistoryAction::Deleted => "deleted",
            HistoryAction::Restored => "restored",
        }
    }

    /// Every action, for the test that pins them against the migration.
    pub const ALL: &'static [HistoryAction] = &[
        HistoryAction::Created,
        HistoryAction::Updated,
        HistoryAction::Deleted,
        HistoryAction::Restored,
    ];
}

/// Who is writing. Built once per request, at the top of every write handler.
///
/// Two `String`s that always travel together, in the order every stamped column
/// in this codebase declares them: the stable id first, the display name second.
#[derive(Debug, Clone)]
pub struct ChronologyWriter {
    /// The Authentik username — the stable identity, stored in `created_by` /
    /// `updated_by` / `changed_by`.
    pub by_id: String,
    /// What a screen prints. Held because the same helper produces it and a
    /// caller that needed it would otherwise re-derive it differently.
    pub by: String,
}

/// Open a write: take the signature from the login.
///
/// There is deliberately nothing to fail here. The refusal that matters — an
/// anonymous request — has already happened in axum's extractor by the time this
/// runs, and putting a second, softer check in front of it would create a second
/// answer to "who may write", which is how two answers drift apart.
///
/// # Domain note
///
/// Design R2, changed 2026-08-25: THREE AUTHORS. Roman seeds the chronology, and
/// Chuck and Marie add, edit and delete from day one. There is no version of
/// this function that returns a different `ChronologyWriter` for one of them.
pub fn open_write(user: &AuthUser) -> ChronologyWriter {
    let (by_id, by) = attribution(user);
    ChronologyWriter { by_id, by }
}

/// The event as it stood after a write, in the shape history stores.
///
/// ## Why a SNAPSHOT and not a diff
///
/// Ruled 2026-08-25 (Phase A report, R-A). The change rule (design R4) says the
/// field set grows forever, so a typed history table would need a migration
/// every time an attribute was promoted, and would silently stop recording the
/// fields it did not know about. A snapshot never goes stale: reading history is
/// a diff between adjacent snapshots, computed at read time by code that knows
/// today's field set — not frozen into columns by the code that wrote it.
///
/// `deleted_at` is IN the snapshot, which is what makes a delete and a restore
/// distinguishable from each other by their content and not only by their
/// action word.
///
/// Pure — no I/O, so a test can assert the shape without a database.
pub fn snapshot_of(row: &ChronologyEventStateRow) -> serde_json::Value {
    serde_json::json!({
        "id": row.id,
        "case_slug": row.case_slug,
        "event_date": row.event_date,
        "date_precision": row.date_precision,
        "approximate": row.approximate,
        "phase": row.phase,
        "title": row.title,
        "fact": row.fact,
        "attributes": row.attributes,
        "created_by": row.created_by,
        "created_at": row.created_at,
        "updated_by": row.updated_by,
        "updated_at": row.updated_at,
        "deleted_at": row.deleted_at,
    })
}

/// Why a sealed write could not be completed.
#[derive(Debug, thiserror::Error)]
pub enum SealError {
    /// The database refused something during the seal.
    #[error("{source}")]
    Repo {
        #[source]
        source: PipelineRepoError,
    },

    /// The event vanished between the write and the snapshot.
    ///
    /// Inside one transaction this cannot happen, which is exactly why it is an
    /// error rather than an `Option` the caller shrugs at: reaching it means an
    /// assumption this module is built on has stopped holding, and the operator
    /// needs the event id in the log, not a quiet empty response.
    #[error("chronology event {event_id} could not be read back inside its own write transaction")]
    Vanished { event_id: Uuid },
}

impl From<PipelineRepoError> for SealError {
    fn from(source: PipelineRepoError) -> Self {
        SealError::Repo { source }
    }
}

/// Close a write: snapshot the event, append ONE history row, commit.
///
/// This is the only place a chronology write transaction is committed, and the
/// only caller of `insert_history`. Together those two facts are what make
/// "one history row per write" a property of the code rather than a habit of
/// whoever wrote the last handler.
///
/// Returns the event as it stands after the write — deleted or not — which is
/// what every write endpoint answers with, so a surface never has to guess the
/// new state from a status code (§C3: "the list/page reflects the server's
/// response — no optimistic divergence").
///
/// ## Rust Learning: `&mut *tx` inside a consumed transaction
///
/// `tx` is owned here, and sqlx's queries want an executor. `&mut *tx`
/// re-borrows the transaction as a `&mut PgConnection` for the duration of one
/// query, leaving `tx` usable afterwards — which is what lets the same function
/// read, write and then `commit()`.
///
/// # Errors
/// [`SealError::Repo`] for any database failure, [`SealError::Vanished`] if the
/// event cannot be read back. Either way the transaction is dropped without a
/// commit, so the write it was sealing is rolled back with it — a change nobody
/// could see the history of is never left behind.
pub async fn seal_and_commit(
    mut tx: sqlx::Transaction<'_, sqlx::Postgres>,
    event_id: Uuid,
    action: HistoryAction,
    writer: &ChronologyWriter,
) -> Result<ChronologyEventStateRow, SealError> {
    let row = get_event_any_state(&mut *tx, event_id)
        .await?
        .ok_or(SealError::Vanished { event_id })?;
    let snapshot = snapshot_of(&row);
    insert_history(
        &mut *tx,
        event_id,
        action.as_str(),
        &snapshot,
        &writer.by_id,
    )
    .await?;
    tx.commit().await.map_err(PipelineRepoError::from)?;
    Ok(row)
}

#[cfg(test)]
#[path = "chronology_guard_tests.rs"]
mod tests;
