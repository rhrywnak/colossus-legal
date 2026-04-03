//! Repository for pipeline tables in the `colossus_legal_v2` database.
//!
//! All functions take a `&PgPool` parameter (the pipeline pool, NOT the main pool).
//! This keeps the repository stateless — the caller decides which pool to pass.
//!
//! ## Rust Learning: Module directory split
//!
//! This module is split into two files:
//! - `mod.rs` — shared types, error, document and config CRUD
//! - `extraction.rs` — extraction_runs, extraction_items, extraction_relationships

mod extraction;
pub mod review;
pub mod steps;
pub mod users;

pub use extraction::*;

use serde::{Deserialize, Serialize};
use sqlx::PgPool;

// ── Error type ───────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum PipelineRepoError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Document not found: {0}")]
    NotFound(String),
}

impl From<sqlx::Error> for PipelineRepoError {
    fn from(e: sqlx::Error) -> Self {
        PipelineRepoError::Database(e.to_string())
    }
}

// ── Types ────────────────────────────────────────────────────────

/// Input for creating pipeline configuration (from the upload request).
#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineConfigInput {
    pub pass1_model: Option<String>,
    pub pass2_model: Option<String>,
    pub pass1_max_tokens: Option<i32>,
    pub pass2_max_tokens: Option<i32>,
    pub schema_file: String,
    pub admin_instructions: Option<String>,
    pub prior_context_doc_ids: Option<Vec<String>>,
}

/// A document record from the `documents` table.
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct DocumentRecord {
    pub id: String,
    pub title: String,
    pub file_path: String,
    pub file_hash: String,
    pub document_type: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_reviewer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assigned_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Total cost in USD across all completed pipeline steps (computed via LEFT JOIN).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_cost_usd: Option<f64>,
    /// Whether this document has any failed pipeline steps (computed via LEFT JOIN).
    pub has_failed_steps: bool,
}

/// A page of extracted text from the `document_text` table.
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct DocumentTextRecord {
    pub document_id: String,
    pub page_number: i32,
    pub text_content: String,
}

/// A pipeline_config record from the database.
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct PipelineConfigRecord {
    pub document_id: String,
    pub pass1_model: String,
    pub pass2_model: Option<String>,
    pub pass1_max_tokens: i32,
    pub pass2_max_tokens: Option<i32>,
    pub schema_file: String,
    pub admin_instructions: Option<String>,
    pub prior_context_doc_ids: Option<Vec<String>>,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ── Document & config functions ──────────────────────────────────

/// Insert a new document record. Status = "UPLOADED".
pub async fn insert_document(
    pool: &PgPool,
    id: &str,
    title: &str,
    file_path: &str,
    file_hash: &str,
    document_type: &str,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        r#"INSERT INTO documents (id, title, file_path, file_hash, document_type, status)
           VALUES ($1, $2, $3, $4, $5, 'UPLOADED')"#,
    )
    .bind(id)
    .bind(title)
    .bind(file_path)
    .bind(file_hash)
    .bind(document_type)
    .execute(pool)
    .await?;
    Ok(())
}

/// Insert pipeline configuration for a document.
pub async fn insert_pipeline_config(
    pool: &PgPool,
    document_id: &str,
    config: &PipelineConfigInput,
    created_by: &str,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        r#"INSERT INTO pipeline_config
           (document_id, pass1_model, pass2_model, pass1_max_tokens, pass2_max_tokens,
            schema_file, admin_instructions, prior_context_doc_ids, created_by)
           VALUES ($1, COALESCE($2, 'claude-sonnet-4-6'), $3,
                   COALESCE($4, 32000), $5, $6, $7, $8, $9)"#,
    )
    .bind(document_id)
    .bind(&config.pass1_model)
    .bind(&config.pass2_model)
    .bind(config.pass1_max_tokens)
    .bind(config.pass2_max_tokens)
    .bind(&config.schema_file)
    .bind(&config.admin_instructions)
    .bind(&config.prior_context_doc_ids)
    .bind(created_by)
    .execute(pool)
    .await?;
    Ok(())
}

