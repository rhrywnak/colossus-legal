//! The two `scan_runs` STATE questions the server owns a scan with: "is one
//! already running here?" and "stop this one".
//!
//! ## Why a new module rather than more functions on `scan_runs.rs`
//!
//! `scan_runs.rs` is at 289 counted lines against the 300-line limit
//! (CLAUDE.md rule 17), so two more functions with the doc comments this repo
//! expects would push it over. The split is also thematic rather than
//! arbitrary, exactly as `scan_run_verdicts.rs` and `scan_run_projection.rs`
//! were: that module owns a run's LIFECYCLE — the states a run moves itself
//! through as it works (stub → running → finalize/fail) — while these two reads
//! and one write are about a run's state as seen from OUTSIDE, by a second
//! request arriving while the first is still going.
//!
//! `scan_runs.rs` remains the module to read for the run lifecycle; start there.
//!
//! CRITICAL: `scan_runs` lives in the pipeline database (`colossus_legal_v2`),
//! so callers pass `&state.pipeline_pool`.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::scan_runs::SCAN_STATUS_RUNNING;
use super::PipelineRepoError;

/// A run a HUMAN stopped mid-flight (task SCAN_SERVER_STATE, part C).
///
/// Terminal, like `completed` and `failed`, and deliberately none of them. It is
/// not `completed` — the run did not judge its whole pool and its counts are a
/// partial tally, so calling it complete would be the "104 judged · 0 relevant"
/// lie in a different costume. It is not `failed` — nothing went wrong, and the
/// startup sweep's "interrupted by restart" would name a cause that did not
/// happen. Standing Rule 1: three operationally distinct endings, three tokens.
///
/// It lives HERE rather than beside the other three in `scan_runs.rs` for the
/// reason that module names: it was at the 300-line limit, and this const belongs
/// with [`cancel_scan_run`], the only thing that writes it.
// STRUCTURAL: a DATABASE STATUS TOKEN. The write path (`cancel_scan_run`), the
// projecting-run query and the browser's `ScanRunState` union must all agree on
// this spelling; a deployment that varied it would write rows its own projection
// could not find. Not configuration — changing it is a data-format change.
pub(crate) const SCAN_STATUS_CANCELLED: &str = "cancelled";

/// The scenario's in-flight run, as the refusal needs to describe it.
///
/// Carries `started_at` so the caller can log how long the run the human is
/// colliding with has been going — the first question anybody asks when told
/// "a scan is already running".
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RunningScanRow {
    pub run_id: Uuid,
    pub started_at: DateTime<Utc>,
}

// CONST: the in-flight lookup, held as a `const` for the house SQL-shape test
// pattern (see `LIST_SCAN_RUNS_SQL`). Query text, not config — Rule 13 N/A.
//
// `LIMIT 1` is not a tie-break: since the `scan_runs_one_running_per_scenario`
// partial unique index (migration 20260910142419) there can be at most one
// matching row, and the LIMIT says out loud that the caller wants THE running
// run rather than a list. `ORDER BY started_at DESC` is there for the same
// reason — so that on a database built before the index, this still answers with
// the newest of any strays instead of an arbitrary one.
const RUNNING_SCAN_SQL: &str = "SELECT run_id, started_at \
     FROM scan_runs WHERE scenario_id = $1 AND status = $2 \
     ORDER BY started_at DESC LIMIT 1";

