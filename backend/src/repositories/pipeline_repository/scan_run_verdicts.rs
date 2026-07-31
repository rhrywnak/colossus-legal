//! Repository for `scan_run_verdicts` — the PER-CANDIDATE verdict detail of a
//! Theme Scan run.
//!
//! Split out of `scan_runs.rs` (which owns the per-run HEADER table) when that
//! module reached the 300-line limit. The split is thematic, not arbitrary: this
//! is a different table with a different write shape (one bulk transactional
//! insert at the end of a run, versus the header's four-stage lifecycle), and the
//! two are only related by the `run_id` foreign key.
//!
//! `scan_runs.rs` remains the module to read for the run lifecycle; start there.

use sqlx::PgPool;
use uuid::Uuid;

use super::PipelineRepoError;

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
mod tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;
    use std::time::Duration;

    /// A pool aimed at a dead port: any real query fails fast, so a test can prove
    /// a code path did NOT touch the database.
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
