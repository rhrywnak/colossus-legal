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

/// Count remaining pending items for a document.
pub async fn count_pending(pool: &PgPool, document_id: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM extraction_items
         WHERE document_id = $1 AND LOWER(review_status) = 'pending'",
    )
    .bind(document_id)
    .fetch_one(pool)
    .await
}