/// The scenario's currently-running scan, or `None` when nothing is in flight.
///
/// `None` is the ordinary answer and is not a gap: most POSTs arrive at an idle
/// scenario. A failure to READ is a different thing entirely and propagates —
/// the caller must never treat an unreadable check as permission to start a
/// second scan, because that fails in the expensive direction (two background
/// jobs judging the same pool, twice the LLM spend).
///
/// # Errors
/// Returns [`PipelineRepoError`] if the query fails.
pub async fn find_running_scan_run(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Option<RunningScanRow>, PipelineRepoError> {
    let row = sqlx::query_as::<_, RunningScanRow>(RUNNING_SCAN_SQL)
        .bind(scenario_id)
        .bind(SCAN_STATUS_RUNNING)
        .fetch_optional(pool)
        .await?;
    Ok(row)
}

// CONST: the cancel write. Query text, not config — Rule 13 N/A.
//
// Three things this statement deliberately does NOT do:
//
//   * it does not touch the counts. `relevant_count`, `irrelevant_count`,
//     `failed_count` and `candidates_judged` are where `bump_scan_run_progress`
//     left them, and that is the honest record of what the run judged before it
//     was stopped. Zeroing them would erase work that was really done and paid
//     for; recomputing them would need a tally this statement does not have.
//   * it does not write `summary_json`. A stopped run has no finished summary,
//     and `NULL` there is what tells the poll and the history that there is no
//     report to render (Standing Rule 1 — absent, not fabricated).
//   * it does not write an `error`. It CLEARS one, because nothing went wrong.
//     A stopped run carrying "interrupted by restart" would name a cause that
//     did not happen.
//
// `WHERE ... AND status = $3` is the guard that makes this safe to call from a
// background task racing its own finalize: a run that already completed, failed
// or was cancelled matches zero rows, and the caller sees that in the count
// rather than overwriting a terminal state with a later one.
const CANCEL_SCAN_RUN_SQL: &str = "UPDATE scan_runs \
     SET status = $2, error = NULL, last_progress_at = NOW() \
     WHERE run_id = $1 AND status = $3";

/// Record a run as `cancelled`, keeping its counts exactly as they stand.
///
/// Returns the number of rows updated so the caller can distinguish "stopped it"
/// (`1`) from "it was no longer running" (`0`) — the same found/not-found signal
/// [`super::scan_runs::delete_scan_run`] uses, and for the same reason: a silent
/// success would tell a human a scan was stopped when something else had already
/// settled it.
///
/// The one legitimate writer is the judging task itself, on the way out of a
/// cancelled loop. The cancel ROUTE only signals the token; it never writes this
/// row. One writer means the counts on a `cancelled` row are always the counts
/// the loop actually reached, with no window in which a route's write and a
/// task's last progress bump disagree.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the update fails.
pub async fn cancel_scan_run(pool: &PgPool, run_id: Uuid) -> Result<u64, PipelineRepoError> {
    let result = sqlx::query(CANCEL_SCAN_RUN_SQL)
        .bind(run_id)
        .bind(SCAN_STATUS_CANCELLED)
        .bind(SCAN_STATUS_RUNNING)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// What [`promote_scan_run_running`] found when it tried to flip the stub.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromoteOutcome {
    /// The row was in its birth state and is now `running`.
    Promoted,
    /// The UPDATE matched zero rows: the stub is gone, or something else already
    /// moved it out of `failed`. The caller refuses rather than judging blind.
    NotPromotable,
    /// The scenario already has a `running` run — the partial unique index
    /// refused the second one. NOT an error: it is the same answer the
    /// pre-check gives, arriving through the backstop instead.
    AlreadyRunning,
}

/// The index whose violation means "this scenario already has a running scan".
///
/// Named as a const because it appears in two places that must agree — the
/// migration that creates it and the classifier below — and a typo here would
/// silently downgrade a clean 409 into a 500 (`is_one_running_violation` would
/// simply never match).
// STRUCTURAL: a SCHEMA IDENTIFIER — the name migration 20260910142419 gives the
// index, and the string Postgres reports back in the violation. It is fixed by
// that migration in every environment; an operator cannot change it without
// editing a version-controlled migration, at which point this const moves too.
const ONE_RUNNING_INDEX: &str = "scan_runs_one_running_per_scenario";

/// Is this sqlx error the one-running-per-scenario index refusing a second run?
///
/// Deliberately narrow: it matches on the CONSTRAINT NAME, not merely on the
/// unique-violation SQLSTATE. Any other unique violation on this table would be a
/// genuine bug and must keep travelling as a 500 rather than being answered with
/// a cheerful "a scan is already running" that is not true.
///
/// ## Rust Learning: `as_database_error()` and the `DatabaseError` trait
///
/// `sqlx::Error` is a broad enum (pool timeouts, decode failures, protocol
/// errors); only some variants carry a message the SERVER sent.
/// `as_database_error()` returns `Option<&dyn DatabaseError>` — `Some` only for
/// those — and the trait exposes the driver-independent bits, of which
/// `constraint()` is the one that answers this question exactly. Everything else
/// falls through to `false` via `is_some_and`, so no non-database error can be
/// mistaken for the invariant firing.
///
/// `pub(super)` rather than private: the only caller is
/// [`super::scan_runs::promote_scan_run_running`], which is where the UPDATE that
/// can trip the index lives — the classifier followed `PromoteOutcome` here when
/// `scan_runs.rs` reached the size limit, not because anything else needs it.
pub(super) fn is_one_running_violation(e: &sqlx::Error) -> bool {
    e.as_database_error()
        .and_then(|db| db.constraint())
        .is_some_and(|name| name == ONE_RUNNING_INDEX)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Both laws below are enforced by the SQL, so they are asserted against the
    // statement rather than a returned value — the house pattern for a rule that
    // lives in a query (see `scan_run_projection`'s tests and `documents_delete`).
    // A unit test cannot run a query; it can prove the query cannot express the
    // wrong thing.

    #[test]
    fn the_in_flight_lookup_is_fenced_to_one_scenario_and_to_running() {
        // A lookup that forgot the scenario fence would refuse a scan because some
        // OTHER scenario was busy — a scan the human cannot start and cannot see a
        // reason for. Both binds are the whole point of the read.
        assert!(
            RUNNING_SCAN_SQL.contains("scenario_id = $1"),
            "the in-flight lookup must be fenced to its scenario: {RUNNING_SCAN_SQL}"
        );
        assert!(
            RUNNING_SCAN_SQL.contains("status = $2"),
            "the in-flight lookup must gate on status: {RUNNING_SCAN_SQL}"
        );
        assert_eq!(
            SCAN_STATUS_RUNNING, "running",
            "the bound status is the one `promote_scan_run_running` writes"
        );
    }

    /// The index name the classifier matches is the one the MIGRATION creates.
    ///
    /// A disk/code consistency check (CLAUDE.md rule 21), and it is stronger than
    /// pinning the literal against itself would be: the string only earns its keep
    /// by matching what Postgres reports, and what Postgres reports is decided by
    /// a file on disk. A typo in either — or a migration renaming the index —
    /// makes `is_one_running_violation` silently never match, which downgrades
    /// every concurrent-start collision from a clean 409 to a 500 about a
    /// constraint the human has no way to understand. Nothing else would notice:
    /// the code compiles, the pre-check still works, and the failure needs two
    /// POSTs in the same millisecond to appear.
    #[test]
    fn the_index_name_is_the_one_the_migration_creates() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("pipeline_migrations/20260910142419_scan_runs_one_running_per_scenario.sql");
        let sql = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("the migration is readable at {}: {e}", path.display()));

        assert!(
            sql.contains(&format!(
                "CREATE UNIQUE INDEX IF NOT EXISTS {ONE_RUNNING_INDEX}"
            )),
            "the migration must create the index this classifier matches on \
             (`{ONE_RUNNING_INDEX}`)"
        );
        // …and it must be the PARTIAL index, not a plain unique one: a unique
        // index on `scenario_id` alone would forbid a scenario from EVER being
        // scanned twice.
        assert!(
            sql.contains("WHERE status = 'running'"),
            "the index must be partial — a bare unique index on scenario_id would \
             allow one scan per scenario, ever"
        );
    }

    #[test]
    fn the_cancel_write_clears_the_error_and_touches_no_count() {
        assert!(
            CANCEL_SCAN_RUN_SQL.contains("error = NULL"),
            "a stopped run failed at nothing and must carry no reason: {CANCEL_SCAN_RUN_SQL}"
        );
        // The counts are the record of work really done before the stop. Naming any
        // of them in this UPDATE would either erase that work or invent a number.
        for column in [
            "relevant_count",
            "irrelevant_count",
            "failed_count",
            "candidates_judged",
            "summary_json",
        ] {
            assert!(
                !CANCEL_SCAN_RUN_SQL.contains(column),
                "cancel must leave `{column}` exactly as the run left it: {CANCEL_SCAN_RUN_SQL}"
            );
        }
    }

    #[test]
    fn the_cancel_write_can_only_move_a_running_run() {
        // Without this fence the cancel route could overwrite a run that finished
        // half a second earlier, turning a real completed report into a `cancelled`
        // row with no summary — the record destroyed by the click that arrived late.
        assert!(
            CANCEL_SCAN_RUN_SQL.contains("WHERE run_id = $1 AND status = $3"),
            "cancel must be fenced to the run AND to the running state: {CANCEL_SCAN_RUN_SQL}"
        );
        assert_eq!(
            SCAN_STATUS_CANCELLED, "cancelled",
            "the recorded status is the one the projection and the panel read"
        );
    }
}
