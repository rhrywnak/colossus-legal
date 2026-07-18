//! Repository for the Theme Scan audit + benchmark tables (`scan_runs`,
//! `scan_run_verdicts`) in the `colossus_legal_v2` pipeline database.
//!
//! ## The background-job lifecycle (this module owns the writes)
//!
//! A scan is a background `tokio` task, so its `scan_runs` row moves through
//! states rather than being written once:
//!
//! 1. [`insert_scan_run_running`] — the POST inserts the row as `running` with
//!    the progress DENOMINATOR (`candidates_total`) known up front, then returns.
//! 2. [`bump_scan_run_progress`] — the task calls this once per judged candidate
//!    (`candidates_judged += 1`, the live outcome bucket `+= 1`, `last_progress_at`).
//! 3. [`finalize_scan_run_completed`] — on success, the task writes the
//!    authoritative final counts/tokens/cost/duration + the `summary_json`.
//! 4. [`fail_scan_run`] — on any job error, `status = failed` + a reason.
//! 5. [`sweep_running_scan_runs`] — at backend startup, any lingering `running`
//!    row was orphaned by a restart → `failed` "interrupted by restart".
//!
//! [`get_scan_run`] reads one row back for the poll. `scan_run_verdicts` (the
//! per-candidate detail the agreement query joins on) is still written via
//! [`insert_scan_run_verdicts`] — `summary_json` is only a render convenience.
//!
//! ## Rust Learning: caller-owns-serialization for the JSONB snapshots
//!
//! `resolved_params` and `summary_json` are `serde_json::Value`s the CALLER
//! builds, not typed structs this module serializes. That keeps the repository
//! dumb (it binds bytes, it does not know the resolver/summary shape) and puts
//! each snapshot's shape next to the code that produces it.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::PipelineRepoError;

// CONST: the `scan_runs.status` vocabulary, owned by code (the migration keeps NO
// DB CHECK on the column so it can evolve without a migration). Named constants
// rather than string literals so a typo is a compile error, not a silent bad row.
/// `pub(crate)` so the POST handler can label the freshly-spawned run without a
/// magic string of its own.
pub(crate) const SCAN_STATUS_RUNNING: &str = "running";
const SCAN_STATUS_COMPLETED: &str = "completed";
const SCAN_STATUS_FAILED: &str = "failed";

/// The message stamped on a run the startup sweep finds still `running`.
const INTERRUPTED_BY_RESTART: &str = "interrupted by restart";

// ─── 1. START (the `running` INSERT) ─────────────────────────────────────────

/// The fields known when a background scan STARTS.
#[derive(Debug, Clone)]
pub struct ScanRunStart {
    pub run_id: Uuid,
    pub scenario_id: Uuid,
    pub model_id: String,
    /// `{"temperature": <number|null>, "timeout_secs": <int>, "max_tokens": <int>}`.
    pub resolved_params: serde_json::Value,
    pub dry_run: bool,
    /// The progress denominator, known from the candidate-pool read.
    pub candidates_total: i32,
    pub started_at: DateTime<Utc>,
}

/// Insert the run as `running`. The final tally/token/cost columns start at
/// 0/NULL and are overwritten by [`finalize_scan_run_completed`]; `candidates_read`
/// is set to `candidates_total` here (we DID read the whole pool to size it).
pub async fn insert_scan_run_running(
    pool: &PgPool,
    start: &ScanRunStart,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        r#"INSERT INTO scan_runs (
               run_id, scenario_id, model_id, resolved_params, dry_run,
               candidates_read, relevant_count, irrelevant_count, failed_count,
               input_tokens, output_tokens, computed_cost, started_at, duration_ms,
               status, candidates_total, candidates_judged, last_progress_at
           ) VALUES (
               $1, $2, $3, $4, $5,
               $6, 0, 0, 0,
               NULL, NULL, NULL, $7, 0,
               $8, $6, 0, $7
           )"#,
    )
    .bind(start.run_id)
    .bind(start.scenario_id)
    .bind(&start.model_id)
    .bind(&start.resolved_params)
    .bind(start.dry_run)
    .bind(start.candidates_total)
    .bind(start.started_at)
    .bind(SCAN_STATUS_RUNNING)
    .execute(pool)
    .await?;
    Ok(())
}

