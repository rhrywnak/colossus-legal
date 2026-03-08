//! Repository for QAEntry nodes in Neo4j.
//! QAEntry is a generic Q&A persistence format. App-specific data lives
//! in the `metadata` JSON string field.

use chrono::Utc;
use neo4rs::{query, Graph};
use serde::{Deserialize, Serialize};
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
    pub rating: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_qa_id: Option<String>,
    /// App-specific metadata as a JSON value.
    /// For colossus-legal: retrieval stats + cited_node_ids.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
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
    pub rating: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_qa_id: Option<String>,
    /// total_ms extracted from metadata for display.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_ms: Option<i64>,
    /// The requesting user's rating (1–5) for this entry, or None if unrated.
    /// Populated by the handler after fetching from PostgreSQL.
    pub user_rating: Option<i16>,
}

#[derive(Debug, thiserror::Error)]
pub enum QAError {
    #[error("Neo4j error: {0}")]
    Neo4j(String),
    #[error("QA entry not found: {0}")]
    NotFound(String),
    #[error("Invalid rating: {0}")]
    InvalidRating(String),
}

impl From<neo4rs::Error> for QAError {
    fn from(e: neo4rs::Error) -> Self {
        QAError::Neo4j(e.to_string())
    }
}

/// Create a new QAEntry node and link it to the Case node via ASKED_IN.
/// All values are parameterized ($param) to prevent Cypher injection.
pub async fn create_qa_entry(
    graph: &Graph,
    entry: CreateQAEntry,
) -> Result<QAEntry, QAError> {
    let id = Uuid::new_v4().to_string();
    let asked_at = Utc::now().to_rfc3339();

    // Serialize metadata to a JSON string for Neo4j storage.
    // Neo4j doesn't have a native JSON type, so we store it as a string
    // property and deserialize on read.
    let metadata_str = entry
        .metadata
        .as_ref()
        .and_then(|m| serde_json::to_string(m).ok());

    let mut result = graph
        .execute(
            query(
                "CREATE (q:QAEntry {
                    id: $id,
                    scope_type: $scope_type,
                    scope_id: $scope_id,
                    session_id: $session_id,
                    question: $question,
                    answer: $answer,
                    asked_by: $asked_by,
                    asked_at: $asked_at,
                    model: $model,
                    rating: $rating,
                    rating_by: $rating_by,
                    parent_qa_id: $parent_qa_id,
                    metadata: $metadata
                })
                WITH q
                OPTIONAL MATCH (c:Case {id: $scope_id})
                FOREACH (_ IN CASE WHEN c IS NOT NULL THEN [1] ELSE [] END |
                    CREATE (q)-[:ASKED_IN]->(c)
                )
                RETURN q.id AS id",
            )
            .param("id", id.clone())
            .param("scope_type", entry.scope_type.clone())
            .param("scope_id", entry.scope_id.clone())
            .param("session_id", option_to_neo4j(&entry.session_id))
            .param("question", entry.question.clone())
            .param("answer", entry.answer.clone())
            .param("asked_by", entry.asked_by.clone())
            .param("asked_at", asked_at.clone())
            .param("model", entry.model.clone())
            .param("rating", option_to_neo4j(&None::<String>))
            .param("rating_by", option_to_neo4j(&None::<String>))
            .param("parent_qa_id", option_to_neo4j(&entry.parent_qa_id))
            .param("metadata", option_to_neo4j(&metadata_str)),
        )
        .await?;

    if result.next().await?.is_none() {
        return Err(QAError::Neo4j("CREATE returned no rows".to_string()));
    }

    Ok(QAEntry {
        id,
        scope_type: entry.scope_type,
        scope_id: entry.scope_id,
        session_id: entry.session_id,
        question: entry.question,
        answer: entry.answer,
        asked_by: entry.asked_by,
        asked_at,
        model: entry.model,
        rating: None,
        rating_by: None,
        parent_qa_id: entry.parent_qa_id,
        metadata: entry.metadata,
    })
}

