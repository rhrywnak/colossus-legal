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
//! ## The event model (why a row per merge, not a boolean)
//!
//! Merging is legitimately repeatable — the human checks a few picks today and a
//! few more next week, from this run or an older one — and the reconcile is
//! status-preserving, so repetition is safe. This module records EACH merge as its
//! own row, with the selection that produced it. A single `merged_at`/boolean
//! column could not represent that history, which is why the table exists.
//!
//! ## Audit-only: nothing here feeds the UI
//!
//! These rows once drove a "merged N× · last …" counter on the run header. That
//! counter belonged to the retired run-level merge model — merge is now pick-keyed,
//! so a per-RUN merge count answers a question the workbench no longer asks, and
//! the `LIST_SCAN_RUNS_SQL` subqueries that computed it are gone.
//!
//! The table remains as chain-of-custody hygiene: for a trial-preparation system,
//! "which run's verdicts were applied, when, and to which picks" is worth keeping
//! whether or not a screen shows it. Note it records EVENTS, including merges whose
//! writes were entirely blocked by the reconcile guard — something the per-fact
//! `scenario_fact_refs.source_run_id` provenance cannot express, which is why both
//! exist.

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
    /// The graph_node_ids the human CHECKED for this merge, stored as a JSON array.
    ///
    /// ## Domain note: the selection IS the human's decision
    ///
    /// In the pick-keyed model the human's act is *choosing which picks get a
    /// machine judgment*. Recording only `rows_affected` would preserve the
    /// event's outcome but not the act — an audit trail for a trial-preparation
    /// system needs the choice itself ("we applied the model's read of C-14, C-22
    /// and C-31 on the 19th").
    ///
    /// Deliberately distinct from [`Self::rows_affected`]: this is what was
    /// CHOSEN, that is what the status-preserving reconcile actually WROTE. They
    /// differ whenever a chosen pick's target was already included or dropped, and
    /// keeping both is what makes the reconcile guard's effect auditable after the
    /// fact.
    ///
    /// Non-optional on the Rust side even though the column is nullable: every
    /// merge written from here knows its selection. The column's NULL is reserved
    /// for rows written before the column existed ("selection not recorded"),
    /// which is a different claim from an empty array ("selected nothing").
    pub selected_node_ids: Vec<String>,
}

// CONST: the merge-event INSERT, held as a `const` so a SQL-shape unit test can
// pin the column list without a live database (house pattern — mirrors
// `scan_runs::LIST_SCAN_RUNS_SQL` / `DELETE_SCAN_RUN_SQL`). Query text, not
// deployment config, so Rule 13 does not apply.
const INSERT_SCAN_RUN_MERGE_SQL: &str = "INSERT INTO scan_run_merges \
     (merge_id, run_id, scenario_id, merged_at, rows_affected, selected_node_ids) \
     VALUES ($1, $2, $3, $4, $5, $6)";

/// Record one merge event.
///
/// ## Rust Learning: `impl sqlx::PgExecutor<'_>` so this can run inside a caller's transaction
///
/// The parameter is a generic executor, not a concrete `&PgPool`. That lets the
/// Merge service pass a `&mut *tx` here so this INSERT and the merge
/// `INSERT … SELECT` commit together (or roll back together) — the merge and its
/// provenance are one atomic unit. The `'_` is the elided lifetime of whatever
/// executor is borrowed. Same generic-executor pattern as
/// `scenario_store::upsert_fact_ref`. Passing a plain `&PgPool` still works
/// (it also implements `PgExecutor`) for a standalone write.
///
/// ## Rust Learning: `sqlx::types::Json` binds a Rust value to a JSONB column
///
/// `Vec<String>` has no direct Postgres JSONB mapping — bound bare, sqlx would
/// target `TEXT[]` and the types would not line up. Wrapping it in
/// `sqlx::types::Json(...)` tells sqlx "serialize this with serde and send it as
/// JSON". The wrapper borrows (`Json(&Vec<String>)`), so nothing is cloned, and
/// serializing a `Vec<String>` cannot fail — there is no error path to swallow
/// here, which is why this stays infallible rather than returning a serialize
/// error the caller would have to handle.
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
        .bind(sqlx::types::Json(&record.selected_node_ids))
        .execute(executor)
        .await?;
    Ok(())
}

