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

pub mod documents;
pub mod extraction;
pub mod models;
pub mod review;
pub mod steps;
pub mod users;

pub use extraction::*;
pub use models::LlmModelRecord;

use crate::models::document_status::{
    RUN_STATUS_COMPLETED, STATUS_NEW, STEP_STATUS_FAILED,
};
use crate::pipeline::config::PipelineConfigOverrides;

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
    // ── Progress tracking (process endpoint) ────────────────────
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_step: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processing_step_label: Option<String>,
    pub chunks_total: Option<i32>,
    pub chunks_processed: Option<i32>,
    pub entities_found: Option<i32>,
    pub percent_complete: Option<i32>,
    // ── Error detail (process endpoint) ─────────────────────────
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_step: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_chunk: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_suggestion: Option<String>,
    // ── Cancellation ────────────────────────────────────────────
    pub is_cancelled: bool,
    // ── Auto-write tracking ─────────────────────────────────────
    pub entities_written: Option<i32>,
    pub entities_flagged: Option<i32>,
    pub relationships_written: Option<i32>,
    // ── Latest extraction run stats (computed via LEFT JOIN) ─────
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    pub run_chunk_count: Option<i32>,
    pub run_chunks_succeeded: Option<i32>,
    pub run_chunks_failed: Option<i32>,
    // ── PDF content classification (upload-time; see migration
    //     20260420143625_add_document_content_classification.sql) ────
    /// One of `"text_based"`, `"scanned"`, `"mixed"`, or `"unknown"`
    /// (default). Never null on rows inserted after the migration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    pub page_count: Option<i32>,
    pub text_pages: Option<i32>,
    pub scanned_pages: Option<i32>,
    /// One-based page indices that need OCR. Empty for `text_based`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages_needing_ocr: Option<Vec<i32>>,
    pub total_chars: Option<i32>,
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

/// Insert a new document record. Status = "NEW".
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
           VALUES ($1, $2, $3, $4, $5, $6)"#,
    )
    .bind(id)
    .bind(title)
    .bind(file_path)
    .bind(file_hash)
    .bind(document_type)
    .bind(STATUS_NEW)
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

/// Update the entity and relationship write counts on the documents table.
///
/// Called after Ingest commits nodes to Neo4j. These counts power the
/// Processing tab's "N entities written to graph" indicator. Previously
/// the counts were computed and logged but never persisted, so the UI
/// always displayed 0 (bug B2).
pub async fn update_document_write_counts(
    pool: &PgPool,
    document_id: &str,
    entities_written: i32,
    relationships_written: i32,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "UPDATE documents SET entities_written = $2, relationships_written = $3, \
         updated_at = NOW() WHERE id = $1",
    )
    .bind(document_id)
    .bind(entities_written)
    .bind(relationships_written)
    .execute(pool)
    .await?;
    Ok(())
}

