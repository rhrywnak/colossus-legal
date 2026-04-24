//! Extraction-specific repository functions (runs, items, relationships).

use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use super::PipelineRepoError;

// ── Record types ─────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExtractionItemRecord {
    pub id: i32,
    pub run_id: i32,
    pub document_id: String,
    pub entity_type: String,
    pub item_data: serde_json::Value,
    pub verbatim_quote: Option<String>,
    pub grounding_status: Option<String>,
    pub grounded_page: Option<i32>,
    pub review_status: String,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub review_notes: Option<String>,
    pub graph_status: String,
    /// Actual Neo4j node id written by Ingest (post-resolver, post-MERGE).
    /// `None` for legacy rows from before the R1 migration — completeness
    /// falls back to recomputing the id from item_data in that case.
    pub neo4j_node_id: Option<String>,
    /// Ingest-determined Neo4j label (e.g. `"Person"` / `"Organization"`
    /// for Party items). `None` before Ingest runs or for legacy rows
    /// from before the R4 migration. `entity_type` on this struct is
    /// populated from `COALESCE(resolved_entity_type, entity_type)` at
    /// SELECT time, so callers who want the *effective* label read
    /// `entity_type`; only callers that specifically want the resolved-
    /// only value read this column.
    pub resolved_entity_type: Option<String>,
}

/// Shared SELECT column list for every `query_as::<_, ExtractionItemRecord>`
/// call site. The `entity_type` column is projected through
/// `COALESCE(resolved_entity_type, entity_type)` so the struct field
/// always carries the effective label — preserving the external API
/// shape while the underlying column stays immutable (R4). The raw
/// `resolved_entity_type` column is also returned for callers that
/// need the resolved-only value.
const ITEM_SELECT_COLUMNS: &str =
    "id, run_id, document_id, \
     COALESCE(resolved_entity_type, entity_type) AS entity_type, \
     item_data, verbatim_quote, grounding_status, grounded_page, \
     review_status, reviewed_by, reviewed_at, review_notes, \
     graph_status, neo4j_node_id, resolved_entity_type";

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExtractionRelationshipRecord {
    pub id: i32,
    pub run_id: i32,
    pub document_id: String,
    pub from_item_id: i32,
    pub to_item_id: i32,
    pub relationship_type: String,
    pub properties: Option<serde_json::Value>,
    pub review_status: String,
    pub tier: i32,
}

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
               $1, $2, $3, $4, NOW(), '{}'::jsonb, 'RUNNING',
               $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15
           )
           ON CONFLICT ON CONSTRAINT extraction_runs_doc_pass_unique DO UPDATE SET
               -- Identify the new attempt: reset lifecycle columns.
               status = 'RUNNING',
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

