//! Repository for the Theme Scan per-run header table (`scan_runs`) in the
//! `colossus_legal_v2` pipeline database. The per-candidate verdict detail lives
//! in the sibling [`super::scan_run_verdicts`].
//!
//! ## The background-job lifecycle (this module owns the writes)
//!
//! A scan is a background `tokio` task, so its `scan_runs` row moves through
//! states rather than being written once:
//!
//! 1. [`insert_scan_run_stub`] — the POST writes a MINIMAL row as `failed`
//!    BEFORE any preparation work, so a scan that dies during preparation still
//!    leaves a visible row in Run History (see below).
//! 2. [`promote_scan_run_running`] — once preparation succeeds, the same row is
//!    promoted to `running` with the resolved model, the params snapshot, and the
//!    progress DENOMINATOR (`candidates_total`); the POST then returns.
//! 3. [`bump_scan_run_progress`] — the task calls this once per judged candidate
//!    (`candidates_judged += 1`, the live outcome bucket `+= 1`, `last_progress_at`).
//! 4. [`finalize_scan_run_completed`] — on success, the task writes the
//!    authoritative final counts/tokens/cost/duration + the `summary_json`.
//! 5. [`fail_scan_run`] — on any job error, `status = failed` + a reason.
//! 6. [`sweep_running_scan_runs`] — at backend startup, any lingering `running`
//!    row was orphaned by a restart → `failed` "interrupted by restart".
//!
//! ## Why the row is born `failed` (Standing Rule 1)
//!
//! The row used to be inserted only AFTER every precondition had passed —
//! provider resolution, the vLLM gate, the candidate read. A scan that died in
//! any of those left NO row at all: the panel showed an error toast that the next
//! navigation erased, and Run History stayed blank, so eleven days of failures
//! looked exactly like eleven days of nobody scanning. Writing the stub FIRST, and
//! writing it as `failed` rather than `running`, makes the default outcome of a
//! half-finished start a visible, durable failure record. Promotion to `running`
//! is the deliberate act; nothing needs to remember to record a failure, because
//! the failure is what is already on disk.
//!
//! [`get_scan_run`] reads one row back for the poll; `summary_json` is only a
//! render convenience.
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
/// `pub(crate)` so the sibling `scan_run_projection` module can gate the
/// projecting-run query on the same token this module finalizes runs with. Two
/// spellings of "completed" is exactly the drift the constants exist to prevent.
pub(crate) const SCAN_STATUS_COMPLETED: &str = "completed";
const SCAN_STATUS_FAILED: &str = "failed";

/// The message stamped on a run the startup sweep finds still `running`.
const INTERRUPTED_BY_RESTART: &str = "interrupted by restart";

/// The `model_id` a stub row carries when the caller named no model — the choice
/// is not made until the resolver runs, and `model_id` is NOT NULL. A named
/// sentinel rather than an empty string so the history row reads as an unfinished
/// start instead of a blank cell (the panel falls back to showing the raw id when
/// it matches no catalog entry).
const MODEL_ID_UNRESOLVED: &str = "(model not yet resolved)";

/// The `error` a stub row carries from birth. It is overwritten by the real
/// reason when preparation fails, and cleared by [`promote_scan_run_running`]
/// when the run launches — so seeing THIS text in Run History means the backend
/// died somewhere in the start sequence without getting to record why (a process
/// kill, a panic), which is itself the diagnosis.
///
/// The wording deliberately says "starting" rather than naming a step: the text
/// survives a death at ANY point between the INSERT and the promotion, and a
/// message that named preparation would be actively wrong for the later ones.
const STUB_ERROR_UNFINISHED: &str =
    "the scan never finished starting and recorded no reason — the backend was \
     interrupted before the run could be launched; check the backend log for \
     this run_id";

// ─── 1. START (the stub INSERT, then the promote UPDATE) ─────────────────────

/// The fields known BEFORE any preparation work — everything a stub row needs.
#[derive(Debug, Clone)]
pub struct ScanRunStub {
    pub run_id: Uuid,
    pub scenario_id: Uuid,
    /// The model the caller ASKED for, if any. Not yet resolved (the request may
    /// name none, and the resolver may constrain the choice), so it is recorded
    /// only as a hint; [`promote_scan_run_running`] overwrites it with the model
    /// actually used.
    pub requested_model_id: Option<String>,
    pub started_at: DateTime<Utc>,
}

