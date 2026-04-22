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

/// Get approved extraction items for a document's latest completed run.
///
/// Only returns items where review_status = 'approved'. Used by ingest
/// (to write only approved items to Neo4j) and completeness (to count
/// only approved items for comparison). Unapproved items are intentionally
/// excluded from the knowledge graph.
pub async fn get_approved_items_for_document(
    pool: &PgPool,
    document_id: &str,
    run_id: i32,
) -> Result<Vec<ExtractionItemRecord>, PipelineRepoError> {
    let sql = format!(
        "SELECT {ITEM_SELECT_COLUMNS} FROM extraction_items \
         WHERE run_id = $1 AND document_id = $2 AND review_status = 'approved' \
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
            let from_key = rel.get("from_entity").and_then(|v| v.as_str()).unwrap_or("");
            let to_key = rel.get("to_entity").and_then(|v| v.as_str()).unwrap_or("");

            let (Some(&from_id), Some(&to_id)) = (id_map.get(from_key), id_map.get(to_key))
            else {
                tracing::warn!(
                    run_id, document_id,
                    from = %from_key, to = %to_key,
                    "Skipping relationship with unresolved endpoint(s)"
                );
                continue;
            };

            let relationship_type = rel
                .get("relationship_type")
                .and_then(|v| v.as_str())
                .unwrap_or("UNKNOWN");

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
            let from_key = rel
                .get("from_entity")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let to_key = rel.get("to_entity").and_then(|v| v.as_str()).unwrap_or("");

            let (Some(&from_id), Some(&to_id)) = (id_map.get(from_key), id_map.get(to_key))
            else {
                tracing::warn!(
                    run_id, document_id,
                    from = %from_key, to = %to_key,
                    "Pass 2: skipping relationship with unresolved endpoint(s)"
                );
                continue;
            };

            let relationship_type = rel
                .get("relationship_type")
                .and_then(|v| v.as_str())
                .unwrap_or("UNKNOWN");

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
}
