//! Extraction-run repository functions.
//!
//! Owns the lifecycle of `extraction_runs` rows (upsert at start,
//! mark complete, list, and read the latest COMPLETED pass-1) plus
//! the aggregate per-run chunk statistics and the graph-status
//! writeback on `extraction_items` after Ingest.
//!
//! Siblings:
//! - [`super::extraction_items`] — item-level CRUD that this module's
//!   `update_graph_status_for_run` writes through.
//! - [`super::extraction_relationships`] — relationship CRUD that
//!   shares a run_id with the items here.

use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::models::document_status::{RUN_STATUS_COMPLETED, RUN_STATUS_RUNNING};

use super::PipelineRepoError;

// ── Record types ─────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExtractionRunRecord {
    pub id: i32,
    pub document_id: String,
    pub pass_number: i32,
    pub model_name: String,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    /// NUMERIC(10,4) cast to text in SQL — avoids needing rust_decimal.
    pub cost_usd: Option<String>,
    pub status: String,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

// ── Functions ────────────────────────────────────────────────────

/// Upsert an extraction run record on `(document_id, pass_number)`.
/// Returns the run's id.
///
/// ## Self-idempotency (R5)
///
/// A prior FAILED or stuck-RUNNING attempt leaves an extraction_runs row
/// that no longer represents anything actionable. Without this upsert,
/// a retry would hit the `extraction_runs_doc_pass_unique` constraint
/// (see migration `20260422113610_add_unique_constraint_on_extraction_runs_document_and_pass.sql`).
/// On conflict we reset every INSERT-time column so the reused row
/// represents the *current* attempt, preserving the synthetic id so
/// downstream callers (`store_entities_and_relationships`,
/// `complete_extraction_run`) keep working unchanged.
/// Children of the reused run are wiped separately by
/// [`reset_extraction_run_children`] — call it after this function in
/// the step so the slate is clean before new items get inserted.
///
/// ## F3 Reproducibility
///
/// `template_name` and `rules_name` record which template and global-rules
/// file produced the run. The other F3 fields that used to live on this
/// table (assembled prompt, file hashes, schema content, temperature,
/// max-tokens, admin instructions, prior context) were dropped in migration
/// `20260526133731_drop_dead_schema_surfaces.sql`: they were written here
/// but never read by any query. The queryable reproducibility copy is the
/// `processing_config` JSONB snapshot written by
/// `write_processing_config_snapshot`, which the quality report reads.
/// Both names are optional so a run with no configured rules file still
/// inserts cleanly.
pub async fn insert_extraction_run(
    pool: &PgPool,
    document_id: &str,
    pass_number: i32,
    model_name: &str,
    schema_version: &str,
    // Retained F3 names; the hashes / prompt / schema content / model params
    // they used to sit beside were dropped (see the doc comment above).
    template_name: Option<&str>,
    rules_name: Option<&str>,
) -> Result<i32, PipelineRepoError> {
    let row = sqlx::query_scalar::<_, i32>(
        r#"INSERT INTO extraction_runs (
               document_id, pass_number, model_name, schema_version,
               started_at, raw_output, status,
               template_name, rules_name
           ) VALUES (
               $1, $2, $3, $4, NOW(), '{}'::jsonb, $7,
               $5, $6
           )
           ON CONFLICT ON CONSTRAINT extraction_runs_doc_pass_unique DO UPDATE SET
               -- Identify the new attempt: reset lifecycle columns.
               status = $7,
               started_at = NOW(),
               completed_at = NULL,
               raw_output = '{}'::jsonb,
               input_tokens = NULL,
               output_tokens = NULL,
               cost_usd = NULL,
               -- Overwrite current-attempt metadata with the new values.
               model_name = EXCLUDED.model_name,
               schema_version = EXCLUDED.schema_version,
               template_name = EXCLUDED.template_name,
               rules_name = EXCLUDED.rules_name,
               -- Chunk stats / config snapshot get filled in later in
               -- the step; clear any stale values from the prior attempt.
               chunk_count = NULL,
               chunks_succeeded = NULL,
               chunks_failed = NULL,
               processing_config = NULL
           RETURNING id"#,
    )
    .bind(document_id)
    .bind(pass_number)
    .bind(model_name)
    .bind(schema_version)
    .bind(template_name)
    .bind(rules_name)
    .bind(RUN_STATUS_RUNNING)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Wipe child rows of an extraction_runs row so a reused run_id starts
/// clean. Idempotent: a brand-new run has no children, so every DELETE
/// affects zero rows.
///
/// Must be called after [`insert_extraction_run`] in the step path to
/// make re-running LlmExtract truly self-idempotent (R5). Without this,
/// items / relationships from a prior failed attempt would coexist with
/// rows from the current attempt under the same run_id.
///
/// Runs in a transaction so a partial delete doesn't leave orphans
/// behind. FK-safe order:
///   review_edit_history -> extraction_relationships
///                       -> extraction_items
pub async fn reset_extraction_run_children(
    pool: &PgPool,
    run_id: i32,
) -> Result<(), PipelineRepoError> {
    let mut txn = pool.begin().await?;

    sqlx::query(
        "DELETE FROM review_edit_history \
         WHERE item_id IN (SELECT id FROM extraction_items WHERE run_id = $1)",
    )
    .bind(run_id)
    .execute(&mut *txn)
    .await?;

    sqlx::query("DELETE FROM extraction_relationships WHERE run_id = $1")
        .bind(run_id)
        .execute(&mut *txn)
        .await?;

    sqlx::query("DELETE FROM extraction_items WHERE run_id = $1")
        .bind(run_id)
        .execute(&mut *txn)
        .await?;

    txn.commit().await?;
    Ok(())
}