/// Insert the minimal `failed` stub row that makes a died-during-startup scan
/// visible in Run History (see the module doc for why `failed` is the birth
/// state). Every NOT NULL column gets an honest placeholder: the counts are
/// genuinely zero (nothing was judged), `duration_ms` is genuinely zero (nothing
/// ran), and `candidates_total` stays NULL because the pool has not been read yet
/// — NULL is "not known", distinct from a `0` that would claim an empty pool.
///
/// ## Rust Learning: `Option<&str>` from an `Option<String>` field
///
/// `requested_model_id.as_deref()` turns `&Option<String>` into `Option<&str>`
/// without cloning the `String` — `as_deref` maps `Option<T>` through `Deref`, so
/// `Option<String>` becomes `Option<&str>` borrowed from the original. The
/// `unwrap_or` then substitutes the sentinel for the `None` case, yielding a
/// plain `&str` that sqlx binds directly.
pub async fn insert_scan_run_stub(
    pool: &PgPool,
    stub: &ScanRunStub,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        r#"INSERT INTO scan_runs (
               run_id, scenario_id, model_id, resolved_params, dry_run,
               candidates_read, relevant_count, irrelevant_count, failed_count,
               input_tokens, output_tokens, computed_cost, started_at, duration_ms,
               status, candidates_total, candidates_judged, last_progress_at, error
           ) VALUES (
               $1, $2, $3, $4, false,
               0, 0, 0, 0,
               NULL, NULL, NULL, $5, 0,
               $6, NULL, 0, $5, $7
           )"#,
    )
    .bind(stub.run_id)
    .bind(stub.scenario_id)
    .bind(
        stub.requested_model_id
            .as_deref()
            .unwrap_or(MODEL_ID_UNRESOLVED),
    )
    // `resolved_params` is NOT NULL jsonb and nothing is resolved yet. A stage
    // marker rather than `{}`: an empty object and "we never got there" must be
    // distinguishable in the audit trail (Standing Rule 1).
    .bind(serde_json::json!({ "stage": "preparing" }))
    .bind(stub.started_at)
    .bind(SCAN_STATUS_FAILED)
    .bind(STUB_ERROR_UNFINISHED)
    .execute(pool)
    .await?;
    Ok(())
}

/// The fields settled once a background scan's preparation SUCCEEDS.
#[derive(Debug, Clone)]
pub struct ScanRunStart {
    pub run_id: Uuid,
    pub model_id: String,
    /// `{"temperature": <number|null>, "timeout_secs": <int>, "max_tokens": <int>,
    /// "prompt_file": <string>}`.
    pub resolved_params: serde_json::Value,
    /// The progress denominator: how many candidates the judge will be asked
    /// about, AFTER de-duplication and the pre-filter (task 2.15 Tier 2).
    pub candidates_total: i32,
    /// The candidate POOL the gather read returned, before anything was folded or
    /// set aside. Distinct from `candidates_total` since Tier 2 — the history's
    /// Candidates column and its +Δ delta measure the evidence that EXISTS about
    /// the subject, which a pre-filter setting must not appear to change.
    pub candidates_read: i32,
}

