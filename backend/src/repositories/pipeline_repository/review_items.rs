//! Review-panel item row types and read queries.
//!
//! Owns the row shapes the review panel UI consumes ([`ReviewItemRow`],
//! [`ReviewActionResult`], [`ItemTypeInfo`]) and the read-only queries
//! that populate them: list with filters/pagination, count, single-item
//! fetch, type-info lookup, and relationship-count for cascade
//! warnings. Mutating operations (approve / reject / edit / bulk
//! approve / undo) live in [`super::review_actions`].

use serde::Serialize;
use sqlx::PgPool;

/// A flattened extraction item row for the review panel.
/// Pulls `label` and `properties` out of the JSONB `item_data` column.
///
/// `entity_type` stays pinned to the LLM's immutable label (always
/// `"Party"` for Party items, even after Ingest resolves them) because
/// the schema-driven category lookup in `items.rs` keys on that value.
/// `resolved_entity_type` carries the Neo4j label Ingest chose
/// (`"Person"` / `"Organization"`) so the UI can render the effective
/// type and the People & Links tab can filter on it.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ReviewItemRow {
    pub id: i32,
    pub entity_type: String,
    pub resolved_entity_type: Option<String>,
    pub label: Option<String>,
    pub verbatim_quote: Option<String>,
    pub grounding_status: Option<String>,
    pub grounded_page: Option<i32>,
    /// Diagnostic reason persisted alongside `grounding_status='derived_invalid'`.
    /// Surfaces in the Review tab UI so reviewers see WHY a derived item
    /// was rejected without grepping logs. Populated only by the v5.1
    /// `validate_derived_provenance` path; NULL for every other status.
    pub verification_reason: Option<String>,
    pub review_status: String,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub review_notes: Option<String>,
    pub properties: Option<serde_json::Value>,
}

/// Result of an approve/reject/edit operation.
///
/// Returned from every mutating review action (in [`super::review_actions`])
/// so the caller — typically an Axum handler — can build the response
/// body without a second SELECT.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ReviewActionResult {
    pub id: i32,
    pub review_status: String,
    pub grounded_page: Option<i32>,
    pub grounding_status: Option<String>,
}

/// Get the entity_type and document_id for an item.
#[derive(Debug, sqlx::FromRow)]
pub struct ItemTypeInfo {
    pub entity_type: String,
    pub document_id: String,
}

/// List extraction items for a run with optional filters and pagination.
pub async fn list_items(
    pool: &PgPool,
    run_id: i32,
    review_status: Option<&str>,
    entity_type: Option<&str>,
    grounding_status: Option<&str>,
    limit: i64,
    offset: i64,
) -> Result<Vec<ReviewItemRow>, sqlx::Error> {
    sqlx::query_as::<_, ReviewItemRow>(
        "SELECT id, entity_type, resolved_entity_type,
                item_data->>'label' AS label, verbatim_quote,
                grounding_status, grounded_page, verification_reason,
                review_status, reviewed_by,
                reviewed_at, review_notes, item_data->'properties' AS properties
         FROM extraction_items
         WHERE run_id = $1
           AND ($2::text IS NULL OR review_status = $2)
           AND ($3::text IS NULL OR COALESCE(resolved_entity_type, entity_type) = $3)
           AND ($4::text IS NULL OR grounding_status = $4)
         ORDER BY id
         LIMIT $5 OFFSET $6",
    )
    .bind(run_id)
    .bind(review_status)
    .bind(entity_type)
    .bind(grounding_status)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

/// Count extraction items for a run with optional filters.
pub async fn count_items(
    pool: &PgPool,
    run_id: i32,
    review_status: Option<&str>,
    entity_type: Option<&str>,
    grounding_status: Option<&str>,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM extraction_items
         WHERE run_id = $1
           AND ($2::text IS NULL OR review_status = $2)
           AND ($3::text IS NULL OR COALESCE(resolved_entity_type, entity_type) = $3)
           AND ($4::text IS NULL OR grounding_status = $4)",
    )
    .bind(run_id)
    .bind(review_status)
    .bind(entity_type)
    .bind(grounding_status)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

/// Fetch a single extraction item by ID (for guards and history).
pub async fn get_item_by_id(
    pool: &PgPool,
    item_id: i32,
) -> Result<Option<ReviewActionResult>, sqlx::Error> {
    sqlx::query_as::<_, ReviewActionResult>(
        "SELECT id, review_status, grounded_page, grounding_status
         FROM extraction_items WHERE id = $1",
    )
    .bind(item_id)
    .fetch_optional(pool)
    .await
}

/// Read the immutable `entity_type` + `document_id` for an item.
///
/// Returns `None` if the item does not exist. Used by API handlers that
/// need to authorise an operation against the item's owning document
/// (e.g., "can this user edit items in this document") without loading
/// the full row.
pub async fn get_item_type_info(
    pool: &PgPool,
    item_id: i32,
) -> Result<Option<ItemTypeInfo>, sqlx::Error> {
    sqlx::query_as::<_, ItemTypeInfo>(
        "SELECT entity_type, document_id FROM extraction_items WHERE id = $1",
    )
    .bind(item_id)
    .fetch_optional(pool)
    .await
}

/// Count relationships that reference an item (for cascade warnings).
pub async fn count_relationships_for_item(pool: &PgPool, item_id: i32) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM extraction_relationships
         WHERE from_item_id = $1 OR to_item_id = $1",
    )
    .bind(item_id)
    .fetch_one(pool)
    .await
}
