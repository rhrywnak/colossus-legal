//! Review-specific queries for extraction items.
//!
//! Supports the review panel: list with filtering/pagination,
//! approve, reject, edit, and bulk approve operations.

use serde::Serialize;
use sqlx::PgPool;

/// A flattened extraction item row for the review panel.
/// Pulls `label` and `properties` out of the JSONB `item_data` column.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ReviewItemRow {
    pub id: i32,
    pub entity_type: String,
    pub label: Option<String>,
    pub verbatim_quote: Option<String>,
    pub grounding_status: Option<String>,
    pub grounded_page: Option<i32>,
    pub review_status: String,
    pub reviewed_by: Option<String>,
    pub reviewed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub review_notes: Option<String>,
    pub properties: Option<serde_json::Value>,
}

/// Result of an approve/reject/edit operation.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ReviewActionResult {
    pub id: i32,
    pub review_status: String,
    pub grounded_page: Option<i32>,
    pub grounding_status: Option<String>,
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
        "SELECT id, entity_type, item_data->>'label' AS label, verbatim_quote,
                grounding_status, grounded_page, review_status, reviewed_by,
                reviewed_at, review_notes, item_data->'properties' AS properties
         FROM extraction_items
         WHERE run_id = $1
           AND ($2::text IS NULL OR review_status = $2)
           AND ($3::text IS NULL OR entity_type = $3)
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
           AND ($3::text IS NULL OR entity_type = $3)
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

/// Approve an extraction item.
pub async fn approve_item(
    pool: &PgPool,
    item_id: i32,
    reviewed_by: &str,
    notes: Option<&str>,
) -> Result<Option<ReviewActionResult>, sqlx::Error> {
    sqlx::query_as::<_, ReviewActionResult>(
        "UPDATE extraction_items
         SET review_status = 'approved', reviewed_by = $1, reviewed_at = NOW(), review_notes = $2
         WHERE id = $3
         RETURNING id, review_status, grounded_page, grounding_status",
    )
    .bind(reviewed_by)
    .bind(notes)
    .bind(item_id)
    .fetch_optional(pool)
    .await
}

/// Reject an extraction item.
pub async fn reject_item(
    pool: &PgPool,
    item_id: i32,
    reviewed_by: &str,
    reason: &str,
) -> Result<Option<ReviewActionResult>, sqlx::Error> {
    sqlx::query_as::<_, ReviewActionResult>(
        "UPDATE extraction_items
         SET review_status = 'rejected', reviewed_by = $1, reviewed_at = NOW(), review_notes = $2
         WHERE id = $3
         RETURNING id, review_status, grounded_page, grounding_status",
    )
    .bind(reviewed_by)
    .bind(reason)
    .bind(item_id)
    .fetch_optional(pool)
    .await
}

/// Edit and approve an extraction item (partial update).
pub async fn edit_item(
    pool: &PgPool,
    item_id: i32,
    reviewed_by: &str,
    grounded_page: Option<i32>,
    verbatim_quote: Option<&str>,
    notes: Option<&str>,
) -> Result<Option<ReviewActionResult>, sqlx::Error> {
    sqlx::query_as::<_, ReviewActionResult>(
        "UPDATE extraction_items
         SET grounded_page = COALESCE($1, grounded_page),
             verbatim_quote = COALESCE($2, verbatim_quote),
             grounding_status = CASE WHEN $1 IS NOT NULL THEN 'manual' ELSE grounding_status END,
             review_status = 'edited',
             reviewed_by = $3, reviewed_at = NOW(), review_notes = $4
         WHERE id = $5
         RETURNING id, review_status, grounded_page, grounding_status",
    )
    .bind(grounded_page)
    .bind(verbatim_quote)
    .bind(reviewed_by)
    .bind(notes)
    .bind(item_id)
    .fetch_optional(pool)
    .await
}

/// Bulk approve items for a document. Returns count of approved items.
pub async fn bulk_approve(
    pool: &PgPool,
    document_id: &str,
    reviewed_by: &str,
    filter: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE extraction_items
         SET review_status = 'approved', reviewed_by = $1, reviewed_at = NOW(),
             review_notes = 'Bulk approved'
         WHERE document_id = $2
           AND LOWER(review_status) = 'pending'
           AND ($3 = 'all' OR grounding_status IN ('exact', 'normalized'))",
    )
    .bind(reviewed_by)
    .bind(document_id)
    .bind(filter)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