/// Promote a stub row to `running`: record the model actually resolved, the
/// params snapshot, and the progress denominator, and CLEAR the stub's error.
///
/// Clearing `error` is load-bearing, not cosmetic: the stub was born carrying a
/// failure reason, so a promotion that left it in place would show a running (and
/// later completed) run alongside an error message that never happened.
/// `candidates_read` records the POOL and `candidates_total` the judged
/// denominator; before task 2.15 they were the same number and this function
/// bound one value to both. They separated when the pre-filter landed: the pool
/// is what the gather read returned, the denominator is what the judge was asked.
/// The final tally/token/cost columns stay at their stub values until
/// [`finalize_scan_run_completed`] overwrites them.
///
/// The `WHERE status = $8` clause is a guard, not decoration: it makes promotion
/// apply only to a row still in the birth state, so a promote that arrives after
/// something else already failed or finished the run cannot resurrect it. Zero
/// rows updated is reported to the caller rather than swallowed.
pub async fn promote_scan_run_running(
    pool: &PgPool,
    start: &ScanRunStart,
) -> Result<u64, PipelineRepoError> {
    let result = sqlx::query(
        r#"UPDATE scan_runs SET
               status = $2,
               model_id = $3,
               resolved_params = $4,
               candidates_read = $5,
               candidates_total = $6,
               error = NULL,
               last_progress_at = $7
           WHERE run_id = $1 AND status = $8"#,
    )
    .bind(start.run_id)
    .bind(SCAN_STATUS_RUNNING)
    .bind(&start.model_id)
    .bind(&start.resolved_params)
    .bind(start.candidates_read)
    .bind(start.candidates_total)
    .bind(Utc::now())
    .bind(SCAN_STATUS_FAILED)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
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
        "SELECT run_id, scenario_id, status, model_id, \
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
    pub status: String,
    pub candidates_total: Option<i32>,
    pub candidates_judged: i32,
    pub relevant_count: i32,
    pub irrelevant_count: i32,
    pub failed_count: i32,
    pub computed_cost: Option<f64>,
    pub duration_ms: i64,
    pub started_at: DateTime<Utc>,
    /// The size of the candidate pool this run actually read.
    ///
    /// ## Why this is separate from `candidates_total` (task 1.7C)
    ///
    /// `candidates_total` is the PROGRESS DENOMINATOR, bound at promote time so a
    /// running scan can render "43 of 148". `candidates_read` is what the run
    /// reported having read when it finished. They agree on every completed run,
    /// and they diverge exactly where it matters: a run that failed before reading
    /// the pool has a denominator and a zero read.
    ///
    /// The history table's "Candidates" column and the "+Δ since the previous
    /// scan" delta both key off THIS column, because both are claims about the
    /// pool, not about progress. `0` means "never got to read the pool" and is
    /// rendered as an em dash rather than as the number zero — see
    /// `services::scan_run_delta`.
    pub candidates_read: i32,
    /// Why a `failed` run failed, verbatim. `None` on a run that did not fail.
    ///
    /// Stored since migration 20260715121130 and simply never served. The history
    /// table needs it: "Failed" with no reason sends the reader to the logs, which
    /// is the silent-failure shape Standing Rule 1 exists to prevent.
    pub error: Option<String>,
    /// Whether this run was a dry run (judged, nothing merged).
    ///
    /// Served so the history can label it. Measured on DEV 2026-08-03: 3 of the 4
    /// stored runs are dry runs, and a table that renders them identically to real
    /// ones tells the reader something false about what has been done to the pool.
    pub dry_run: bool,
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

/// How many COMPLETED runs one scenario has (task 2.15, piece 3).
///
/// ## Why completed, and not "any run"
///
/// The question the caller is asking is "has anything ever judged this
/// scenario's pool?" — because the answer decides whether the page may describe
/// its candidates as scan output. A run that failed at the vLLM gate judged
/// nothing, so counting it would let a scenario claim a scan parentage no verdict
/// supports. The failed run is still visible in the history, which is where it
/// belongs.
///
/// A COUNT rather than reusing `list_scan_runs`: the caller needs one number on a
/// page-load path that already makes six reads, and shipping every header row to
/// discard all but the length is work nobody uses.
pub async fn count_completed_scan_runs(
    pool: &PgPool,
    scenario_id: Uuid,
) -> Result<i64, PipelineRepoError> {
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM scan_runs WHERE scenario_id = $1 AND status = $2")
            .bind(scenario_id)
            .bind(SCAN_STATUS_COMPLETED)
            .fetch_one(pool)
            .await?;
    Ok(count)
}

/// The history-list query. Extracted as a `const` so the scenario-scoping and
/// newest-first ordering can be asserted by a SQL-shape unit test without a live
/// database (the house pattern — see `documents_delete.rs`). `computed_cost` is
/// cast `::float8` because a bare `NUMERIC` is not `f64`-decodable here (see the
/// [`ScanRunHeaderRow`] doc). Not deployment-varying — this is query text, not
/// config, so Rule 13 does not apply.
//
// The merge-history subqueries that used to fold `merge_count` / `last_merged_at`
// into each header are GONE with the run-level merge model. A per-run merge
// counter only made sense when a run was the unit of merge; now that merge is
// pick-keyed, the question it answered ("how many times was this run merged?") is
// not one the workbench asks. The `scan_run_merges` rows are still written — they
// are the audit trail — they simply no longer feed a header column. Query text,
// not config — Rule 13 N/A.
const LIST_SCAN_RUNS_SQL: &str = "SELECT run_id, model_id, status, \
     candidates_total, candidates_judged, \
     relevant_count, irrelevant_count, failed_count, \
     computed_cost::float8 AS computed_cost, duration_ms, started_at, \
     candidates_read, error, dry_run \
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

// The per-candidate verdict detail (`scan_run_verdicts`) moved to the sibling
// `scan_run_verdicts.rs` when this module reached the 300-line limit. It is
// re-exported from `pipeline_repository`, so callers are unaffected.

#[cfg(test)]
#[path = "scan_runs_tests.rs"]
mod tests;

// The stub/promote contract has its own sibling test module — `scan_runs_tests.rs`
// had reached the size limit, and the birth-and-promotion tests are a distinct
// subject from the INSERT's column shape. It borrows that module's source-reading
// helpers rather than copying them (see their `pub(super)` docs).
#[cfg(test)]
#[path = "scan_run_start_tests.rs"]
mod start_tests;
