//! Extraction-run repository functions.
//!
//! Owns the lifecycle of `extraction_runs` rows (upsert at start,
//! mark complete, list, and read the latest COMPLETED pass-1) plus
//! the per-chunk progress table (`extraction_chunks`) and the
//! graph-status writeback on `extraction_items` after Ingest.
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
/// `complete_extraction_run`, chunk bookkeeping) keep working unchanged.
/// Children of the reused run are wiped separately by
/// [`reset_extraction_run_children`] — call it after this function in
/// the step so the slate is clean before new items / chunks get
/// inserted.
///
/// ## F3 Reproducibility
///
/// The F3 parameters capture everything needed to reproduce an
/// extraction: the assembled prompt, template/rules file hashes, the
/// schema content, and model parameters. All are optional so older code
/// paths still compile.
#[allow(clippy::too_many_arguments)]
pub async fn insert_extraction_run(
    pool: &PgPool,
    document_id: &str,
    pass_number: i32,
    model_name: &str,
    schema_version: &str,
    // F3 reproducibility fields:
    assembled_prompt: Option<&str>,
    template_name: Option<&str>,
    template_hash: Option<&str>,
    rules_name: Option<&str>,
    rules_hash: Option<&str>,
    schema_hash: Option<&str>,
    schema_content: Option<&serde_json::Value>,
    temperature: Option<f64>,
    max_tokens_requested: Option<i32>,
    admin_instructions: Option<&str>,
    prior_context: Option<&str>,
) -> Result<i32, PipelineRepoError> {
    let row = sqlx::query_scalar::<_, i32>(
        r#"INSERT INTO extraction_runs (
               document_id, pass_number, model_name, schema_version,
               started_at, raw_output, status,
               assembled_prompt, template_name, template_hash,
               rules_name, rules_hash, schema_hash, schema_content,
               temperature, max_tokens_requested,
               admin_instructions, prior_context
           ) VALUES (
               $1, $2, $3, $4, NOW(), '{}'::jsonb, $16,
               $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15
           )
           ON CONFLICT ON CONSTRAINT extraction_runs_doc_pass_unique DO UPDATE SET
               -- Identify the new attempt: reset lifecycle columns.
               status = $16,
               started_at = NOW(),
               completed_at = NULL,
               raw_output = '{}'::jsonb,
               input_tokens = NULL,
               output_tokens = NULL,
               cost_usd = NULL,
               -- Overwrite current-attempt metadata with the new values.
               model_name = EXCLUDED.model_name,
               schema_version = EXCLUDED.schema_version,
               assembled_prompt = EXCLUDED.assembled_prompt,
               template_name = EXCLUDED.template_name,
               template_hash = EXCLUDED.template_hash,
               rules_name = EXCLUDED.rules_name,
               rules_hash = EXCLUDED.rules_hash,
               schema_hash = EXCLUDED.schema_hash,
               schema_content = EXCLUDED.schema_content,
               temperature = EXCLUDED.temperature,
               max_tokens_requested = EXCLUDED.max_tokens_requested,
               admin_instructions = EXCLUDED.admin_instructions,
               prior_context = EXCLUDED.prior_context,
               -- Chunk stats / config snapshot get filled in later in
               -- the step; clear any stale values from the prior attempt.
               chunk_count = NULL,
               chunks_succeeded = NULL,
               chunks_failed = NULL,
               chunks_pruned_nodes = NULL,
               chunks_pruned_relationships = NULL,
               processing_config = NULL
           RETURNING id"#,
    )
    .bind(document_id)
    .bind(pass_number)
    .bind(model_name)
    .bind(schema_version)
    .bind(assembled_prompt)
    .bind(template_name)
    .bind(template_hash)
    .bind(rules_name)
    .bind(rules_hash)
    .bind(schema_hash)
    .bind(schema_content)
    .bind(temperature)
    .bind(max_tokens_requested)
    .bind(admin_instructions)
    .bind(prior_context)
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
/// items / relationships / chunks from a prior failed attempt would
/// coexist with rows from the current attempt under the same run_id.
///
/// Runs in a transaction so a partial delete doesn't leave orphans
/// behind. FK-safe order:
///   review_edit_history -> extraction_relationships
///                       -> extraction_items
///                       -> extraction_chunks
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

    sqlx::query("DELETE FROM extraction_chunks WHERE extraction_run_id = $1")
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

// ── Per-chunk extraction tracking (extraction_chunks) ───────────

/// Insert a pending extraction chunk record. Returns the chunk UUID.
///
/// Called at the start of each chunk's processing. Status starts as 'pending'
/// and is updated to 'success' or 'failed' by `complete_extraction_chunk`.
///
/// `chunk_metadata` is the splitter's per-chunk metadata map (atomic-unit
/// range, identifiers, preamble flags, fallback reason, boundary pattern,
/// etc.) serialised as JSONB. Written once at insert because the value is
/// immutable for the chunk's lifetime — it describes the chunk's structural
/// origin, not its extraction outcome. FixedSizeSplitter currently emits
/// `{}`; the StructureAwareSplitter populates it.
pub async fn insert_extraction_chunk(
    pool: &PgPool,
    run_id: i32,
    chunk_index: i32,
    chunk_text: &str,
    chunk_metadata: &serde_json::Value,
) -> Result<uuid::Uuid, PipelineRepoError> {
    let id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO extraction_chunks
           (id, extraction_run_id, chunk_index, chunk_text, chunk_metadata,
            status, created_at)
           VALUES ($1, $2, $3, $4, $5, 'pending', NOW())"#,
    )
    .bind(id)
    .bind(run_id)
    .bind(chunk_index)
    .bind(chunk_text)
    .bind(chunk_metadata)
    .execute(pool)
    .await?;
    Ok(id)
}

/// Update an extraction chunk with its result.
///
/// Called after each chunk completes (success or failure). Sets final status,
/// entity/relationship counts, token usage, duration, and error message.
#[allow(clippy::too_many_arguments)]
pub async fn complete_extraction_chunk(
    pool: &PgPool,
    chunk_id: uuid::Uuid,
    status: &str,
    node_count: Option<i32>,
    relationship_count: Option<i32>,
    input_tokens: Option<i32>,
    output_tokens: Option<i32>,
    duration_ms: Option<i32>,
    error_message: Option<&str>,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        r#"UPDATE extraction_chunks
           SET status = $1, node_count = $2, relationship_count = $3,
               input_tokens = $4, output_tokens = $5,
               duration_ms = $6, error_message = $7
           WHERE id = $8"#,
    )
    .bind(status)
    .bind(node_count)
    .bind(relationship_count)
    .bind(input_tokens)
    .bind(output_tokens)
    .bind(duration_ms)
    .bind(error_message)
    .bind(chunk_id)
    .execute(pool)
    .await?;
    Ok(())
}

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
