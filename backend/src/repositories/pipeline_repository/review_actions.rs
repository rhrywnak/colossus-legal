//! Mutating review actions: approve / reject / edit / bulk approve / undo.
//!
//! Every function here returns a [`super::review_items::ReviewActionResult`]
//! (or `u64` row count for bulk) so the calling Axum handler can build
//! the response without a second SELECT. All state transitions on
//! `review_status` flow through this file — read-only queries live in
//! [`super::review_items`].

use sqlx::PgPool;

use super::review_grounding::grounded_statuses_vec;
use super::review_items::ReviewActionResult;

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
///
/// `filter == "all"` ignores the grounding-status whitelist and approves
/// every pending row; any other value restricts approval to items whose
/// `grounding_status` is in
/// [`super::review_grounding::GROUNDED_STATUSES`].
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
           AND ($3 = 'all' OR grounding_status = ANY($4))",
    )
    .bind(reviewed_by)
    .bind(document_id)
    .bind(filter)
    .bind(grounded_statuses_vec())
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
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