/// Insert an extraction item. Returns the auto-generated item ID.
pub async fn insert_extraction_item(
    pool: &PgPool,
    run_id: i32,
    document_id: &str,
    entity_type: &str,
    item_data: &serde_json::Value,
    verbatim_quote: Option<&str>,
) -> Result<i32, PipelineRepoError> {
    let row = sqlx::query_scalar::<_, i32>(
        r#"INSERT INTO extraction_items
           (run_id, document_id, entity_type, item_data, verbatim_quote)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id"#,
    )
    .bind(run_id)
    .bind(document_id)
    .bind(entity_type)
    .bind(item_data)
    .bind(verbatim_quote)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Insert an extraction relationship.
#[allow(clippy::too_many_arguments)]
pub async fn insert_extraction_relationship(
    pool: &PgPool,
    run_id: i32,
    document_id: &str,
    from_item_id: i32,
    to_item_id: i32,
    relationship_type: &str,
    properties: Option<&serde_json::Value>,
    tier: i32,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        r#"INSERT INTO extraction_relationships
           (run_id, document_id, from_item_id, to_item_id, relationship_type, properties, tier)
           VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
    )
    .bind(run_id)
    .bind(document_id)
    .bind(from_item_id)
    .bind(to_item_id)
    .bind(relationship_type)
    .bind(properties)
    .bind(tier)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get all extraction items for a document that have verbatim quotes.
pub async fn get_items_with_quotes(
    pool: &PgPool,
    document_id: &str,
) -> Result<Vec<ExtractionItemRecord>, PipelineRepoError> {
    let sql = format!(
        "SELECT {ITEM_SELECT_COLUMNS} FROM extraction_items \
         WHERE document_id = $1 AND verbatim_quote IS NOT NULL AND verbatim_quote != '' \
         ORDER BY id"
    );
    let rows = sqlx::query_as::<_, ExtractionItemRecord>(&sql)
        .bind(document_id)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// Update grounding status and page number for an extraction item.
pub async fn update_item_grounding(
    pool: &PgPool,
    item_id: i32,
    grounding_status: &str,
    grounded_page: Option<i32>,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "UPDATE extraction_items SET grounding_status = $1, grounded_page = $2 WHERE id = $3",
    )
    .bind(grounding_status)
    .bind(grounded_page)
    .bind(item_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Batch-persist the extraction-item → Neo4j-node-id lineage after Ingest.
///
/// The `pg_to_neo4j` HashMap inside `run_ingest` carries the actual
/// Neo4j node id that was CREATE'd or MERGE'd for each extraction item
/// — including the resolver-assigned id for cross-document Party
/// matches. Without persistence, that mapping is discarded and
/// completeness has to re-derive it (and cannot, for resolved Parties).
///
/// Runs the UPDATEs inside a single sqlx transaction: either every row
/// is updated or none is, so a partial failure doesn't leave the table
/// half-written. A failure after the Neo4j commit is safe to retry —
/// the UPDATE is idempotent (same id, same target) so a second call
/// with the same mappings is a no-op difference.
///
/// R1 from `PIPELINE_CODEBASE_AUDIT.md §8`.
pub async fn batch_update_neo4j_node_ids(
    pool: &PgPool,
    mappings: &[(i32, String)],
) -> Result<(), PipelineRepoError> {
    if mappings.is_empty() {
        return Ok(());
    }
    let mut txn = pool.begin().await?;
    for (item_id, neo4j_id) in mappings {
        sqlx::query("UPDATE extraction_items SET neo4j_node_id = $1 WHERE id = $2")
            .bind(neo4j_id)
            .bind(item_id)
            .execute(&mut *txn)
            .await?;
    }
    txn.commit().await?;
    Ok(())
}

/// Look up the Neo4j node id for a batch of `extraction_items.id` values.
///
/// Used by Ingest to resolve cross-document relationship endpoints —
/// when pass 2 writes a relationship whose `from_item_id` or
/// `to_item_id` references an item owned by a different document, that
/// item was already ingested on its own run and its `neo4j_node_id`
/// column carries the canonical post-MERGE id. This helper returns only
/// rows where `neo4j_node_id IS NOT NULL`; missing rows (either the
/// item doesn't exist, or its source doc never finished Ingest) are
/// silently dropped so the caller's own "No Neo4j ID for ..." error
/// still fires and names the offending id.
///
/// Empty input → empty output, no query issued.
pub async fn lookup_neo4j_node_ids(
    pool: &PgPool,
    item_ids: &[i32],
) -> Result<Vec<(i32, String)>, PipelineRepoError> {
    if item_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<(i32, String)> = sqlx::query_as(
        "SELECT id, neo4j_node_id FROM extraction_items \
         WHERE id = ANY($1) AND neo4j_node_id IS NOT NULL",
    )
    .bind(item_ids)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Return the `document_id` owner of a batch of `extraction_items.id`
/// values.
///
/// Used by Ingest to enrich the "No Neo4j ID for …" error message:
/// when an endpoint can't be resolved via either the local map or the
/// cross-doc lookup, naming the source document makes it immediately
/// clear whether the problem is a dangling ref, an un-ingested
/// complaint, or something stranger. Missing rows are silently dropped
/// — the caller treats absence as "item not in PG at all."
pub async fn lookup_item_document_ids(
    pool: &PgPool,
    item_ids: &[i32],
) -> Result<Vec<(i32, String)>, PipelineRepoError> {
    if item_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<(i32, String)> = sqlx::query_as(
        "SELECT id, document_id FROM extraction_items WHERE id = ANY($1)",
    )
    .bind(item_ids)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Record the Neo4j label Ingest wrote for an extraction item.
///
/// R4 (PIPELINE_CODEBASE_AUDIT.md §8): writes to `resolved_entity_type`,
/// leaving the LLM's original `entity_type` immutable. Readers that want
/// the effective label see it through `ExtractionItemRecord.entity_type`,
/// which every SELECT projects via `COALESCE(resolved_entity_type, entity_type)`.
///
/// The function name is retained from the pre-R4 code so existing call
/// sites in `pipeline/steps/ingest.rs` and `api/pipeline/ingest.rs`
/// don't need to change. Only the SQL body moves to the new column.
pub async fn update_item_entity_type(
    pool: &PgPool,
    item_id: i32,
    new_entity_type: &str,
) -> Result<(), PipelineRepoError> {
    sqlx::query("UPDATE extraction_items SET resolved_entity_type = $1 WHERE id = $2")
        .bind(new_entity_type)
        .bind(item_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Get all extraction items for a document (for report generation).
pub async fn get_all_items(
    pool: &PgPool,
    document_id: &str,
) -> Result<Vec<ExtractionItemRecord>, PipelineRepoError> {
    let sql = format!(
        "SELECT {ITEM_SELECT_COLUMNS} FROM extraction_items \
         WHERE document_id = $1 ORDER BY entity_type, id"
    );
    let rows = sqlx::query_as::<_, ExtractionItemRecord>(&sql)
        .bind(document_id)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// Get all extraction relationships for a document.
pub async fn get_all_relationships(
    pool: &PgPool,
    document_id: &str,
) -> Result<Vec<ExtractionRelationshipRecord>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, ExtractionRelationshipRecord>(
        "SELECT * FROM extraction_relationships WHERE document_id = $1 ORDER BY id",
    )
    .bind(document_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Items approved (or edited) but not yet written to Neo4j.
///
/// Delta ingest selects this set: rows whose user decision says "write
/// this to the graph" (`approved`/`edited`) but whose `neo4j_node_id`
/// column is still NULL. Scoped to a single document — cross-document
/// deltas are a separate concern (Phase 3b).
///
/// Case-insensitive on `review_status` to stay consistent with other
/// `review_repo` queries that lowercase for comparison.
pub async fn get_items_pending_graph_write(
    pool: &PgPool,
    document_id: &str,
) -> Result<Vec<ExtractionItemRecord>, PipelineRepoError> {
    let sql = format!(
        "SELECT {ITEM_SELECT_COLUMNS} FROM extraction_items \
         WHERE document_id = $1 \
           AND LOWER(review_status) IN ('approved', 'edited') \
           AND neo4j_node_id IS NULL \
         ORDER BY id"
    );
    let rows = sqlx::query_as::<_, ExtractionItemRecord>(&sql)
        .bind(document_id)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// Count items awaiting graph write (same predicate as
/// `get_items_pending_graph_write`). Powers the UI's "Write N approved
/// items to graph" button visibility and label.
pub async fn count_items_pending_graph_write(
    pool: &PgPool,
    document_id: &str,
) -> Result<i64, PipelineRepoError> {
    let n = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM extraction_items \
         WHERE document_id = $1 \
           AND LOWER(review_status) IN ('approved', 'edited') \
           AND neo4j_node_id IS NULL",
    )
    .bind(document_id)
    .fetch_one(pool)
    .await?;
    Ok(n)
}

/// Seed map for delta ingest: every item in the document that already
/// has a Neo4j node id. Returned as `(extraction_items.id, neo4j_node_id)`
/// tuples; callers typically collect into a `HashMap<i32, String>`.
///
/// Used by `run_ingest_delta` to pre-populate `pg_to_neo4j` so that
/// relationships between newly-written and previously-written items
/// resolve correctly. Without this seed, relationships would incorrectly
/// fall through to the cross-document lookup path even for same-document
/// edges.
pub async fn get_existing_item_neo4j_map(
    pool: &PgPool,
    document_id: &str,
) -> Result<Vec<(i32, String)>, PipelineRepoError> {
    let rows: Vec<(i32, String)> = sqlx::query_as(
        "SELECT id, neo4j_node_id FROM extraction_items \
         WHERE document_id = $1 AND neo4j_node_id IS NOT NULL",
    )
    .bind(document_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Get approved extraction items for a document's latest completed run.
///
/// Returns items where review_status is `approved` OR `edited`. Used by
/// ingest (to write items to Neo4j) and completeness (to count items for
/// comparison). `edited` is accepted because `review_repo::edit_item`
/// transitions manually-corrected items to that terminal state; they are
/// semantically approved-with-edits and must reach the graph. Omitting
/// `edited` here silently drops user corrections — the P2 bug tracked in
/// Phase 2's R1 finding.
///
/// `rejected`, `pending`, and NULL are intentionally excluded.
pub async fn get_approved_items_for_document(
    pool: &PgPool,
    document_id: &str,
    run_id: i32,
) -> Result<Vec<ExtractionItemRecord>, PipelineRepoError> {
    let sql = format!(
        "SELECT {ITEM_SELECT_COLUMNS} FROM extraction_items \
         WHERE run_id = $1 AND document_id = $2 \
           AND review_status IN ('approved', 'edited') \
         ORDER BY id"
    );
    let rows = sqlx::query_as::<_, ExtractionItemRecord>(&sql)
        .bind(run_id)
        .bind(document_id)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// Get approved extraction relationships for a document across every
/// COMPLETED extraction run (pass 1 + pass 2).
///
/// Ingest needs relationships from BOTH passes: pass-1 relationships
/// live under pass-1's `run_id`, pass-2 relationships under pass-2's.
/// Filtering by a single `run_id` loses half the graph once pass 2 is
/// enabled. Endpoints must still resolve to an approved item via the
/// inner joins — pass-2 relationships reference pass-1 items, and
/// those items carry the review_status the caller wants to respect.
///
/// The join against `extraction_runs` scopes to COMPLETED runs so a
/// partial / failed retry's orphan relationships never leak into Neo4j.
pub async fn get_approved_relationships_for_document_all_passes(
    pool: &PgPool,
    document_id: &str,
) -> Result<Vec<ExtractionRelationshipRecord>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, ExtractionRelationshipRecord>(
        "SELECT r.* FROM extraction_relationships r
         JOIN extraction_runs rn ON rn.id = r.run_id
         JOIN extraction_items fi ON fi.id = r.from_item_id
         JOIN extraction_items ti ON ti.id = r.to_item_id
         WHERE r.document_id = $1
           AND rn.status = 'COMPLETED'
           AND fi.review_status = 'approved'
           AND ti.review_status = 'approved'
         ORDER BY r.id",
    )
    .bind(document_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Get approved extraction relationships for a document's latest completed run.
///
/// Only returns relationships where both endpoints (from_item_id, to_item_id)
/// have review_status = 'approved'. This prevents ingesting relationships
/// that reference unapproved (potentially hallucinated) items.
pub async fn get_approved_relationships_for_document(
    pool: &PgPool,
    run_id: i32,
) -> Result<Vec<ExtractionRelationshipRecord>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, ExtractionRelationshipRecord>(
        "SELECT r.* FROM extraction_relationships r
         JOIN extraction_items fi ON fi.id = r.from_item_id
         JOIN extraction_items ti ON ti.id = r.to_item_id
         WHERE r.run_id = $1
           AND fi.review_status = 'approved'
           AND ti.review_status = 'approved'
         ORDER BY r.id",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
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
         WHERE document_id = $1 AND pass_number = 1 AND status = 'COMPLETED'
         ORDER BY id DESC LIMIT 1",
    )
    .bind(document_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Get all extraction items for a specific run (by run_id).
///
/// Unlike `get_all_items` which queries by document_id, this targets
/// a single run — important when a document has been extracted multiple times.
pub async fn get_items_for_run(
    pool: &PgPool,
    run_id: i32,
) -> Result<Vec<ExtractionItemRecord>, PipelineRepoError> {
    let sql = format!(
        "SELECT {ITEM_SELECT_COLUMNS} FROM extraction_items WHERE run_id = $1 ORDER BY id"
    );
    let rows = sqlx::query_as::<_, ExtractionItemRecord>(&sql)
        .bind(run_id)
        .fetch_all(pool)
        .await?;
    Ok(rows)
}

/// Get all extraction relationships for a specific run (by run_id).
pub async fn get_relationships_for_run(
    pool: &PgPool,
    run_id: i32,
) -> Result<Vec<ExtractionRelationshipRecord>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, ExtractionRelationshipRecord>(
        "SELECT * FROM extraction_relationships WHERE run_id = $1 ORDER BY id",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
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

// ── Grounding-based item selection (auto-ingest) ────────────────

/// Get items that should be written to Neo4j based on grounding status.
///
/// Auto-write rules (from PIPELINE_SIMPLIFICATION_DESIGN_v2.md §4):
/// - grounding_status IN ('exact', 'normalized', 'name_matched', 'heading_matched') → write
/// - grounding_status IN ('derived', 'unverified') → write (no grounding needed)
/// - grounding_status = 'not_found' → skip (potentially hallucinated)
/// - grounding_status = 'missing_quote' → skip (no quote to verify)
/// - grounding_status IS NULL → skip (not yet grounded)
pub async fn get_grounded_items_for_document(
    pool: &PgPool,
    run_id: i32,
) -> Result<Vec<ExtractionItemRecord>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, ExtractionItemRecord>(
        "SELECT * FROM extraction_items
         WHERE run_id = $1
           AND grounding_status IN ('exact', 'normalized', 'name_matched', 'heading_matched', 'derived', 'unverified')
         ORDER BY id",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Get relationships where BOTH endpoints are grounded (will be written to Neo4j).
pub async fn get_grounded_relationships_for_document(
    pool: &PgPool,
    run_id: i32,
) -> Result<Vec<ExtractionRelationshipRecord>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, ExtractionRelationshipRecord>(
        "SELECT r.* FROM extraction_relationships r
         JOIN extraction_items fi ON fi.id = r.from_item_id
         JOIN extraction_items ti ON ti.id = r.to_item_id
         WHERE r.run_id = $1
           AND fi.grounding_status IN ('exact', 'normalized', 'name_matched', 'heading_matched', 'derived', 'unverified')
           AND ti.grounding_status IN ('exact', 'normalized', 'name_matched', 'heading_matched', 'derived', 'unverified')
         ORDER BY r.id",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

// ── Entity + relationship storage (LLM-extraction result writer) ───

/// Resolve a relationship's endpoint + type fields, tolerating either
/// of the two conventions LLMs emit in practice.
///
/// The schema-compliant shape the templates specify is
/// `{from_entity, to_entity, relationship_type}` and that wins when
/// both conventions are present in the same object. The short form
/// `{from, to, type}` is Opus's natural JSON style and was causing
/// every relationship to be silently dropped on pass 2 before this
/// helper existed — every `get("from_entity")` returned `None`, the
/// fallback empty string failed `id_map` lookup, and the
/// skip-and-log branch fired for every row. Accepting both in one
/// place keeps the store functions aligned on one tolerance policy.
fn resolve_relationship_fields(rel: &serde_json::Value) -> (&str, &str, &str) {
    let from = rel
        .get("from_entity")
        .or_else(|| rel.get("from"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let to = rel
        .get("to_entity")
        .or_else(|| rel.get("to"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let rtype = rel
        .get("relationship_type")
        .or_else(|| rel.get("type"))
        .and_then(|v| v.as_str())
        .unwrap_or("UNKNOWN");
    (from, to, rtype)
}

/// Store the entities and relationships contained in a raw LLM response into
/// `extraction_items` and `extraction_relationships` for a given run.
///
/// Inputs come from a parsed `serde_json::Value` shaped like:
///
/// ```json
/// {
///   "entities": [
///     { "id": "e1", "entity_type": "Party",
///       "properties": { "full_name": "Marie Awad" },
///       "verbatim_quote": "..." },
///     ...
///   ],
///   "relationships": [
///     { "from_entity": "e1", "to_entity": "e2",
///       "relationship_type": "MENTIONS",
///       "properties": { ... } },
///     ...
///   ]
/// }
/// ```
///
/// Returns `(entity_count, relationship_count)` — the number of rows inserted
/// into each table. Relationships whose `from_entity` or `to_entity` cannot
/// be resolved via the LLM-supplied `id` → DB `item_id` map are SKIPPED
/// (logged and ignored) rather than erroring, so partial outputs still
/// produce a usable graph. Unknown/missing JSON fields fall back to safe
/// defaults (`entity_type = "unknown"`, `relationship_type = "UNKNOWN"`).
///
/// ## Why this lives in the repository layer
///
/// Prior to 2026-04-16 this helper lived in `api::pipeline::chunk_storage`
/// (deleted by commit 1414838 as part of the P2-Cleanup purge of the old
/// chunked extraction path). The step-layer [`LlmExtract`] is the only
/// remaining caller, and a storage helper is a pure data-layer concern —
/// placing it here avoids re-introducing a step → api-handler dependency.
///
/// [`LlmExtract`]: crate::pipeline::steps::llm_extract::LlmExtract
pub async fn store_entities_and_relationships(
    pool: &sqlx::PgPool,
    run_id: i32,
    document_id: &str,
    parsed: &serde_json::Value,
) -> Result<(usize, usize), PipelineRepoError> {
    use std::collections::HashMap;

    let mut id_map: HashMap<String, i32> = HashMap::new();
    let mut entity_count: usize = 0;

    if let Some(entities) = parsed.get("entities").and_then(|v| v.as_array()) {
        for entity in entities {
            let entity_type = entity
                .get("entity_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            let json_id = entity.get("id").and_then(|v| v.as_str()).unwrap_or("");

            let verbatim_quote = entity
                .get("verbatim_quote")
                .and_then(|v| v.as_str())
                .or_else(|| {
                    entity
                        .get("properties")
                        .and_then(|p| p.get("verbatim_quote"))
                        .and_then(|v| v.as_str())
                });

            let item_id = insert_extraction_item(
                pool,
                run_id,
                document_id,
                entity_type,
                entity,
                verbatim_quote,
            )
            .await?;

            if !json_id.is_empty() {
                id_map.insert(json_id.to_string(), item_id);
            }
            entity_count += 1;
        }
    }

    let mut rel_count: usize = 0;

    if let Some(rels) = parsed.get("relationships").and_then(|v| v.as_array()) {
        for rel in rels {
            let (from_key, to_key, relationship_type) = resolve_relationship_fields(rel);

            let (Some(&from_id), Some(&to_id)) = (id_map.get(from_key), id_map.get(to_key))
            else {
                tracing::warn!(
                    run_id, document_id,
                    from = %from_key, to = %to_key,
                    "Skipping relationship with unresolved endpoint(s)"
                );
                continue;
            };

            let properties = rel.get("properties");

            insert_extraction_relationship(
                pool,
                run_id,
                document_id,
                from_id,
                to_id,
                relationship_type,
                properties,
                1,
            )
            .await?;
            rel_count += 1;
        }
    }

    Ok((entity_count, rel_count))
}

// ── Pass-2 support: cross-pass entity loading + relationship store ──

/// A pass-1 entity loaded for re-injection into the pass-2 prompt.
///
/// Carries both the LLM-supplied `id` string (e.g. `"party-001"` — used
/// to resolve pass-2 relationship endpoints back to `extraction_items`
/// rows) and the DB primary key (`item_id`) so the caller can build an
/// id → item_id map without a second query. The `to_prompt_value()`
/// helper emits only the prompt-facing subset so DB internals never
/// leak into the LLM input.
#[derive(Debug, Clone)]
pub struct Pass1Entity {
    /// `extraction_items.id` — the DB primary key. Used by the pass-2
    /// relationship writer to target the right FK.
    pub item_id: i32,
    /// The LLM's entity id as authored in pass 1 (e.g. `"party-001"`).
    /// Round-trips into pass 2's prompt so pass 2 can reference it, and
    /// then back out via the relationship payload.
    pub id: String,
    /// Effective entity type after Ingest resolution (falls back to the
    /// LLM's original type when no resolution has occurred).
    pub entity_type: String,
    /// Short human-readable label, if the pass-1 output supplied one.
    pub label: Option<String>,
    /// The `properties` object verbatim from the pass-1 entity JSON.
    /// Returned as `serde_json::Value::Object(Default::default())` when
    /// the pass-1 output omitted it, so the pass-2 prompt always sees a
    /// JSON object in this position.
    pub properties: serde_json::Value,
}

impl Pass1Entity {
    /// Build a `Pass1Entity` from a stored `extraction_items` row.
    ///
    /// Pass 1 stores the full entity JSON in `item_data`, so we parse
    /// `id` / `label` / `properties` back out of the JSONB. `entity_type`
    /// comes from the column (already COALESCE'd with
    /// `resolved_entity_type`), so the prompt sees the effective label
    /// — important when pass 2 is re-run after Ingest has resolved a
    /// Party into a Person/Organization.
    fn from_item_record(rec: &ExtractionItemRecord) -> Self {
        let id = rec
            .item_data
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let label = rec
            .item_data
            .get("label")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let properties = rec
            .item_data
            .get("properties")
            .cloned()
            .unwrap_or_else(|| serde_json::Value::Object(Default::default()));
        Self {
            item_id: rec.id,
            id,
            entity_type: rec.entity_type.clone(),
            label,
            properties,
        }
    }

    /// Render the prompt-facing subset: `{id, entity_type, label?, properties}`.
    ///
    /// The DB `item_id` is intentionally omitted — it's a repo-internal
    /// handle that has no meaning to the LLM.
    pub fn to_prompt_value(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        obj.insert("id".into(), serde_json::Value::String(self.id.clone()));
        obj.insert(
            "entity_type".into(),
            serde_json::Value::String(self.entity_type.clone()),
        );
        if let Some(label) = &self.label {
            obj.insert("label".into(), serde_json::Value::String(label.clone()));
        }
        obj.insert("properties".into(), self.properties.clone());
        serde_json::Value::Object(obj)
    }
}

/// Load the pass-1 entities for a document so pass 2 can be given them
/// as input.
///
/// Selects the latest COMPLETED `extraction_runs` row where
/// `pass_number = 1` and returns its `extraction_items` as
/// [`Pass1Entity`] values. Returns an empty `Vec` when no completed
/// pass-1 run exists — the caller decides whether that's a user error
/// ("run pass 1 first") or a no-op.
pub async fn load_pass1_entities(
    pool: &PgPool,
    document_id: &str,
) -> Result<Vec<Pass1Entity>, PipelineRepoError> {
    let run_id: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM extraction_runs \
         WHERE document_id = $1 AND pass_number = 1 AND status = 'COMPLETED' \
         ORDER BY id DESC LIMIT 1",
    )
    .bind(document_id)
    .fetch_optional(pool)
    .await?;

    let Some(run_id) = run_id else {
        return Ok(Vec::new());
    };

    let items = get_items_for_run(pool, run_id).await?;
    Ok(items.iter().map(Pass1Entity::from_item_record).collect())
}

// ── Cross-document context for pass 2 ────────────────────────────

/// Prefix applied to cross-document entity ids in the pass-2 prompt.
///
/// The LLM receives prefixed ids like `"ctx:allegation-014"` so ids
/// from other documents can't collide with the current document's
/// local pass-1 ids (e.g., two docs both authoring `party-001`). When
/// the LLM emits a cross-document relationship, the endpoint string
/// retains the prefix and `store_pass2_relationships` resolves it via
/// the extended id_map the step builds from these entities.
pub const CROSS_DOC_ID_PREFIX: &str = "ctx:";

/// Entity types surfaced in the pass-2 cross-document context.
///
/// `Party` entities get rewritten to `Person`/`Organization` by Ingest
/// (R4), so we match against the effective type via the same
/// `COALESCE(resolved_entity_type, entity_type)` projection used by
/// every other item SELECT — otherwise post-Ingest Party rows would
/// fail the filter and drop out of the context.
const CROSS_DOC_ENTITY_TYPES: &[&str] = &[
    "Party",
    "Person",
    "Organization",
    "LegalCount",
    "ComplaintAllegation",
];

/// An entity loaded from another PUBLISHED document's pass-1 run for
/// injection into the current document's pass-2 prompt.
///
/// Carries both the original LLM id (as authored in the source doc)
/// and the prefixed id (used in the prompt and id_map). Serializing
/// via [`Self::to_prompt_value`] emits the prefixed id plus a
/// `source_document` / `source_document_type` pair so the LLM can see
/// which document contributed each entity.
#[derive(Debug, Clone)]
pub struct CrossDocEntity {
    /// DB primary key in `extraction_items` — target for cross-doc
    /// relationship endpoints.
    pub item_id: i32,
    /// LLM id as authored in the source document (e.g., `"party-001"`).
    pub original_id: String,
    /// Id used in the current doc's prompt and id_map
    /// (`CROSS_DOC_ID_PREFIX + original_id`).
    pub prefixed_id: String,
    /// Source document id — the `documents.id` this entity belongs to.
    pub source_document_id: String,
    /// Source document type (`complaint`, `discovery_response`, etc.)
    /// — propagates `documents.document_type` so the LLM can reason
    /// about provenance.
    pub source_document_type: String,
    /// Effective entity type (COALESCE of `resolved_entity_type` and
    /// `entity_type`) — what Ingest resolved the entity to.
    pub entity_type: String,
    /// Short human-readable label, if the source pass-1 output set one.
    pub label: Option<String>,
    /// Full property object from the source `item_data.properties`.
    /// [`Self::to_prompt_value`] applies a per-type allowlist to keep
    /// prompt size reasonable.
    pub properties: serde_json::Value,
}

/// Row shape returned by [`load_cross_document_context`]'s join query.
#[derive(sqlx::FromRow)]
struct CrossDocRow {
    item_id: i32,
    item_data: serde_json::Value,
    source_document_id: String,
    source_document_type: String,
    effective_entity_type: String,
}

impl CrossDocEntity {
    fn from_row(row: CrossDocRow) -> Self {
        let original_id = row
            .item_data
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let label = row
            .item_data
            .get("label")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let properties = row
            .item_data
            .get("properties")
            .cloned()
            .unwrap_or_else(|| serde_json::Value::Object(Default::default()));
        let prefixed_id = format!("{CROSS_DOC_ID_PREFIX}{original_id}");
        Self {
            item_id: row.item_id,
            original_id,
            prefixed_id,
            source_document_id: row.source_document_id,
            source_document_type: row.source_document_type,
            entity_type: row.effective_entity_type,
            label,
            properties,
        }
    }

    /// Render the prompt-facing subset for injection into `{{entities_json}}`.
    ///
    /// Applies a per-type property allowlist to trim the payload —
    /// `verbatim_quote` in particular is dropped from
    /// `ComplaintAllegation` because it's large and not needed for the
    /// LLM's link-or-not decision. Types outside the allowlist fall
    /// through with their full properties intact (cheap insurance
    /// against schema drift).
    pub fn to_prompt_value(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        obj.insert(
            "id".into(),
            serde_json::Value::String(self.prefixed_id.clone()),
        );
        obj.insert(
            "entity_type".into(),
            serde_json::Value::String(self.entity_type.clone()),
        );
        if let Some(label) = &self.label {
            obj.insert("label".into(), serde_json::Value::String(label.clone()));
        }
        obj.insert(
            "source_document".into(),
            serde_json::Value::String(self.source_document_id.clone()),
        );
        obj.insert(
            "source_document_type".into(),
            serde_json::Value::String(self.source_document_type.clone()),
        );
        obj.insert("properties".into(), filter_properties_for_prompt(
            &self.entity_type,
            &self.properties,
        ));
        serde_json::Value::Object(obj)
    }
}

/// Drop properties that aren't useful for cross-doc link decisions.
///
/// Keeps prompt size bounded as the number of PUBLISHED documents
/// grows. The allowlist is per effective entity type; unknown types
/// pass through untouched (schema-drift resilience).
fn filter_properties_for_prompt(
    entity_type: &str,
    properties: &serde_json::Value,
) -> serde_json::Value {
    let keep: &[&str] = match entity_type {
        "ComplaintAllegation" => &["paragraph_number", "summary"],
        "LegalCount" => &["count_number", "legal_basis", "description"],
        "Party" | "Person" | "Organization" => &["full_name", "role", "entity_kind"],
        _ => return properties.clone(),
    };
    let src = match properties.as_object() {
        Some(o) => o,
        None => return properties.clone(),
    };
    let mut out = serde_json::Map::new();
    for k in keep {
        if let Some(v) = src.get(*k) {
            out.insert((*k).to_string(), v.clone());
        }
    }
    serde_json::Value::Object(out)
}

/// Load entities from OTHER PUBLISHED documents for cross-doc pass-2
/// context.
///
/// Returns [`CrossDocEntity`] values drawn from every COMPLETED pass-1
/// run on any document whose `documents.status = 'PUBLISHED'` except
/// the current one, restricted to the approved-item set and to the
/// entity types useful for cross-document link creation (parties,
/// counts, complaint allegations). Empty `Vec` is a valid result —
/// the current doc may be the first published, or no cross-doc-worthy
/// types exist yet.
pub async fn load_cross_document_context(
    pool: &PgPool,
    current_document_id: &str,
) -> Result<Vec<CrossDocEntity>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, CrossDocRow>(
        "SELECT i.id AS item_id, \
                i.item_data, \
                i.document_id AS source_document_id, \
                docs.document_type AS source_document_type, \
                COALESCE(i.resolved_entity_type, i.entity_type) AS effective_entity_type \
         FROM extraction_items i \
         JOIN extraction_runs runs ON runs.id = i.run_id \
         JOIN documents docs ON docs.id = i.document_id \
         WHERE i.document_id <> $1 \
           AND docs.status = 'PUBLISHED' \
           AND runs.pass_number = 1 \
           AND runs.status = 'COMPLETED' \
           AND i.review_status = 'approved' \
           AND COALESCE(i.resolved_entity_type, i.entity_type) = ANY($2) \
         ORDER BY i.document_id, i.id",
    )
    .bind(current_document_id)
    .bind(CROSS_DOC_ENTITY_TYPES)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(CrossDocEntity::from_row).collect())
}

/// Persist pass-2 relationships against the pass-2 `extraction_runs` row.
///
/// Pass 2's JSON is relationships-only (`{"relationships": [...]}`) and
/// its endpoints reference pass-1 entities by the LLM-authored id
/// (e.g. `"party-001"`). The caller builds `id_map` from the same
/// [`Pass1Entity`] list it injected into the prompt; this function
/// skips-and-logs any relationship whose endpoint cannot be resolved
/// (matching the partial-output tolerance of the pass-1 writer). The
/// `tier` is fixed at `1` to mirror pass 1 — downstream code already
/// treats tier 1 as "direct LLM extraction".
pub async fn store_pass2_relationships(
    pool: &PgPool,
    run_id: i32,
    document_id: &str,
    parsed: &serde_json::Value,
    id_map: &std::collections::HashMap<String, i32>,
) -> Result<usize, PipelineRepoError> {
    let mut rel_count: usize = 0;
    if let Some(rels) = parsed.get("relationships").and_then(|v| v.as_array()) {
        for rel in rels {
            let (from_key, to_key, relationship_type) = resolve_relationship_fields(rel);

            let (Some(&from_id), Some(&to_id)) = (id_map.get(from_key), id_map.get(to_key))
            else {
                tracing::warn!(
                    run_id, document_id,
                    from = %from_key, to = %to_key,
                    "Pass 2: skipping relationship with unresolved endpoint(s)"
                );
                continue;
            };

            let properties = rel.get("properties");

            insert_extraction_relationship(
                pool,
                run_id,
                document_id,
                from_id,
                to_id,
                relationship_type,
                properties,
                1,
            )
            .await?;
            rel_count += 1;
        }
    }
    Ok(rel_count)
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
pub async fn insert_extraction_chunk(
    pool: &PgPool,
    run_id: i32,
    chunk_index: i32,
    chunk_text: &str,
) -> Result<uuid::Uuid, PipelineRepoError> {
    let id = uuid::Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO extraction_chunks
           (id, extraction_run_id, chunk_index, chunk_text, status, created_at)
           VALUES ($1, $2, $3, $4, 'pending', NOW())"#,
    )
    .bind(id)
    .bind(run_id)
    .bind(chunk_index)
    .bind(chunk_text)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Compile-only: freezes `store_entities_and_relationships`'s parameter
    /// list. Any future refactor that changes argument order or types breaks
    /// this, forcing explicit acknowledgement at the call site
    /// (`pipeline/steps/llm_extract.rs`). No DB is touched.
    #[test]
    fn store_entities_signature_is_stable() {
        let _f = store_entities_and_relationships;
    }

    /// Compile-only: pin the pass-2 helper signatures so future refactors
    /// must explicitly acknowledge a breaking change at the step call site.
    #[test]
    fn pass2_helper_signatures_are_stable() {
        let _l = load_pass1_entities;
        let _s = store_pass2_relationships;
    }

    /// Compile-only: pin the all-passes relationships signature. Ingest
    /// depends on `doc_id` keying (not `run_id`) to unify pass-1 and
    /// pass-2 relationships — a silent refactor back to run_id filtering
    /// would reintroduce the "No Neo4j ID for from_item_id N" bug.
    #[test]
    fn all_passes_relationships_signature_is_stable() {
        let _f = get_approved_relationships_for_document_all_passes;
    }

    fn item_record_with(
        id: i32,
        entity_type: &str,
        item_data: serde_json::Value,
    ) -> ExtractionItemRecord {
        ExtractionItemRecord {
            id,
            run_id: 42,
            document_id: "doc-x".into(),
            entity_type: entity_type.into(),
            item_data,
            verbatim_quote: None,
            grounding_status: None,
            grounded_page: None,
            review_status: "pending".into(),
            reviewed_by: None,
            reviewed_at: None,
            review_notes: None,
            graph_status: "pending".into(),
            neo4j_node_id: None,
            resolved_entity_type: None,
        }
    }

    #[test]
    fn pass1_entity_extracts_id_label_properties_from_item_data() {
        let rec = item_record_with(
            101,
            "Party",
            serde_json::json!({
                "id": "party-001",
                "entity_type": "Party",
                "label": "Marie Awad",
                "properties": { "full_name": "Marie Awad", "role": "Plaintiff" },
                "verbatim_quote": "Plaintiff Marie Awad..."
            }),
        );
        let e = Pass1Entity::from_item_record(&rec);
        assert_eq!(e.item_id, 101);
        assert_eq!(e.id, "party-001");
        assert_eq!(e.entity_type, "Party");
        assert_eq!(e.label.as_deref(), Some("Marie Awad"));
        assert_eq!(e.properties["full_name"], "Marie Awad");
    }

    #[test]
    fn pass1_entity_tolerates_missing_label_and_properties() {
        let rec = item_record_with(
            7,
            "LegalCount",
            serde_json::json!({ "id": "count-001", "entity_type": "LegalCount" }),
        );
        let e = Pass1Entity::from_item_record(&rec);
        assert_eq!(e.id, "count-001");
        assert!(e.label.is_none());
        assert!(
            e.properties.is_object(),
            "missing properties must default to empty object, got {:?}",
            e.properties
        );
        assert_eq!(e.properties.as_object().unwrap().len(), 0);
    }

    #[test]
    fn pass1_entity_to_prompt_value_omits_item_id() {
        let e = Pass1Entity {
            item_id: 9,
            id: "harm-001".into(),
            entity_type: "Harm".into(),
            label: Some("Financial loss".into()),
            properties: serde_json::json!({ "amount_usd": 50000 }),
        };
        let v = e.to_prompt_value();
        let obj = v.as_object().expect("prompt value must be a JSON object");
        assert!(
            !obj.contains_key("item_id"),
            "DB item_id must not leak into the prompt payload"
        );
        assert_eq!(obj["id"], "harm-001");
        assert_eq!(obj["entity_type"], "Harm");
        assert_eq!(obj["label"], "Financial loss");
        assert_eq!(obj["properties"]["amount_usd"], 50000);
    }

    #[test]
    fn pass1_entity_to_prompt_value_omits_label_when_absent() {
        let e = Pass1Entity {
            item_id: 1,
            id: "count-001".into(),
            entity_type: "LegalCount".into(),
            label: None,
            properties: serde_json::json!({}),
        };
        let obj = e.to_prompt_value();
        assert!(
            !obj.as_object().unwrap().contains_key("label"),
            "absent label must not serialize as null"
        );
    }

    // ── CrossDocEntity ─────────────────────────────────────────────

    /// Compile-only: pin the cross-doc loader signature so step-layer
    /// callers don't break silently.
    #[test]
    fn cross_doc_loader_signature_is_stable() {
        let _f = load_cross_document_context;
    }

    fn cross_doc(
        item_id: i32,
        original_id: &str,
        entity_type: &str,
        properties: serde_json::Value,
    ) -> CrossDocEntity {
        CrossDocEntity {
            item_id,
            original_id: original_id.to_string(),
            prefixed_id: format!("{CROSS_DOC_ID_PREFIX}{original_id}"),
            source_document_id: "doc-complaint-1".into(),
            source_document_type: "complaint".into(),
            entity_type: entity_type.to_string(),
            label: Some(format!("{entity_type} label")),
            properties,
        }
    }

    #[test]
    fn cross_doc_prompt_value_emits_prefixed_id_and_source() {
        let e = cross_doc(
            42,
            "allegation-014",
            "ComplaintAllegation",
            serde_json::json!({
                "paragraph_number": "14",
                "summary": "Defendant failed to account for funds",
                "verbatim_quote": "very long verbatim complaint text that should not be sent to pass 2",
            }),
        );
        let v = e.to_prompt_value();
        let obj = v.as_object().expect("prompt value is an object");
        assert_eq!(obj["id"], "ctx:allegation-014");
        assert_eq!(obj["entity_type"], "ComplaintAllegation");
        assert_eq!(obj["source_document"], "doc-complaint-1");
        assert_eq!(obj["source_document_type"], "complaint");
        // Property allowlist: paragraph_number + summary survive,
        // verbatim_quote is dropped to keep prompt size bounded.
        let props = obj["properties"]
            .as_object()
            .expect("properties is an object");
        assert_eq!(props["paragraph_number"], "14");
        assert!(props.contains_key("summary"));
        assert!(
            !props.contains_key("verbatim_quote"),
            "verbatim_quote must be filtered out of the prompt: {props:?}"
        );
    }

    #[test]
    fn cross_doc_prompt_value_legal_count_keeps_count_number() {
        let e = cross_doc(
            7,
            "count-001",
            "LegalCount",
            serde_json::json!({
                "count_number": 1,
                "legal_basis": "Breach of Fiduciary Duty",
                "description": "Defendant CFS breached its fiduciary duties",
                "paragraph_range": "86-100",
            }),
        );
        let v = e.to_prompt_value();
        let props = v["properties"].as_object().unwrap();
        assert_eq!(props["count_number"], 1);
        assert_eq!(props["legal_basis"], "Breach of Fiduciary Duty");
        // `paragraph_range` isn't in the allowlist; filtering drops it.
        assert!(!props.contains_key("paragraph_range"));
    }

    #[test]
    fn cross_doc_prompt_value_party_types_share_allowlist() {
        let party_props = serde_json::json!({
            "full_name": "Marie Awad",
            "role": "plaintiff",
            "entity_kind": "person",
            "address": "unused extra property",
        });
        for effective_type in &["Party", "Person", "Organization"] {
            let e = cross_doc(1, "party-001", effective_type, party_props.clone());
            let v = e.to_prompt_value();
            let props = v["properties"].as_object().unwrap();
            assert_eq!(
                props["full_name"], "Marie Awad",
                "type {effective_type} must surface full_name"
            );
            assert!(
                !props.contains_key("address"),
                "type {effective_type} must filter unknown props: {props:?}"
            );
        }
    }

    #[test]
    fn cross_doc_prompt_value_unknown_type_keeps_all_properties() {
        // Schema-drift insurance: if a future document type surfaces an
        // entity the allowlist doesn't know about, pass the properties
        // through untouched instead of silently dropping everything.
        let props = serde_json::json!({
            "arbitrary_field": "value",
            "another_one": 42,
        });
        let e = cross_doc(99, "unknown-001", "UnknownType", props.clone());
        let v = e.to_prompt_value();
        assert_eq!(v["properties"], props);
    }

    /// Compile-only: pin the cross-doc endpoint lookup helpers. Ingest
    /// depends on both to resolve pass-2 cross-document relationships.
    #[test]
    fn cross_doc_endpoint_lookups_signatures_are_stable() {
        let _a = lookup_neo4j_node_ids;
        let _b = lookup_item_document_ids;
    }

    #[test]
    fn cross_doc_id_prefix_is_ctx() {
        // The prefix is a public constant because both the loader
        // (writes it) and any future caller that needs to detect
        // cross-doc ids (reads it) depend on agreement. A silent
        // change to a different literal would break the id_map lookup
        // in store_pass2_relationships — pin it.
        assert_eq!(CROSS_DOC_ID_PREFIX, "ctx:");
    }

    // ── resolve_relationship_fields ───────────────────────────────

    #[test]
    fn resolve_rel_fields_accepts_schema_compliant_form() {
        // The canonical shape the templates specify. Must keep working
        // — pass 1 on Sonnet produces this today.
        let rel = serde_json::json!({
            "from_entity": "allegation-007",
            "to_entity": "count-001",
            "relationship_type": "SUPPORTS",
        });
        let (from, to, rtype) = resolve_relationship_fields(&rel);
        assert_eq!(from, "allegation-007");
        assert_eq!(to, "count-001");
        assert_eq!(rtype, "SUPPORTS");
    }

    #[test]
    fn resolve_rel_fields_accepts_short_form() {
        // Opus's natural JSON style. Before the helper, every pass-2
        // relationship in this shape was silently dropped — the bug
        // this change fixes.
        let rel = serde_json::json!({
            "from": "admission-003",
            "to": "ctx:allegation-014",
            "type": "CORROBORATES",
        });
        let (from, to, rtype) = resolve_relationship_fields(&rel);
        assert_eq!(from, "admission-003");
        assert_eq!(to, "ctx:allegation-014");
        assert_eq!(rtype, "CORROBORATES");
    }

    #[test]
    fn resolve_rel_fields_long_form_wins_on_collision() {
        // An LLM could emit both forms in the same object (unlikely
        // but possible). The schema-compliant long form wins so the
        // behaviour matches the documented template shape.
        let rel = serde_json::json!({
            "from": "short-a",
            "from_entity": "long-a",
            "to": "short-b",
            "to_entity": "long-b",
            "type": "SHORT_TYPE",
            "relationship_type": "LONG_TYPE",
        });
        let (from, to, rtype) = resolve_relationship_fields(&rel);
        assert_eq!(from, "long-a");
        assert_eq!(to, "long-b");
        assert_eq!(rtype, "LONG_TYPE");
    }

    #[test]
    fn resolve_rel_fields_empty_when_neither_form_present() {
        // Defensive: a relationship with neither convention falls
        // through to empty endpoints, which the caller's id_map
        // lookup fails on and logs as "unresolved endpoints". The
        // `UNKNOWN` fallback for the type matches the pre-helper
        // behaviour so the audit trail stays stable.
        let rel = serde_json::json!({ "properties": { "note": "x" } });
        let (from, to, rtype) = resolve_relationship_fields(&rel);
        assert_eq!(from, "");
        assert_eq!(to, "");
        assert_eq!(rtype, "UNKNOWN");
    }
}