// ─── 2. PROGRESS (per-candidate bump) ────────────────────────────────────────

/// Which live running-count column a judged candidate advances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressBucket {
    Relevant,
    Irrelevant,
    Failed,
}

/// The (fixed, code-owned) column name for a bucket. Split out so the
/// no-injection reasoning below is unit-testable.
fn bucket_column(bucket: ProgressBucket) -> &'static str {
    match bucket {
        ProgressBucket::Relevant => "relevant_count",
        ProgressBucket::Irrelevant => "irrelevant_count",
        ProgressBucket::Failed => "failed_count",
    }
}

/// Bump progress for one judged candidate: `candidates_judged += 1`, the bucket's
/// running count `+= 1`, and `last_progress_at = NOW()`.
///
/// ## Rust Learning: why `format!`-ing the column name is safe here
///
/// The column name comes from [`bucket_column`], which returns one of three
/// `&'static str` LITERALS chosen by a Rust `match` — never from user input. So
/// interpolating it into the SQL cannot be an injection vector (unlike binding a
/// value, a column/table name cannot be a bound parameter, so this is the correct
/// way to vary it). The `run_id` — the only untrusted-shaped value — is still a
/// bound `$1` parameter. The `SET x = x + 1` increment is atomic per statement,
/// so the concurrent `buffer_unordered` fan-out cannot lose an update.
pub async fn bump_scan_run_progress(
    pool: &PgPool,
    run_id: Uuid,
    bucket: ProgressBucket,
) -> Result<(), PipelineRepoError> {
    let col = bucket_column(bucket);
    let sql = format!(
        "UPDATE scan_runs \
         SET candidates_judged = candidates_judged + 1, {col} = {col} + 1, \
             last_progress_at = NOW() \
         WHERE run_id = $1"
    );
    sqlx::query(&sql).bind(run_id).execute(pool).await?;
    Ok(())
}

// ─── 3. COMPLETE (finalize) ──────────────────────────────────────────────────

/// The authoritative fields settled when a scan COMPLETES.
#[derive(Debug, Clone)]
pub struct ScanRunFinal {
    pub run_id: Uuid,
    pub relevant_count: i32,
    pub irrelevant_count: i32,
    pub failed_count: i32,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub computed_cost: Option<f64>,
    pub duration_ms: i64,
    /// The finished `ThemeScanSummary`, serialized by the caller.
    pub summary_json: serde_json::Value,
}

/// Finalize a `running` run to `completed`, overwriting the live estimates with
/// the authoritative final counts and storing the render summary.
pub async fn finalize_scan_run_completed(
    pool: &PgPool,
    final_: &ScanRunFinal,
) -> Result<(), PipelineRepoError> {
    // Fixed 8-decimal string mirrors the NUMERIC(12,8) column (no rust_decimal
    // feature); None → NULL, passed through the `::numeric` cast.
    let cost_str = final_.computed_cost.map(|c| format!("{c:.8}"));
    sqlx::query(
        r#"UPDATE scan_runs SET
               status = $2,
               relevant_count = $3, irrelevant_count = $4, failed_count = $5,
               input_tokens = $6, output_tokens = $7, computed_cost = $8::numeric,
               duration_ms = $9, summary_json = $10, last_progress_at = NOW()
           WHERE run_id = $1"#,
    )
    .bind(final_.run_id)
    .bind(SCAN_STATUS_COMPLETED)
    .bind(final_.relevant_count)
    .bind(final_.irrelevant_count)
    .bind(final_.failed_count)
    .bind(final_.input_tokens)
    .bind(final_.output_tokens)
    .bind(cost_str)
    .bind(final_.duration_ms)
    .bind(&final_.summary_json)
    .execute(pool)
    .await?;
    Ok(())
}

// ─── 4. FAIL ─────────────────────────────────────────────────────────────────

