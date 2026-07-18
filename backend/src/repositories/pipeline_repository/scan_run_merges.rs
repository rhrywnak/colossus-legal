//! Repository for the `scan_run_merges` audit table in the `colossus_legal_v2`
//! pipeline database — one row per scan-run → scenario **merge event**.
//!
//! ## Why a whole module for one insert
//!
//! `scan_runs.rs` is already near the Rule 17 size limit, and the merge-event
//! write has a distinct owner (the Merge/set-as-basis service) from the scan
//! lifecycle writes. Splitting it keeps each module focused — the same discipline
//! that put `scan_run_verdicts`' reads beside its parent rather than in a third
//! file only when they grew.
//!
//! ## The event model (why COUNT/MAX, not a boolean)
//!
//! Re-merge is legitimate (the reconcile is status-preserving), so a run can be
//! merged more than once over time. This module records EACH merge as its own
//! row; the run detail then reads `COUNT(*)` + `MAX(merged_at)` per run (see
//! `scan_runs::LIST_SCAN_RUNS_SQL`) to show "merged N×, last at …". A single
//! `merged_at`/boolean column could not represent that history — which is exactly
//! why the audit table exists.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{merge_scan_run_into_scenario, PipelineRepoError};

/// The fields recorded for one merge event.
///
/// Built by the caller (the Merge service) and passed by reference to
/// [`insert_scan_run_merge`]. Every field is non-optional: a merge event always
/// knows its run, its scenario, when it happened, and how many rows it applied.
#[derive(Debug, Clone)]
pub struct ScanRunMergeRecord {
    /// Application-generated UUID (minted in Rust before the INSERT, house
    /// pattern — see `scan_runs.run_id`).
    pub merge_id: Uuid,
    pub run_id: Uuid,
    pub scenario_id: Uuid,
    /// Bound from `Utc::now()` at the call site (no DB default), matching the
    /// `scan_runs.started_at` house pattern — the application owns the timestamp.
    pub merged_at: DateTime<Utc>,
    /// Candidate-fact rows the merge inserted/refreshed as undecided (the count
    /// the merge endpoint returns). A recorded `0` is a real event, distinct from
    /// "never merged" (no row) — Standing Rule 1.
    pub rows_affected: i32,
}

// CONST: the merge-event INSERT, held as a `const` so a SQL-shape unit test can
// pin the column list without a live database (house pattern — mirrors
// `scan_runs::LIST_SCAN_RUNS_SQL` / `DELETE_SCAN_RUN_SQL`). Query text, not
// deployment config, so Rule 13 does not apply.
const INSERT_SCAN_RUN_MERGE_SQL: &str = "INSERT INTO scan_run_merges \
     (merge_id, run_id, scenario_id, merged_at, rows_affected) \
     VALUES ($1, $2, $3, $4, $5)";

/// Record one merge event.
///
/// ## Rust Learning: `impl sqlx::PgExecutor<'_>` so this can run inside a caller's transaction
///
/// The parameter is a generic executor, not a concrete `&PgPool`. That lets the
/// Merge service pass a `&mut *tx` here so this INSERT and the merge
/// `INSERT … SELECT` commit together (or roll back together) — the merge and its
/// provenance are one atomic unit. The `'_` is the elided lifetime of whatever
/// executor is borrowed. Same generic-executor pattern as
/// `scenario_store::reconcile_fact_ref`. Passing a plain `&PgPool` still works
/// (it also implements `PgExecutor`) for a standalone write.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the INSERT fails. Inside a transaction that
/// error propagates to the caller, which drops the `Transaction` without
/// committing — so a failed event record also aborts the merge (no half-merge).
pub async fn insert_scan_run_merge(
    executor: impl sqlx::PgExecutor<'_>,
    record: &ScanRunMergeRecord,
) -> Result<(), PipelineRepoError> {
    sqlx::query(INSERT_SCAN_RUN_MERGE_SQL)
        .bind(record.merge_id)
        .bind(record.run_id)
        .bind(record.scenario_id)
        .bind(record.merged_at)
        .bind(record.rows_affected)
        .execute(executor)
        .await?;
    Ok(())
}