/// How much provenance a run is carrying — the two independent records that a
/// delete would destroy.
///
/// Returned as a pair rather than a bare `bool` so the caller's refusal message
/// can say WHICH record exists. The two are genuinely independent: a merge whose
/// writes were entirely blocked by the reconcile guard leaves an event with no
/// fact-ref references, and a fact ref can outlive nothing else if its event rows
/// were removed by an older code path. Either alone is reason to refuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunProvenance {
    /// Rows in `scan_run_merges` for this run — merges that were performed.
    pub merge_events: i64,
    /// Rows in `scenario_fact_refs` still attributing their judgment to this run.
    pub attributed_facts: i64,
}

impl RunProvenance {
    /// Whether this run has any provenance worth protecting.
    ///
    /// ## Domain note: why deletion is refused rather than cascaded
    ///
    /// Deleting a merged run destroys BOTH records at once — `scan_run_merges`
    /// rows cascade away on the run's FK, and `source_run_id` references null out.
    /// For a trial-preparation system that is an unacceptable chain-of-custody
    /// loss: the case would still carry the model's judgments while losing every
    /// trace of where they came from. Unmerged runs stay freely deletable, which
    /// is what keeps junk-scan hygiene possible.
    pub fn is_protected(self) -> bool {
        self.merge_events > 0 || self.attributed_facts > 0
    }
}

// CONST: the provenance pre-check. Two scalar subqueries in ONE round-trip rather
// than two queries — the check runs on every delete attempt, and the two counts
// must describe the same instant to be a coherent refusal reason. Held as a
// `const` for the house SQL-shape-test pattern. Query text, not config.
const COUNT_RUN_PROVENANCE_SQL: &str = "SELECT \
     (SELECT COUNT(*) FROM scan_run_merges WHERE run_id = $1) AS merge_events, \
     (SELECT COUNT(*) FROM scenario_fact_refs WHERE source_run_id = $1) AS attributed_facts";

/// Count what a run's deletion would destroy.
///
/// Read BEFORE any delete so the refusal is a clean 409 rather than a
/// half-completed cascade. Zero/zero means the run is safely deletable.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the query fails. A read failure must NOT be
/// treated as "no provenance" — the caller propagates it, because silently
/// permitting a delete on an unreadable check is exactly the destructive
/// direction (Standing Rule 1).
pub async fn count_run_provenance(
    pool: &PgPool,
    run_id: Uuid,
) -> Result<RunProvenance, PipelineRepoError> {
    let row: (i64, i64) = sqlx::query_as(COUNT_RUN_PROVENANCE_SQL)
        .bind(run_id)
        .fetch_one(pool)
        .await?;
    Ok(RunProvenance {
        merge_events: row.0,
        attributed_facts: row.1,
    })
}