/// Get QAEntry history for a scope, newest first (summaries only).
/// Question preview is truncated to 200 chars (Unicode-safe via `.chars().take()`).
pub async fn get_qa_history(
    graph: &Graph,
    scope_type: &str,
    scope_id: &str,
    limit: i64,
) -> Result<Vec<QAEntrySummary>, QAError> {
    let mut result = graph
        .execute(
            query(
                "MATCH (q:QAEntry {scope_type: $scope_type, scope_id: $scope_id})
                 RETURN q.id AS id,
                        q.scope_type AS scope_type,
                        q.scope_id AS scope_id,
                        q.session_id AS session_id,
                        q.question AS question,
                        q.asked_by AS asked_by,
                        q.asked_at AS asked_at,
                        q.model AS model,
                        q.rating AS rating,
                        q.parent_qa_id AS parent_qa_id,
                        q.metadata AS metadata
                 ORDER BY q.asked_at DESC
                 LIMIT $limit",
            )
            .param("scope_type", scope_type)
            .param("scope_id", scope_id)
            .param("limit", limit),
        )
        .await?;

    let mut entries = Vec::new();
    while let Some(row) = result.next().await? {
        let question: String = row.get("question").unwrap_or_default();
        let question_preview: String = question.chars().take(200).collect();

        // Extract total_ms from the metadata JSON string
        let metadata_str: Option<String> = row.get("metadata").ok();
        let total_ms = metadata_str
            .as_deref()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .and_then(|v| v.get("total_ms")?.as_i64());

        entries.push(QAEntrySummary {
            id: row.get("id").unwrap_or_default(),
            scope_type: row.get("scope_type").unwrap_or_default(),
            scope_id: row.get("scope_id").unwrap_or_default(),
            session_id: row.get("session_id").ok(),
            question_preview,
            asked_by: row.get("asked_by").unwrap_or_default(),
            asked_at: row.get("asked_at").unwrap_or_default(),
            model: row.get("model").unwrap_or_default(),
            rating: row.get("rating").ok(),
            parent_qa_id: row.get("parent_qa_id").ok(),
            total_ms,
            user_rating: None,
        });
    }

    Ok(entries)
}

/// Get a single QAEntry by ID, with full answer and metadata.
pub async fn get_qa_entry(
    graph: &Graph,
    id: &str,
) -> Result<Option<QAEntry>, QAError> {
    let mut result = graph
        .execute(
            query(
                "MATCH (q:QAEntry {id: $id})
                 RETURN q.id AS id,
                        q.scope_type AS scope_type,
                        q.scope_id AS scope_id,
                        q.session_id AS session_id,
                        q.question AS question,
                        q.answer AS answer,
                        q.asked_by AS asked_by,
                        q.asked_at AS asked_at,
                        q.model AS model,
                        q.rating AS rating,
                        q.rating_by AS rating_by,
                        q.parent_qa_id AS parent_qa_id,
                        q.metadata AS metadata",
            )
            .param("id", id),
        )
        .await?;

    let Some(row) = result.next().await? else {
        return Ok(None);
    };

    // Deserialize metadata from JSON string back to serde_json::Value
    let metadata_str: Option<String> = row.get("metadata").ok();
    let metadata = metadata_str
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    Ok(Some(QAEntry {
        id: row.get("id").unwrap_or_default(),
        scope_type: row.get("scope_type").unwrap_or_default(),
        scope_id: row.get("scope_id").unwrap_or_default(),
        session_id: row.get("session_id").ok(),
        question: row.get("question").unwrap_or_default(),
        answer: row.get("answer").unwrap_or_default(),
        asked_by: row.get("asked_by").unwrap_or_default(),
        asked_at: row.get("asked_at").unwrap_or_default(),
        model: row.get("model").unwrap_or_default(),
        rating: row.get("rating").ok(),
        rating_by: row.get("rating_by").ok(),
        parent_qa_id: row.get("parent_qa_id").ok(),
        metadata,
    }))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert Option<String> to a neo4rs parameter value.
/// None becomes "" (empty string). On read, `.ok()` returns None for
/// missing properties, so empty strings and null are equivalent.
fn option_to_neo4j(opt: &Option<String>) -> String {
    opt.clone().unwrap_or_default()
}