/// Additive counterpart to `update_document_write_counts` for delta ingest.
///
/// Delta runs top up the graph with newly-approved items; the counts
/// shown in the UI should reflect cumulative totals across the original
/// Ingest plus every subsequent delta. Use `COALESCE` so a NULL starting
/// value (unlikely but possible) degrades to zero rather than leaving
/// the row unchanged.
pub async fn add_document_write_counts(
    pool: &PgPool,
    document_id: &str,
    delta_entities: i32,
    delta_relationships: i32,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "UPDATE documents \
         SET entities_written = COALESCE(entities_written, 0) + $2, \
             relationships_written = COALESCE(relationships_written, 0) + $3, \
             updated_at = NOW() \
         WHERE id = $1",
    )
    .bind(document_id)
    .bind(delta_entities)
    .bind(delta_relationships)
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
    let result = sqlx::query("UPDATE documents SET status = $1, updated_at = NOW() WHERE id = $2")
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
                COALESCE(err.has_failed, false) AS has_failed_steps,
                d.processing_step, d.processing_step_label,
                d.chunks_total, d.chunks_processed, d.entities_found, d.percent_complete,
                d.failed_step, d.failed_chunk, d.error_message, d.error_suggestion,
                d.is_cancelled,
                d.entities_written, d.entities_flagged, d.relationships_written,
                run.model_name,
                run.chunk_count AS run_chunk_count,
                run.chunks_succeeded AS run_chunks_succeeded,
                run.chunks_failed AS run_chunks_failed,
                d.content_type, d.page_count, d.text_pages, d.scanned_pages,
                d.pages_needing_ocr, d.total_chars
         FROM documents d
         LEFT JOIN (
             SELECT document_id, SUM(cost_usd::float8) AS total_cost_usd
             FROM extraction_runs
             WHERE status = $1 AND cost_usd IS NOT NULL
             GROUP BY document_id
         ) cost ON cost.document_id = d.id
         LEFT JOIN (
             SELECT document_id, true AS has_failed
             FROM pipeline_steps
             WHERE status = $2
             GROUP BY document_id
         ) err ON err.document_id = d.id
         LEFT JOIN LATERAL (
             SELECT model_name, chunk_count, chunks_succeeded, chunks_failed
             FROM extraction_runs
             WHERE document_id = d.id AND status = $1
             ORDER BY id DESC LIMIT 1
         ) run ON true
         ORDER BY d.created_at DESC",
    )
    .bind(RUN_STATUS_COMPLETED)
    .bind(STEP_STATUS_FAILED)
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
                COALESCE(err.has_failed, false) AS has_failed_steps,
                d.processing_step, d.processing_step_label,
                d.chunks_total, d.chunks_processed, d.entities_found, d.percent_complete,
                d.failed_step, d.failed_chunk, d.error_message, d.error_suggestion,
                d.is_cancelled,
                d.entities_written, d.entities_flagged, d.relationships_written,
                run.model_name,
                run.chunk_count AS run_chunk_count,
                run.chunks_succeeded AS run_chunks_succeeded,
                run.chunks_failed AS run_chunks_failed,
                d.content_type, d.page_count, d.text_pages, d.scanned_pages,
                d.pages_needing_ocr, d.total_chars
         FROM documents d
         LEFT JOIN (
             SELECT document_id, SUM(cost_usd::float8) AS total_cost_usd
             FROM extraction_runs
             WHERE status = $2 AND cost_usd IS NOT NULL AND document_id = $1
             GROUP BY document_id
         ) cost ON cost.document_id = d.id
         LEFT JOIN (
             SELECT document_id, true AS has_failed
             FROM pipeline_steps
             WHERE status = $3 AND document_id = $1
             GROUP BY document_id
         ) err ON err.document_id = d.id
         LEFT JOIN LATERAL (
             SELECT model_name, chunk_count, chunks_succeeded, chunks_failed
             FROM extraction_runs
             WHERE document_id = $1 AND status = $2
             ORDER BY id DESC LIMIT 1
         ) run ON true
         WHERE d.id = $1",
    )
    .bind(document_id)
    .bind(RUN_STATUS_COMPLETED)
    .bind(STEP_STATUS_FAILED)
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

/// Row-shaped helper used only by [`get_pipeline_config_overrides`].
///
/// The nullable override columns have an awkward 8-tuple shape; a named
/// struct keeps the type signature readable and avoids the
/// `clippy::type_complexity` lint on anonymous tuples.
#[derive(sqlx::FromRow)]
struct PipelineConfigOverridesRow {
    profile_name: Option<String>,
    extraction_model: Option<String>,
    pass2_extraction_model: Option<String>,
    template_file: Option<String>,
    system_prompt_file: Option<String>,
    chunking_mode: Option<String>,
    chunk_size: Option<i32>,
    chunk_overlap: Option<i32>,
    max_tokens: Option<i32>,
    temperature: Option<f64>,
    run_pass2: Option<bool>,
}

/// Read per-document override columns from `pipeline_config`.
///
/// Returns a [`PipelineConfigOverrides`] populated from the nullable columns
/// added by migration `20260420_config_system.sql`. Each field is `Option` —
/// `None` means "use the profile default."
///
/// If no `pipeline_config` row exists for the document, returns
/// `PipelineConfigOverrides::default()` (all `None`). Callers can then
/// still resolve against the profile without a separate existence check.
pub async fn get_pipeline_config_overrides(
    db: &PgPool,
    document_id: &str,
) -> Result<PipelineConfigOverrides, PipelineRepoError> {
    let row: Option<PipelineConfigOverridesRow> = sqlx::query_as(
        "SELECT profile_name, extraction_model, pass2_extraction_model, \
                template_file, system_prompt_file, \
                chunking_mode, chunk_size, chunk_overlap, max_tokens, \
                temperature::float8 AS temperature, run_pass2 \
         FROM pipeline_config WHERE document_id = $1",
    )
    .bind(document_id)
    .fetch_optional(db)
    .await?;

    let result = match row {
        Some(r) => PipelineConfigOverrides {
            profile_name: r.profile_name,
            extraction_model: r.extraction_model,
            pass2_extraction_model: r.pass2_extraction_model,
            template_file: r.template_file,
            system_prompt_file: r.system_prompt_file,
            chunking_mode: r.chunking_mode,
            chunk_size: r.chunk_size,
            chunk_overlap: r.chunk_overlap,
            max_tokens: r.max_tokens,
            temperature: r.temperature,
            run_pass2: r.run_pass2,
            // pipeline_config columns for chunking_config/context_config
            // arrive in Group 3's migration. Until then, no per-document
            // override is read here — `resolve_config` falls through to the
            // profile's map.
            chunking_config: None,
            context_config: None,
        },
        None => PipelineConfigOverrides::default(),
    };

    tracing::info!(
        target: "structured_debug",
        document_id,
        overrides_chunking_mode = ?result.chunking_mode,
        overrides_profile_name = ?result.profile_name,
        overrides_chunking_config = ?result.chunking_config,
        "STRUCTURED-DEBUG: Q1 — overrides read from pipeline_config"
    );

    Ok(result)
}