/// Every `graph_node_id` whose fact ref currently credits this run.
///
/// This is the "applied" state, derived exactly rather than guessed: a suggestion
/// is already-applied precisely when a fact ref names this run as its source. The
/// alternative signals are both wrong — a present `confidence` proves only that
/// SOME run scored the fact, and a merge-event row proves only that a merge
/// happened, not that this pick survived the status-preserving guard.
///
/// Returned as a `Vec<String>` for the caller to index however it needs (it builds
/// a `HashSet` for O(1) membership while walking the suggestions).
///
/// # Errors
/// Returns [`PipelineRepoError`] if the query fails.
pub async fn list_applied_node_ids_for_run(
    pool: &PgPool,
    run_id: Uuid,
) -> Result<Vec<String>, PipelineRepoError> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT graph_node_id FROM scenario_fact_refs WHERE source_run_id = $1")
            .bind(run_id)
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
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
/// owns the timestamp, matching `scan_runs.started_at`. `selected_ids` are the
/// graph_node_ids the human checked — only their verdicts are written (Option A);
/// the recorded `rows_affected` therefore reflects the SELECTED count (minus any
/// already-curated rows the reconcile skipped), which is the honest audit figure.
/// Returns the merged row count (the number of undecided suggestions inserted/refreshed).
///
/// # Errors
/// [`PipelineRepoError`] if `begin`, either write, or `commit` fails — the caller
/// maps it to its user-facing merge error.
pub async fn merge_run_into_scenario_recording(
    pool: &PgPool,
    scenario_id: Uuid,
    run_id: Uuid,
    selected_ids: &[String],
    merged_at: DateTime<Utc>,
) -> Result<u64, PipelineRepoError> {
    let mut tx = pool.begin().await?;

    let merged = merge_scan_run_into_scenario(&mut *tx, scenario_id, run_id, selected_ids).await?;

    let record = ScanRunMergeRecord {
        merge_id: Uuid::new_v4(),
        run_id,
        scenario_id,
        merged_at,
        rows_affected: narrow_rows_affected(merged),
        // The selection is recorded as the human gave it — NOT filtered down to the
        // picks that actually landed. A pick the reconcile skipped (its target was
        // already included/dropped) was still chosen, and the audit must say so;
        // the difference against `rows_affected` is exactly the guard's footprint.
        selected_node_ids: selected_ids.to_vec(),
    };
    insert_scan_run_merge(&mut *tx, &record).await?;

    tx.commit().await?;
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pin the INSERT column list: the write must name exactly the six audit
    /// columns, in the order the binds supply them. A SQL-shape test (no live DB)
    /// mirroring `scan_runs`' const-SQL tests — it catches a column rename or a
    /// bind/column drift that a clean compile would not.
    #[test]
    fn insert_sql_names_the_six_audit_columns() {
        for col in [
            "merge_id",
            "run_id",
            "scenario_id",
            "merged_at",
            "rows_affected",
            // The selection: without it the audit records the event but not the
            // human's actual choice of picks.
            "selected_node_ids",
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
        // Six placeholders for six columns — a bind/column count drift is a bug.
        for ph in ["$1", "$2", "$3", "$4", "$5", "$6"] {
            assert!(
                INSERT_SCAN_RUN_MERGE_SQL.contains(ph),
                "INSERT must bind {ph}"
            );
        }
    }

    /// Either provenance record alone must protect a run. The two are independent
    /// (see [`RunProvenance`]), so an OR — not an AND — is the correct predicate;
    /// an AND would let a run be deleted whenever one of the two records had
    /// already been lost, which is exactly when the surviving one matters most.
    #[test]
    fn a_run_is_protected_by_either_provenance_record_alone() {
        let none = RunProvenance {
            merge_events: 0,
            attributed_facts: 0,
        };
        assert!(
            !none.is_protected(),
            "an unmerged run stays deletable — junk-scan hygiene depends on it"
        );

        let events_only = RunProvenance {
            merge_events: 1,
            attributed_facts: 0,
        };
        assert!(
            events_only.is_protected(),
            "a merge whose writes were all blocked by the reconcile guard still \
             happened, and its event row is the only record of it"
        );

        let facts_only = RunProvenance {
            merge_events: 0,
            attributed_facts: 3,
        };
        assert!(
            facts_only.is_protected(),
            "facts still crediting this run would be orphaned by the delete"
        );

        let both = RunProvenance {
            merge_events: 2,
            attributed_facts: 7,
        };
        assert!(both.is_protected());
    }

    /// SQL-shape guard for the pre-delete check: it must count BOTH records, and
    /// must key each on the run. A check that silently queried only one table
    /// would let a delete through in exactly the case the other table covers.
    #[test]
    fn provenance_check_counts_both_records_for_the_run() {
        let sql = COUNT_RUN_PROVENANCE_SQL;
        assert!(
            sql.contains("FROM scan_run_merges WHERE run_id = $1"),
            "must count merge events for this run: {sql}"
        );
        assert!(
            sql.contains("FROM scenario_fact_refs WHERE source_run_id = $1"),
            "must count facts still crediting this run: {sql}"
        );
        // Both aliases are load-bearing: the caller decodes positionally, so a
        // dropped column would shift the pair and misreport the counts.
        assert!(
            sql.contains("AS merge_events") && sql.contains("AS attributed_facts"),
            "both counts must be named: {sql}"
        );
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
