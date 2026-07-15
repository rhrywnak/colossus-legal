//! Repository for the Theme Scan audit + benchmark tables (`scan_runs`,
//! `scan_run_verdicts`) in the `colossus_legal_v2` pipeline database.
//!
//! Two writes, no reads yet: the scan records a per-run header and its
//! per-candidate verdicts. The benchmark comparison query (JOIN two runs on
//! `graph_node_id`) is run by hand on build1 for now, so this module exposes
//! only the inserts. See the migration `20260715121130_create_scan_runs_and_verdicts`
//! for the column semantics.
//!
//! ## Rust Learning: caller-owns-serialization for the JSONB snapshot
//!
//! `ScanRunRecord.resolved_params` is a `serde_json::Value` the CALLER builds,
//! not a typed struct this module serializes. That keeps the repository dumb
//! (it binds bytes, it does not know the resolver's shape) and puts the
//! `{temperature, timeout_secs, max_tokens}` snapshot shape next to the code
//! that produces it — the same division the migration comment documents.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::PipelineRepoError;

/// One row of `scan_runs` — the per-run header.
///
/// Counts are `i32` (Postgres `INTEGER`); token sums and duration are `i64`
/// (`BIGINT`). `computed_cost` is a plain `f64` here and is cast to `NUMERIC` in
/// SQL on the way in (the project does not enable the `rust_decimal` feature, so
/// direct `NUMERIC` binding is unavailable — same pattern as
/// `extraction_runs.cost_usd`). `resolved_params` is pre-serialized by the caller.
#[derive(Debug, Clone)]
pub struct ScanRunRecord {
    pub run_id: Uuid,
    pub scenario_id: Uuid,
    pub model_id: String,
    /// `{"temperature": <number|null>, "timeout_secs": <int>, "max_tokens": <int>}`.
    pub resolved_params: serde_json::Value,
    pub dry_run: bool,
    pub candidates_read: i32,
    pub relevant_count: i32,
    pub irrelevant_count: i32,
    pub failed_count: i32,
    /// `None` = no call reported usage (never a fabricated 0 — Standing Rule 1).
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    /// `None` for a local vLLM model (no per-token cost) or absent usage.
    pub computed_cost: Option<f64>,
    pub started_at: DateTime<Utc>,
    pub duration_ms: i64,
}

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

/// Insert the per-run header row.
///
/// ## Rust Learning: `$n::numeric` cast instead of the `rust_decimal` feature
///
/// `computed_cost` is `NUMERIC(12,8)` in Postgres. sqlx can only bind a Rust
/// `f64` as `float8`, so we format it to a fixed-precision string and let
/// Postgres cast `$12::numeric` — exactly what `complete_extraction_run` does
/// for `cost_usd`. `None` binds as SQL `NULL`, which the cast passes through.
pub async fn insert_scan_run(pool: &PgPool, run: &ScanRunRecord) -> Result<(), PipelineRepoError> {
    // Fixed 8-decimal string mirrors the NUMERIC(12,8) column scale. None → NULL.
    let cost_str = run.computed_cost.map(|c| format!("{c:.8}"));
    sqlx::query(
        r#"INSERT INTO scan_runs (
               run_id, scenario_id, model_id, resolved_params, dry_run,
               candidates_read, relevant_count, irrelevant_count, failed_count,
               input_tokens, output_tokens, computed_cost, started_at, duration_ms
           ) VALUES (
               $1, $2, $3, $4, $5,
               $6, $7, $8, $9,
               $10, $11, $12::numeric, $13, $14
           )"#,
    )
    .bind(run.run_id)
    .bind(run.scenario_id)
    .bind(&run.model_id)
    .bind(&run.resolved_params)
    .bind(run.dry_run)
    .bind(run.candidates_read)
    .bind(run.relevant_count)
    .bind(run.irrelevant_count)
    .bind(run.failed_count)
    .bind(run.input_tokens)
    .bind(run.output_tokens)
    .bind(cost_str)
    .bind(run.started_at)
    .bind(run.duration_ms)
    .execute(pool)
    .await?;
    Ok(())
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
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;
    use std::time::Duration;

    /// A pool aimed at a dead port: any real query fails fast, so a test can
    /// prove a code path did NOT touch the database.
    fn dead_pool() -> PgPool {
        PgPoolOptions::new()
            .acquire_timeout(Duration::from_millis(500))
            .connect_lazy("postgres://127.0.0.1:1/nodb")
            .expect("connect_lazy builds a pool without connecting")
    }

    #[tokio::test]
    async fn insert_scan_run_verdicts_empty_is_ok_without_touching_the_pool() {
        // The empty-slice early return is a legitimate no-op (a subject with no
        // candidate quotes), distinct from a failure. It must return Ok WITHOUT
        // opening a transaction — the dead pool would error on any real connect.
        let result = insert_scan_run_verdicts(&dead_pool(), &[]).await;
        assert!(
            result.is_ok(),
            "empty verdicts must be a no-op Ok, got {result:?}"
        );
    }
}