/// Partially update the per-document override columns on `pipeline_config`.
///
/// Uses `UPDATE ... SET col = COALESCE($n, col)` so each `None` field in
/// `overrides` leaves the corresponding column untouched. If every field
/// in `overrides` is `None`, the UPDATE is skipped entirely to avoid a
/// pointless roundtrip.
///
/// Returns `PipelineRepoError::NotFound` if no `pipeline_config` row
/// matches `document_id` — the caller should have already inserted one at
/// upload time.
///
/// `temperature` is cast to `NUMERIC` inside the SQL so sqlx doesn't need
/// the `rust_decimal` feature for a direct `NUMERIC(3,2)` bind.
pub async fn patch_pipeline_config_overrides(
    db: &PgPool,
    document_id: &str,
    overrides: &PipelineConfigOverrides,
) -> Result<(), PipelineRepoError> {
    // Short-circuit when there is nothing to update.
    let any_field = overrides.profile_name.is_some()
        || overrides.extraction_model.is_some()
        || overrides.pass2_extraction_model.is_some()
        || overrides.template_file.is_some()
        || overrides.system_prompt_file.is_some()
        || overrides.chunking_mode.is_some()
        || overrides.chunk_size.is_some()
        || overrides.chunk_overlap.is_some()
        || overrides.max_tokens.is_some()
        || overrides.temperature.is_some()
        || overrides.run_pass2.is_some();
    if !any_field {
        let existing: Option<String> =
            sqlx::query_scalar("SELECT document_id FROM pipeline_config WHERE document_id = $1")
                .bind(document_id)
                .fetch_optional(db)
                .await?;
        return if existing.is_some() {
            Ok(())
        } else {
            Err(PipelineRepoError::NotFound(document_id.to_string()))
        };
    }

    let result = sqlx::query(
        "UPDATE pipeline_config SET \
           profile_name = COALESCE($2, profile_name), \
           extraction_model = COALESCE($3, extraction_model), \
           pass2_extraction_model = COALESCE($4, pass2_extraction_model), \
           template_file = COALESCE($5, template_file), \
           system_prompt_file = COALESCE($6, system_prompt_file), \
           chunking_mode = COALESCE($7, chunking_mode), \
           chunk_size = COALESCE($8, chunk_size), \
           chunk_overlap = COALESCE($9, chunk_overlap), \
           max_tokens = COALESCE($10, max_tokens), \
           temperature = COALESCE($11::numeric, temperature), \
           run_pass2 = COALESCE($12, run_pass2) \
         WHERE document_id = $1",
    )
    .bind(document_id)
    .bind(&overrides.profile_name)
    .bind(&overrides.extraction_model)
    .bind(&overrides.pass2_extraction_model)
    .bind(&overrides.template_file)
    .bind(&overrides.system_prompt_file)
    .bind(&overrides.chunking_mode)
    .bind(overrides.chunk_size)
    .bind(overrides.chunk_overlap)
    .bind(overrides.max_tokens)
    .bind(overrides.temperature)
    .bind(overrides.run_pass2)
    .execute(db)
    .await?;

    if result.rows_affected() == 0 {
        return Err(PipelineRepoError::NotFound(document_id.to_string()));
    }
    Ok(())
}

/// Update pipeline config with extraction overrides so the next run uses the same settings.
pub async fn update_pipeline_config(
    pool: &PgPool,
    document_id: &str,
    pass1_model: &str,
    pass1_max_tokens: i32,
    schema_file: &str,
    admin_instructions: Option<&str>,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        "UPDATE pipeline_config SET pass1_model = $2, pass1_max_tokens = $3, schema_file = $4, admin_instructions = $5 WHERE document_id = $1",
    )
    .bind(document_id)
    .bind(pass1_model)
    .bind(pass1_max_tokens)
    .bind(schema_file)
    .bind(admin_instructions)
    .execute(pool)
    .await?;
    Ok(())
}