/// Mark a run `failed` with a reason (Standing Rule 1 — a failed run says why).
pub async fn fail_scan_run(
    pool: &PgPool,
    run_id: Uuid,
    error: &str,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "UPDATE scan_runs SET status = $2, error = $3, last_progress_at = NOW() \
         WHERE run_id = $1",
    )
    .bind(run_id)
    .bind(SCAN_STATUS_FAILED)
    .bind(error)
    .execute(pool)
    .await?;
    Ok(())
}

// ─── 5. STARTUP SWEEP (orphan guard) ─────────────────────────────────────────

/// Fail every run still `running` at backend startup — a `running` row at boot
/// was orphaned by a restart (the `tokio` task did not survive). Returns the
/// number swept, for a startup log. The authoritative orphan guard, run once per
/// boot (no reaper daemon, no no-progress timer that could kill a slow run).
pub async fn sweep_running_scan_runs(pool: &PgPool) -> Result<u64, PipelineRepoError> {
    let result = sqlx::query(
        "UPDATE scan_runs SET status = $2, error = $3, last_progress_at = NOW() \
         WHERE status = $1",
    )
    .bind(SCAN_STATUS_RUNNING)
    .bind(SCAN_STATUS_FAILED)
    .bind(INTERRUPTED_BY_RESTART)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

// ─── 6. READ (the poll) ──────────────────────────────────────────────────────

/// One `scan_runs` row as the GET poll needs it. `summary_json` is `Some` only
/// once `status = completed`; the live counts are an in-progress estimate while
/// `running` and authoritative once `completed`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ScanRunStatusRow {
    pub run_id: Uuid,
    pub scenario_id: Uuid,
    pub status: String,
    pub model_id: String,
    pub dry_run: bool,
    pub candidates_total: Option<i32>,
    pub candidates_judged: i32,
    pub relevant_count: i32,
    pub irrelevant_count: i32,
    pub failed_count: i32,
    pub error: Option<String>,
    pub summary_json: Option<serde_json::Value>,
    pub last_progress_at: Option<DateTime<Utc>>,
    pub started_at: DateTime<Utc>,
}

