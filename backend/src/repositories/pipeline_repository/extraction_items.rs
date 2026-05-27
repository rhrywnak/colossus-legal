//! Extraction-item repository functions.
//!
//! Owns the `extraction_items` row type, the canonical column list used
//! by every `SELECT`, and the full CRUD + query surface for items. The
//! grounding/graph-status writeback that `update_graph_status_for_run`
//! (in [`super::extraction_runs`]) flips on these rows is the only
//! cross-module mutation point — the column itself is owned here.
//!
//! Pass-1 entity loading lives in the sibling [`super::extraction_items_pass1`]
//! module so this file stays under the 300-line ceiling (Pass-1 entity
//! deserialisation is a logically distinct concern, even though it
//! reads the same table).

use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use super::PipelineRepoError;

// ── Record type + shared SELECT projection ──────────────────────

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
    /// from before the R4 migration.
    ///
    /// IMPORTANT: this struct's `entity_type` field carries the RAW
    /// LLM-written value — the same string that matches the schema's
    /// `entity_types[].name`. Schema-keyed flows (verify, completeness,
    /// derived-provenance validation) read `entity_type`. Callers that
    /// need the post-resolution Neo4j label (display surfaces, downstream
    /// Neo4j writes) read `resolved_entity_type` explicitly.
    pub resolved_entity_type: Option<String>,
}

/// Shared SELECT column list for every `query_as::<_, ExtractionItemRecord>`
/// call site. Both `entity_type` (raw LLM-written value, matches the
/// schema's `entity_types[].name`) and `resolved_entity_type` (Ingest-
/// determined Neo4j label, populated post-Ingest) are returned. Callers
/// decide which to read: schema-keyed flows read `entity_type`; display
/// and Neo4j-label flows read `resolved_entity_type`.
///
/// Previously projected `entity_type` through
/// `COALESCE(resolved_entity_type, entity_type) AS entity_type`. That
/// silently rewrote the field after Ingest ran: re-verifying an already-
/// ingested document made the schema HashMap lookup miss on every Party
/// item (the rows came back as "Person"/"Organization" instead of
/// "Party"), the silent-default-to-Verbatim path fired, and name_match
/// entities were stamped `grounding_status = "missing_quote"`.
///
/// Visibility note: `pub(super)` rather than private — the
/// [`super::extraction_items_pass1`] sibling uses the same projection
/// for `load_pass1_entities` to keep the SELECT shape identical, so
/// `FromRow` deserialises without column drift.
pub(super) const ITEM_SELECT_COLUMNS: &str = "id, run_id, document_id, entity_type, \
     item_data, verbatim_quote, grounding_status, grounded_page, \
     review_status, reviewed_by, reviewed_at, review_notes, \
     graph_status, neo4j_node_id, resolved_entity_type";

// ── Functions ────────────────────────────────────────────────────

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

/// Fetch a single extraction item by its primary key. `None` if absent.
///
/// The Pass-2 cross-tier persistence path uses this to recover the full
/// `item_data` for an Allegation so it can recompute that entity's stable
/// Neo4j id via [`crate::api::pipeline::ingest_helpers::stable_entity_id`].
/// The prompt-shaped [`super::extraction_items_pass1::Pass1Entity`] keeps only
/// `id`/`label`/`properties`, so it cannot reproduce the `other`-arm hash
/// (which digests the whole `item_data`) — the stored row can.
pub async fn get_extraction_item_by_id(
    executor: impl sqlx::PgExecutor<'_>,
    item_id: i32,
) -> Result<Option<ExtractionItemRecord>, PipelineRepoError> {
    let sql = format!("SELECT {ITEM_SELECT_COLUMNS} FROM extraction_items WHERE id = $1");
    let row = sqlx::query_as::<_, ExtractionItemRecord>(&sql)
        .bind(item_id)
        .fetch_optional(executor)
        .await?;
    Ok(row)
}

/// Update grounding status, page number, and verification reason for an
/// extraction item.
///
/// `verification_reason` is the diagnostic string written alongside
/// `grounding_status='derived_invalid'` (per v5.1 §5.4 derived-provenance
/// validation). For every other status path the caller passes `None`,
/// which clears any stale reason from a prior verify run. This is
/// intentional: a transition `derived_invalid → derived` (after a
/// re-extraction with the missing provenance now present) must not
/// leave the prior failure reason behind to mislead the next reviewer.
///
/// ## Rust Learning: `Option<&str>` vs `&str`
///
/// `Option<&str>` lets the caller pass either a borrowed string slice
/// or `None`. sqlx's `Encode` impl on `Option<T>` writes the SQL `NULL`
/// literal when the value is `None`, so the column ends up NULL on
/// `None` and TEXT-valued on `Some`. No runtime indirection — the
/// `Option` is just a sum type the encoder pattern-matches at bind time.
pub async fn update_item_grounding(
    pool: &PgPool,
    item_id: i32,
    grounding_status: &str,
    grounded_page: Option<i32>,
    verification_reason: Option<&str>,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "UPDATE extraction_items \
         SET grounding_status = $1, grounded_page = $2, verification_reason = $3 \
         WHERE id = $4",
    )
    .bind(grounding_status)
    .bind(grounded_page)
    .bind(verification_reason)
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
    let rows: Vec<(i32, String)> =
        sqlx::query_as("SELECT id, document_id FROM extraction_items WHERE id = ANY($1)")
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

/// Get all extraction items for a specific run (by run_id).
///
/// Unlike `get_all_items` which queries by document_id, this targets
/// a single run — important when a document has been extracted multiple times.
pub async fn get_items_for_run(
    pool: &PgPool,
    run_id: i32,
) -> Result<Vec<ExtractionItemRecord>, PipelineRepoError> {
    let sql =
        format!("SELECT {ITEM_SELECT_COLUMNS} FROM extraction_items WHERE run_id = $1 ORDER BY id");
    let rows = sqlx::query_as::<_, ExtractionItemRecord>(&sql)
        .bind(run_id)
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