/// Count ungrounded pending items for a document.
///
/// These are items with review_status = 'pending' and grounding_status
/// NOT IN ('exact', 'normalized') — i.e., items skipped by approve-grounded.
pub async fn count_ungrounded_pending(pool: &PgPool, document_id: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM extraction_items
         WHERE document_id = $1
           AND LOWER(review_status) = 'pending'
           AND (grounding_status IS NULL OR grounding_status NOT IN ('exact', 'normalized'))",
    )
    .bind(document_id)
    .fetch_one(pool)
    .await
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

/// Get the entity_type and document_id for an item.
#[derive(Debug, sqlx::FromRow)]
pub struct ItemTypeInfo {
    pub entity_type: String,
    pub document_id: String,
}

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

/// Revert an approved/edited item back to pending.
pub async fn unapprove_item(
    pool: &PgPool,
    item_id: i32,
) -> Result<Option<ReviewActionResult>, sqlx::Error> {
    sqlx::query_as::<_, ReviewActionResult>(
        "UPDATE extraction_items
         SET review_status = 'pending', reviewed_by = NULL, reviewed_at = NULL, review_notes = NULL
         WHERE id = $1 AND review_status IN ('approved', 'edited')
         RETURNING id, review_status, grounded_page, grounding_status",
    )
    .bind(item_id)
    .fetch_optional(pool)
    .await
}

/// Revert a rejected item back to pending.
pub async fn unreject_item(
    pool: &PgPool,
    item_id: i32,
) -> Result<Option<ReviewActionResult>, sqlx::Error> {
    sqlx::query_as::<_, ReviewActionResult>(
        "UPDATE extraction_items
         SET review_status = 'pending', reviewed_by = NULL, reviewed_at = NULL, review_notes = NULL
         WHERE id = $1 AND review_status = 'rejected'
         RETURNING id, review_status, grounded_page, grounding_status",
    )
    .bind(item_id)
    .fetch_optional(pool)
    .await
}

/// Count relationships that reference an item (for cascade warnings).
pub async fn count_relationships_for_item(
    pool: &PgPool,
    item_id: i32,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM extraction_relationships
         WHERE from_item_id = $1 OR to_item_id = $1",
    )
    .bind(item_id)
    .fetch_one(pool)
    .await
}

/// Record a field change in the edit history audit trail.
pub async fn insert_edit_history(
    pool: &PgPool,
    item_id: i32,
    field_changed: &str,
    old_value: Option<&str>,
    new_value: Option<&str>,
    changed_by: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO review_edit_history (item_id, field_changed, old_value, new_value, changed_by)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(item_id)
    .bind(field_changed)
    .bind(old_value)
    .bind(new_value)
    .bind(changed_by)
    .execute(pool)
    .await?;
    Ok(())
}

/// Edit history record for the audit trail.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct EditHistoryRecord {
    pub id: i32,
    pub item_id: i32,
    pub field_changed: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub changed_by: String,
    pub changed_at: chrono::DateTime<chrono::Utc>,
}

/// Get edit history for an item.
pub async fn get_edit_history(
    pool: &PgPool,
    item_id: i32,
) -> Result<Vec<EditHistoryRecord>, sqlx::Error> {
    sqlx::query_as::<_, EditHistoryRecord>(
        "SELECT id, item_id, field_changed, old_value, new_value, changed_by, changed_at
         FROM review_edit_history WHERE item_id = $1 ORDER BY changed_at DESC",
    )
    .bind(item_id)
    .fetch_all(pool)
    .await
}

/// Count remaining pending items that are actionable in the pipeline.
///
/// Only counts grounded items (grounding_status IN ('exact', 'normalized')).
/// Ungrounded pending items are intentionally excluded from the pipeline
/// flow — they don't block the Ingest button from appearing.
pub async fn count_pending(pool: &PgPool, document_id: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM extraction_items
         WHERE document_id = $1
           AND LOWER(review_status) = 'pending'
           AND grounding_status IN ('exact', 'normalized')",
    )
    .bind(document_id)
    .fetch_one(pool)
    .await
}