/// Read one run by id. `None` if the id does not exist (the handler maps that to
/// 404 after the case-fence check).
pub async fn get_scan_run(
    pool: &PgPool,
    run_id: Uuid,
) -> Result<Option<ScanRunStatusRow>, PipelineRepoError> {
    let row = sqlx::query_as::<_, ScanRunStatusRow>(
        "SELECT run_id, scenario_id, status, model_id, dry_run, \
                candidates_total, candidates_judged, \
                relevant_count, irrelevant_count, failed_count, \
                error, summary_json, last_progress_at, started_at \
         FROM scan_runs WHERE run_id = $1",
    )
    .bind(run_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

// ─── 7. LIST (the history headers) ───────────────────────────────────────────

/// One row of the scan-run HISTORY list — a lightweight header, NOT the full
/// result. Deliberately omits `summary_json` and the per-candidate verdicts: the
/// history list renders many runs, and the detail is fetched lazily per-run via
/// [`get_scan_run`] when a row is opened.
///
/// ## Rust Learning: why `computed_cost` is read via a `::float8` cast
///
/// `computed_cost` is `NUMERIC(12,8)` in Postgres. `sqlx` cannot decode a bare
/// `NUMERIC` into `f64` without the `rust_decimal`/`bigdecimal` feature (which
/// this workspace does not enable — the same reason `finalize_scan_run_completed`
/// round-trips the value through a formatted string). Casting `computed_cost::float8`
/// in the SELECT converts it to a Postgres `double precision`, which decodes
/// cleanly into `Option<f64>`. `NULL` (local model / no token usage) stays `None`.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ScanRunHeaderRow {
    pub run_id: Uuid,
    pub model_id: String,
    pub dry_run: bool,
    pub status: String,
    pub candidates_total: Option<i32>,
    pub candidates_judged: i32,
    pub relevant_count: i32,
    pub irrelevant_count: i32,
    pub failed_count: i32,
    pub computed_cost: Option<f64>,
    pub duration_ms: i64,
    pub started_at: DateTime<Utc>,
    /// How many times this run has been merged into its scenario — `COUNT(*)` of
    /// its `scan_run_merges` rows (`0` = never merged). BIGINT → `i64`. Drives the
    /// run detail's "Merged N×" state; `0` shows the plain "Merge into scenario".
    pub merge_count: i64,
    /// The most recent merge time — `MAX(merged_at)` — or `None` when never merged.
    /// A distinct observable from `merge_count = 0` by construction (they always
    /// agree), shown as "last <time>" beside the count.
    pub last_merged_at: Option<DateTime<Utc>>,
}

/// List every run of one scenario, newest first, as lightweight headers.
///
/// Scoped by `scenario_id` (`WHERE scenario_id = $1`), so the caller's scenario
/// fence is sufficient — every returned row already belongs to that scenario, no
/// per-row re-check is needed (unlike [`get_scan_run`], which is keyed by
/// `run_id` alone and needs a scenario-match fence at the service layer). The
/// existing `scan_runs_scenario_id_idx` covers this filter; `ORDER BY started_at
/// DESC` gives the newest-first history the panel renders. An empty result
/// (a scenario that was never scanned) is a legitimate empty `Vec`, distinct from
/// an error (Standing Rule 1).
pub async fn list_scan_runs(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<Vec<ScanRunHeaderRow>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, ScanRunHeaderRow>(LIST_SCAN_RUNS_SQL)
        .bind(scenario_id)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// The history-list query. Extracted as a `const` so the scenario-scoping and
/// newest-first ordering can be asserted by a SQL-shape unit test without a live
/// database (the house pattern — see `documents_delete.rs`). `computed_cost` is
/// cast `::float8` because a bare `NUMERIC` is not `f64`-decodable here (see the
/// [`ScanRunHeaderRow`] doc). Not deployment-varying — this is query text, not
/// config, so Rule 13 does not apply.
//
// The two correlated subqueries fold each run's merge history (the child
// `scan_run_merges` rows) into the header: `merge_count` = how many times it was
// merged, `last_merged_at` = when last. Correlated subqueries (not a GROUP BY
// JOIN) keep this a drop-in extension of the existing single-row-per-run SELECT —
// a run with zero merges still returns exactly one header row, with `merge_count`
// = 0 and `last_merged_at` = NULL (a LEFT JOIN + GROUP BY would reach the same
// result but restructure the whole query). The `scan_run_merges_run_id_idx`
// covers both. Query text, not config — Rule 13 N/A.
const LIST_SCAN_RUNS_SQL: &str = "SELECT run_id, model_id, dry_run, status, \
     candidates_total, candidates_judged, \
     relevant_count, irrelevant_count, failed_count, \
     computed_cost::float8 AS computed_cost, duration_ms, started_at, \
     (SELECT COUNT(*) FROM scan_run_merges m WHERE m.run_id = scan_runs.run_id) AS merge_count, \
     (SELECT MAX(m.merged_at) FROM scan_run_merges m WHERE m.run_id = scan_runs.run_id) AS last_merged_at \
     FROM scan_runs WHERE scenario_id = $1 ORDER BY started_at DESC";

// ─── 8. DELETE (remove one run) ──────────────────────────────────────────────

/// The delete query. Extracted as a `const` (house pattern, mirrors
/// [`LIST_SCAN_RUNS_SQL`]) so a SQL-shape unit test can assert the `scenario_id`
/// fence without a live database. The `scan_run_verdicts` child rows cascade via
/// their `run_id` foreign key (`ON DELETE CASCADE`, migration 20260715121130), so
/// this single statement removes the run AND its per-candidate verdicts. Not
/// deployment-varying — query text, not config, so Rule 13 does not apply.
const DELETE_SCAN_RUN_SQL: &str = "DELETE FROM scan_runs WHERE run_id = $1 AND scenario_id = $2";

/// Delete one scan run, scoped by BOTH `run_id` AND `scenario_id`.
///
/// The `scenario_id` in the `WHERE` is the case-fence made durable at the SQL
/// layer (Standing Rule 1 — a caller cannot delete a run that belongs to another
/// scenario, even with a valid `run_id`): a run in a different scenario matches
/// zero rows, indistinguishable from a truly-absent id. Returns the number of
/// rows deleted so the caller can map `0` to a 404 (not-found) rather than a
/// silent success. The `scan_run_verdicts` detail cascades (see the SQL doc).
///
/// ## Rust Learning: `rows_affected()` as the found/not-found signal
///
/// A `DELETE` that matches nothing is NOT an error in SQL — it succeeds with zero
/// rows touched. `sqlx`'s `PgQueryResult::rows_affected()` returns that count, so
/// the caller distinguishes "deleted it" (`1`) from "no such run here" (`0`)
/// without a preceding `SELECT`. One statement, one round-trip, no TOCTOU window.
pub async fn delete_scan_run(
    pool: &PgPool,
    scenario_id: Uuid,
    run_id: Uuid,
) -> Result<u64, PipelineRepoError> {
    let result = sqlx::query(DELETE_SCAN_RUN_SQL)
        .bind(run_id)
        .bind(scenario_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

// ─── Per-candidate verdict detail (unchanged from Chunk B) ────────────────────

/// One row of `scan_run_verdicts` — a per-candidate verdict.
///
/// On a successful judgement `relevant`/`proposed_role`/`confidence`/`reason`
/// are `Some`; on a per-item failure they are `None` and `error` carries the
/// reason (Standing Rule 1: failed is distinguishable and says why). `raw_reply`
/// is the model's raw text, kept for successes and parse-failures alike.
#[derive(Debug, Clone)]
pub struct ScanRunVerdictRecord {
    pub run_id: Uuid,
    pub graph_node_id: String,
    pub relevant: Option<bool>,
    pub proposed_role: Option<String>,
    /// Postgres `REAL` → `f32` (model emits ~2-decimal confidence).
    pub confidence: Option<f32>,
    pub reason: Option<String>,
    pub raw_reply: Option<String>,
    /// `None` = judged successfully; `Some` = the per-item failure reason.
    pub error: Option<String>,
}

/// Insert every per-candidate verdict for a run in ONE transaction.
///
/// ## Rust Learning: `&mut *txn` — reborrowing the transaction for each `execute`
///
/// `pool.begin()` yields a `Transaction` that owns a connection. Each
/// `execute(&mut *txn)` needs a `&mut` borrow of it, but the loop must run many
/// executes and then `commit()` — so we cannot MOVE the transaction into the
/// first call. `&mut *txn` dereferences the transaction and re-borrows it
/// mutably for just that call, releasing the borrow before the next iteration.
/// One atomic write: either every verdict lands or none does (a partial verdict
/// set would corrupt the benchmark's per-candidate agreement query).
pub async fn insert_scan_run_verdicts(
    pool: &PgPool,
    verdicts: &[ScanRunVerdictRecord],
) -> Result<(), PipelineRepoError> {
    // An empty verdict set is a legitimate no-op (a scan of a subject with no
    // candidate quotes), distinct from a failure — return Ok without opening a
    // transaction rather than committing an empty one.
    if verdicts.is_empty() {
        return Ok(());
    }
    let mut txn = pool.begin().await?;
    for v in verdicts {
        sqlx::query(
            r#"INSERT INTO scan_run_verdicts (
                   run_id, graph_node_id, relevant, proposed_role,
                   confidence, reason, raw_reply, error
               ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        )
        .bind(v.run_id)
        .bind(&v.graph_node_id)
        .bind(v.relevant)
        .bind(&v.proposed_role)
        .bind(v.confidence)
        .bind(&v.reason)
        .bind(&v.raw_reply)
        .bind(&v.error)
        .execute(&mut *txn)
        .await?;
    }
    txn.commit().await?;
    Ok(())
}

#[cfg(test)]
#[path = "scan_runs_tests.rs"]
mod tests;