/// Update document status and set updated_at to now.
pub async fn update_document_status(
    pool: &PgPool,
    document_id: &str,
    status: &str,
) -> Result<(), PipelineRepoError> {
    let result = sqlx::query(
        "UPDATE documents SET status = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(status)
    .bind(document_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(PipelineRepoError::NotFound(document_id.to_string()));
    }
    Ok(())
}

/// Insert extracted text for a single page.
pub async fn insert_document_text(
    pool: &PgPool,
    document_id: &str,
    page_number: i32,
    text_content: &str,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        r#"INSERT INTO document_text (document_id, page_number, text_content)
           VALUES ($1, $2, $3)
           ON CONFLICT (document_id, page_number) DO UPDATE SET text_content = $3"#,
    )
    .bind(document_id)
    .bind(page_number)
    .bind(text_content)
    .execute(pool)
    .await?;
    Ok(())
}

/// List all documents, most recent first.
pub async fn list_all_documents(pool: &PgPool) -> Result<Vec<DocumentRecord>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, DocumentRecord>(
        "SELECT d.id, d.title, d.file_path, d.file_hash, d.document_type, d.status,
                d.created_at, d.updated_at, d.assigned_reviewer, d.assigned_at,
                cost.total_cost_usd,
                COALESCE(err.has_failed, false) AS has_failed_steps
         FROM documents d
         LEFT JOIN (
             SELECT document_id, SUM(cost_usd::float8) AS total_cost_usd
             FROM extraction_runs
             WHERE status = 'COMPLETED' AND cost_usd IS NOT NULL
             GROUP BY document_id
         ) cost ON cost.document_id = d.id
         LEFT JOIN (
             SELECT document_id, true AS has_failed
             FROM pipeline_steps
             WHERE status = 'failed'
             GROUP BY document_id
         ) err ON err.document_id = d.id
         ORDER BY d.created_at DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Get a document by ID. Returns None if not found.
pub async fn get_document(
    pool: &PgPool,
    document_id: &str,
) -> Result<Option<DocumentRecord>, PipelineRepoError> {
    let row = sqlx::query_as::<_, DocumentRecord>(
        "SELECT d.id, d.title, d.file_path, d.file_hash, d.document_type, d.status,
                d.created_at, d.updated_at, d.assigned_reviewer, d.assigned_at,
                cost.total_cost_usd,
                COALESCE(err.has_failed, false) AS has_failed_steps
         FROM documents d
         LEFT JOIN (
             SELECT document_id, SUM(cost_usd::float8) AS total_cost_usd
             FROM extraction_runs
             WHERE status = 'COMPLETED' AND cost_usd IS NOT NULL AND document_id = $1
             GROUP BY document_id
         ) cost ON cost.document_id = d.id
         LEFT JOIN (
             SELECT document_id, true AS has_failed
             FROM pipeline_steps
             WHERE status = 'failed' AND document_id = $1
             GROUP BY document_id
         ) err ON err.document_id = d.id
         WHERE d.id = $1",
    )
    .bind(document_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

/// Get all extracted text pages for a document, ordered by page number.
pub async fn get_document_text(
    pool: &PgPool,
    document_id: &str,
) -> Result<Vec<DocumentTextRecord>, PipelineRepoError> {
    let rows = sqlx::query_as::<_, DocumentTextRecord>(
        "SELECT document_id, page_number, text_content
         FROM document_text WHERE document_id = $1 ORDER BY page_number",
    )
    .bind(document_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Get pipeline config for a document. Returns None if not configured.
pub async fn get_pipeline_config(
    pool: &PgPool,
    document_id: &str,
) -> Result<Option<PipelineConfigRecord>, PipelineRepoError> {
    let row = sqlx::query_as::<_, PipelineConfigRecord>(
        "SELECT document_id, pass1_model, pass2_model, pass1_max_tokens, pass2_max_tokens,
                schema_file, admin_instructions, prior_context_doc_ids, created_by, created_at
         FROM pipeline_config WHERE document_id = $1",
    )
    .bind(document_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}
