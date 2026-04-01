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
}

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

/// Insert an extraction run record. Returns the auto-generated run ID.
/// Initial status is "RUNNING".
pub async fn insert_extraction_run(
    pool: &PgPool,
    document_id: &str,
    pass_number: i32,
    model_name: &str,
    schema_version: &str,
) -> Result<i32, PipelineRepoError> {
    let row = sqlx::query_scalar::<_, i32>(
        r#"INSERT INTO extraction_runs
           (document_id, pass_number, model_name, schema_version, started_at, raw_output, status)
           VALUES ($1, $2, $3, $4, NOW(), '{}'::jsonb, 'RUNNING')
           RETURNING id"#,
    )
    .bind(document_id)
    .bind(pass_number)
    .bind(model_name)
    .bind(schema_version)
    .fetch_one(pool)
    .await?;
    Ok(row)
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
    let rows = sqlx::query_as::<_, ExtractionItemRecord>(
        "SELECT * FROM extraction_items
         WHERE document_id = $1 AND verbatim_quote IS NOT NULL AND verbatim_quote != ''
         ORDER BY id",
    )
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

/// Get all extraction items for a document (for report generation).
pub async fn get_all_items(
    pool: &PgPool,
    document_id: &str,
) -> Result<Vec<ExtractionItemRecord>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, ExtractionItemRecord>(
        "SELECT * FROM extraction_items WHERE document_id = $1 ORDER BY entity_type, id",
    )
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

/// Get the latest COMPLETED extraction run ID for a document.
///
/// Returns `None` if no completed run exists. Used by the ingest handler
/// to find which run's items to write into Neo4j.
pub async fn get_latest_completed_run(
    pool: &PgPool,
    document_id: &str,
) -> Result<Option<i32>, PipelineRepoError> {
    let row = sqlx::query_scalar::<_, i32>(
        "SELECT id FROM extraction_runs
         WHERE document_id = $1 AND status = 'COMPLETED'
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
    let rows = sqlx::query_as::<_, ExtractionItemRecord>(
        "SELECT * FROM extraction_items WHERE run_id = $1 ORDER BY id",
    )
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
