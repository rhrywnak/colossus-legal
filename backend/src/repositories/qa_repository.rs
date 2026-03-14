//! Repository for QA entries in PostgreSQL (CC-WP1-1).
//! QAEntry is a generic Q&A persistence format. App-specific data lives
//! in the `metadata` JSONB field.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Full QAEntry — returned for single-entry lookups.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QAEntry {
    pub id: String,
    pub scope_type: String,
    pub scope_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub question: String,
    pub answer: String,
    pub asked_by: String,
    pub asked_at: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_qa_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_rating: Option<i16>,
}

/// Input for creating a new QAEntry (id and asked_at are generated server-side).
#[derive(Debug, Deserialize)]
pub struct CreateQAEntry {
    pub scope_type: String,
    pub scope_id: String,
    pub session_id: Option<String>,
    pub question: String,
    pub answer: String,
    pub asked_by: String,
    pub model: String,
    pub parent_qa_id: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Summary for history list — no full answer, keeps the response small.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QAEntrySummary {
    pub id: String,
    pub scope_type: String,
    pub scope_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub question_preview: String,
    pub asked_by: String,
    pub asked_at: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_qa_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_ms: Option<i64>,
    /// Populated by the handler after fetching from PostgreSQL.
    pub user_rating: Option<i16>,
}

#[derive(Debug, thiserror::Error)]
pub enum QAError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("QA entry not found: {0}")]
    NotFound(String),
    #[error("Invalid rating: {0}")]
    InvalidRating(String),
}

impl From<sqlx::Error> for QAError {
    fn from(e: sqlx::Error) -> Self {
        QAError::Database(e.to_string())
    }
}

// ---------------------------------------------------------------------------
// Row type — maps PostgreSQL columns via sqlx::FromRow, then converts
// to API types (Uuid→String, DateTime→RFC3339 string).
// ---------------------------------------------------------------------------

#[derive(Debug, sqlx::FromRow)]
struct QaEntryRow {
    id: Uuid,
    scope_type: String,
    scope_id: String,
    session_id: Option<String>,
    question: String,
    answer: String,
    asked_by: String,
    asked_at: DateTime<Utc>,
    model: String,
    parent_qa_id: Option<Uuid>,
    metadata: Option<serde_json::Value>,
    rating: Option<i16>,
    rated_by: Option<String>,
    #[allow(dead_code)]
    rated_at: Option<DateTime<Utc>>,
    #[allow(dead_code)]
    created_at: DateTime<Utc>,
}

impl QaEntryRow {
    /// Convert a database row into the API-facing QAEntry struct.
    fn into_qa_entry(self) -> QAEntry {
        QAEntry {
            id: self.id.to_string(),
            scope_type: self.scope_type,
            scope_id: self.scope_id,
            session_id: self.session_id,
            question: self.question,
            answer: self.answer,
            asked_by: self.asked_by,
            asked_at: self.asked_at.to_rfc3339(),
            model: self.model,
            rating: self.rating,
            rating_by: self.rated_by,
            parent_qa_id: self.parent_qa_id.map(|u| u.to_string()),
            metadata: self.metadata,
            user_rating: None,
        }
    }

    /// Convert a database row into a QAEntrySummary (no full answer).
    /// Question is truncated to 200 chars (Unicode-safe via `.chars().take()`).
    fn into_summary(self) -> QAEntrySummary {
        let question_preview: String = self.question.chars().take(200).collect();
        let total_ms = self
            .metadata
            .as_ref()
            .and_then(|m| m.get("total_ms")?.as_i64());

        QAEntrySummary {
            id: self.id.to_string(),
            scope_type: self.scope_type,
            scope_id: self.scope_id,
            session_id: self.session_id,
            question_preview,
            asked_by: self.asked_by,
            asked_at: self.asked_at.to_rfc3339(),
            model: self.model,
            rating: self.rating,
            parent_qa_id: self.parent_qa_id.map(|u| u.to_string()),
            total_ms,
            user_rating: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Repository functions
// ---------------------------------------------------------------------------

/// Create a new QAEntry in PostgreSQL.
/// Generates a UUID server-side and returns the created entry.
pub async fn create_qa_entry(
    pool: &PgPool,
    entry: CreateQAEntry,
) -> Result<QAEntry, QAError> {
    let id = Uuid::new_v4();
    let parent_qa_id = entry
        .parent_qa_id
        .as_deref()
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|e| QAError::Database(format!("invalid parent_qa_id: {e}")))?;

    let row = sqlx::query_as::<_, QaEntryRow>(
        "INSERT INTO qa_entries (
            id, scope_type, scope_id, session_id, question, answer,
            asked_by, model, parent_qa_id, metadata
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING *",
    )
    .bind(id)
    .bind(&entry.scope_type)
    .bind(&entry.scope_id)
    .bind(&entry.session_id)
    .bind(&entry.question)
    .bind(&entry.answer)
    .bind(&entry.asked_by)
    .bind(&entry.model)
    .bind(parent_qa_id)
    .bind(&entry.metadata)
    .fetch_one(pool)
    .await?;

    Ok(row.into_qa_entry())
}

/// Get QAEntry history for a scope, newest first (summaries only).
pub async fn get_qa_history(
    pool: &PgPool,
    scope_type: &str,
    scope_id: &str,
    limit: i64,
) -> Result<Vec<QAEntrySummary>, QAError> {
    let rows = sqlx::query_as::<_, QaEntryRow>(
        "SELECT * FROM qa_entries
         WHERE scope_type = $1 AND scope_id = $2
         ORDER BY asked_at DESC
         LIMIT $3",
    )
    .bind(scope_type)
    .bind(scope_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(QaEntryRow::into_summary).collect())
}

/// Get a single QAEntry by ID, with full answer and metadata.
pub async fn get_qa_entry(
    pool: &PgPool,
    id: &str,
) -> Result<Option<QAEntry>, QAError> {
    let uuid = Uuid::parse_str(id)
        .map_err(|e| QAError::NotFound(format!("invalid UUID: {e}")))?;

    let row = sqlx::query_as::<_, QaEntryRow>(
        "SELECT * FROM qa_entries WHERE id = $1",
    )
    .bind(uuid)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(QaEntryRow::into_qa_entry))
}

/// Update the rating on a QA entry.
/// Replaces the old `RatingRepository::upsert_rating` — simpler because
/// the rating now lives in the same row as the QA entry.
pub async fn update_rating(
    pool: &PgPool,
    id: &str,
    rating: i16,
    rated_by: &str,
) -> Result<(), QAError> {
    let uuid = Uuid::parse_str(id)
        .map_err(|e| QAError::NotFound(format!("invalid UUID: {e}")))?;

    let result = sqlx::query(
        "UPDATE qa_entries SET rating = $2, rated_by = $3, rated_at = NOW()
         WHERE id = $1",
    )
    .bind(uuid)
    .bind(rating)
    .bind(rated_by)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(QAError::NotFound(id.to_string()));
    }

    Ok(())
}

/// Delete a QA entry. Only the user who asked can delete (ownership check).
pub async fn delete_qa_entry(
    pool: &PgPool,
    id: &str,
    username: &str,
) -> Result<(), QAError> {
    let uuid = Uuid::parse_str(id)
        .map_err(|e| QAError::NotFound(format!("invalid UUID: {e}")))?;

    let result = sqlx::query(
        "DELETE FROM qa_entries WHERE id = $1 AND asked_by = $2",
    )
    .bind(uuid)
    .bind(username)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(QAError::NotFound(id.to_string()));
    }

    Ok(())
}