/// Update an extraction run with results (completed or failed).
pub async fn complete_extraction_run(
    pool: &PgPool,
    run_id: i32,
    raw_output: &serde_json::Value,
    input_tokens: Option<i32>,
    output_tokens: Option<i32>,
    cost_usd: Option<f64>,
    status: &str,
) -> Result<(), PipelineRepoError> {
    // cost_usd is NUMERIC(10,4) in Postgres. We store it as text and cast
    // in SQL because sqlx needs the rust_decimal feature for direct NUMERIC binding.
    let cost_str = cost_usd.map(|c| format!("{c:.4}"));
    sqlx::query(
        r#"UPDATE extraction_runs
           SET raw_output = $1, input_tokens = $2, output_tokens = $3,
               cost_usd = $4::numeric, status = $5, completed_at = NOW()
           WHERE id = $6"#,
    )
    .bind(raw_output)
    .bind(input_tokens)
    .bind(output_tokens)
    .bind(cost_str)
    .bind(status)
    .bind(run_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get the latest COMPLETED **pass-1** extraction run ID for a document.
///
/// Returns `None` if no completed pass-1 run exists.
///
/// Pass-2 runs are intentionally excluded: every downstream caller
/// (ingest, completeness, items listing) uses this id to filter
/// `extraction_items`, and pass 2 writes no items (only relationships,
/// referencing pass-1 items by DB id). Before pass 2 existed, there was
/// at most one COMPLETED run per doc so the pass_number filter was
/// redundant. After Task 3 wired pass 2 into the FSM, leaving the
/// filter off caused ingest to pick up the pass-2 `run_id`, return
/// zero items for that run, and then fail when pass-2 relationships
/// referenced pass-1 items that had never been written to Neo4j.
pub async fn get_latest_completed_run(
    pool: &PgPool,
    document_id: &str,
) -> Result<Option<i32>, PipelineRepoError> {
    let row = sqlx::query_scalar::<_, i32>(
        "SELECT id FROM extraction_runs
         WHERE document_id = $1 AND pass_number = 1 AND status = $2
         ORDER BY id DESC LIMIT 1",
    )
    .bind(document_id)
    .bind(RUN_STATUS_COMPLETED)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Get extraction run metadata for a document.
pub async fn get_extraction_runs(
    pool: &PgPool,
    document_id: &str,
) -> Result<Vec<ExtractionRunRecord>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, ExtractionRunRecord>(
        "SELECT id, document_id, pass_number, model_name, input_tokens, output_tokens,
                cost_usd::text, status, started_at, completed_at
         FROM extraction_runs WHERE document_id = $1 ORDER BY id",
    )
    .bind(document_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Mark items as 'written' or 'flagged' based on grounding status.
/// Called after auto-ingest to record what was written to Neo4j.
/// Returns (written_count, flagged_count).
pub async fn update_graph_status_for_run(
    pool: &PgPool,
    run_id: i32,
) -> Result<(i32, i32), PipelineRepoError> {
    // Mark grounded items as 'written'
    let written = sqlx::query(
        "UPDATE extraction_items SET graph_status = 'written'
         WHERE run_id = $1
           AND grounding_status IN ('exact', 'normalized', 'name_matched', 'heading_matched', 'derived', 'unverified')",
    )
    .bind(run_id)
    .execute(pool)
    .await?
    .rows_affected() as i32;

    // Mark ungrounded items as 'flagged'
    let flagged = sqlx::query(
        "UPDATE extraction_items SET graph_status = 'flagged'
         WHERE run_id = $1
           AND (grounding_status IN ('not_found', 'missing_quote') OR grounding_status IS NULL)",
    )
    .bind(run_id)
    .execute(pool)
    .await?
    .rows_affected() as i32;

    Ok((written, flagged))
}

// ── Per-run chunk statistics (extraction_runs.chunk_count, …) ───
//
// Per-chunk observability rows (the old `extraction_chunks` table) were
// dropped in migration 20260526133731 — they were written but never read.
// The aggregate chunk counts below live on `extraction_runs` and ARE read
// (the documents list/detail SELECT surfaces them in the UI).

/// Update the chunk statistics on an extraction run after all chunks complete.
///
/// Called once after the chunk loop finishes, regardless of partial success.
/// These columns were added by migration 20260412_fp7_chunk_extraction.sql.
pub async fn update_run_chunk_stats(
    pool: &PgPool,
    run_id: i32,
    chunk_count: i32,
    chunks_succeeded: i32,
    chunks_failed: i32,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        r#"UPDATE extraction_runs
           SET chunk_count = $1, chunks_succeeded = $2, chunks_failed = $3
           WHERE id = $4"#,
    )
    .bind(chunk_count)
    .bind(chunks_succeeded)
    .bind(chunks_failed)
    .bind(run_id)
    .execute(pool)
    .await?;
    Ok(())
}