/// Narrow a `u64` merge row-count to the `INTEGER` column. A merge never applies
/// anywhere near `i32::MAX` rows (the ~94-candidate ceiling), so the impossible
/// overflow is logged and capped rather than silently wrapping (Standing Rule 1).
/// Kept local to the repo (rather than reusing the service's `count_to_i32`) so
/// this module does not reach up into the service layer for a one-line narrow.
fn narrow_rows_affected(merged: u64) -> i32 {
    i32::try_from(merged).unwrap_or_else(|_| {
        tracing::error!(
            value = merged,
            "scan_run_merges: rows_affected exceeded i32 — capped"
        );
        i32::MAX
    })
}

/// Merge one stored run's relevant picks into a scenario AND record the merge
/// event, atomically in ONE transaction.
///
/// This is the provenance-aware merge the workbench calls: it wraps the existing
/// status-preserving merge ([`merge_scan_run_into_scenario`], SQL unchanged) and
/// the [`insert_scan_run_merge`] audit write in a single `pool.begin()` boundary,
/// so a merge is never applied-without-recording or recorded-without-applying.
///
/// ## Rust Learning: transactions live in the repository layer here
///
/// `pool.begin()` yields a `Transaction` owning one connection; passing `&mut *tx`
/// (a reborrow) to each write threads BOTH onto that connection so they share a
/// commit boundary. Any `?` before `commit()` drops the transaction → ROLLBACK, so
/// a failed event-insert also aborts the merge (Standing Rule 1 — no half-merge).
/// Owning the transaction here (not in the service) matches the house pattern —
/// `insert_scan_run_verdicts` and `documents_delete` also `begin()` in the repo.
///
/// `merged_at` is passed in (bound from the service's `Utc::now()`) so the caller
/// owns the timestamp, matching `scan_runs.started_at`. Returns the merged row
/// count (the number of undecided suggestions inserted/refreshed).
///
/// # Errors
/// [`PipelineRepoError`] if `begin`, either write, or `commit` fails — the caller
/// maps it to its user-facing merge error.
pub async fn merge_run_into_scenario_recording(
    pool: &PgPool,
    scenario_id: Uuid,
    run_id: Uuid,
    merged_at: DateTime<Utc>,
) -> Result<u64, PipelineRepoError> {
    let mut tx = pool.begin().await?;

    let merged = merge_scan_run_into_scenario(&mut *tx, scenario_id, run_id).await?;

    let record = ScanRunMergeRecord {
        merge_id: Uuid::new_v4(),
        run_id,
        scenario_id,
        merged_at,
        rows_affected: narrow_rows_affected(merged),
    };
    insert_scan_run_merge(&mut *tx, &record).await?;

    tx.commit().await?;
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin the INSERT column list: the write must name exactly the five audit
    /// columns, in the order the binds supply them. A SQL-shape test (no live DB)
    /// mirroring `scan_runs`' const-SQL tests — it catches a column rename or a
    /// bind/column drift that a clean compile would not.
    #[test]
    fn insert_sql_names_the_five_audit_columns() {
        for col in [
            "merge_id",
            "run_id",
            "scenario_id",
            "merged_at",
            "rows_affected",
        ] {
            assert!(
                INSERT_SCAN_RUN_MERGE_SQL.contains(col),
                "INSERT must name the {col} column: {INSERT_SCAN_RUN_MERGE_SQL}"
            );
        }
        assert!(
            INSERT_SCAN_RUN_MERGE_SQL.contains("INSERT INTO scan_run_merges"),
            "must target the scan_run_merges table"
        );
        // Five placeholders for five columns — a bind/column count drift is a bug.
        for ph in ["$1", "$2", "$3", "$4", "$5"] {
            assert!(
                INSERT_SCAN_RUN_MERGE_SQL.contains(ph),
                "INSERT must bind {ph}"
            );
        }
    }

    #[test]
    fn narrow_rows_affected_passes_normal_counts_and_caps_overflow() {
        // A real merge count round-trips unchanged.
        assert_eq!(narrow_rows_affected(0), 0);
        assert_eq!(narrow_rows_affected(94), 94);
        // The impossible >i32::MAX count caps rather than wrapping to a negative
        // (Standing Rule 1 — a wrapped negative would be a nonsense audit figure).
        assert_eq!(narrow_rows_affected(u64::MAX), i32::MAX);
    }
}
