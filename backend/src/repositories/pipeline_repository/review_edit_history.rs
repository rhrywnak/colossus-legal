//! Review-panel edit-history audit trail.
//!
//! Field-by-field record of every reviewer-driven change to an
//! `extraction_items` row. Insert is called from the API handler on
//! each successful edit; read is used by the Review tab's "history"
//! popover. The trail is append-only — there is no update or delete
//! path through this module.

use serde::Serialize;
use sqlx::PgPool;

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
