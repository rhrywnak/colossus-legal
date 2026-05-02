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
use std::collections::HashMap;

// ── Error type ───────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum PipelineRepoError {
    #[error("Database error: {0}")]
    Database(String),
    #[error("Document not found: {0}")]
    NotFound(String),
    /// JSONB column on a `pipeline_config` row decoded from the database
    /// but failed to deserialize into the expected typed shape.
    ///
    /// Reserved for cases where the SQL succeeded (the row exists, the
    /// column is well-formed JSON) but the JSON's *shape* doesn't match
    /// what the application expects — e.g., `chunking_config` is a
    /// JSONB number instead of an object map. The error message names
    /// the offending document_id and column so an auditor can find the
    /// bad row directly.
    ///
    /// Distinct from `Database` so callers can decide whether to retry
    /// (Database errors may be transient; Deserialization errors are
    /// data-shape bugs and a retry won't help) and so audit/alerting
    /// can prioritise this class differently.
    #[error("Deserialization error: {0}")]
    Deserialization(String),
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
    // ── Document format (multi-format ingestion) ────────────────
    //
    // ## Rust Learning: Option<String> for nullable DB columns
    //
    // PostgreSQL columns declared without `NOT NULL` can contain NULL.
    // sqlx's `FromRow` derive maps nullable columns to `Option<T>`.
    // Existing documents uploaded before this migration have NULL for
    // both fields — the `Option` wrapper lets us handle that gracefully
    // instead of panicking on deserialization.
    //
    /// Detected MIME type from file content, e.g. "application/pdf".
    /// NULL for documents uploaded before multi-format support.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Short format key for ExtractText routing: "pdf", "docx", or "txt".
    /// NULL for documents uploaded before multi-format support.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_format: Option<String>,
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
///
/// 8 args is one over clippy's default; grouping the format fields into a
/// dedicated struct would obscure the simple flat insert this function
/// performs and add a layer of indirection at every call site for no
/// readability gain. The lint is silenced locally rather than project-wide
/// so other functions still get the warning.
#[allow(clippy::too_many_arguments)]
pub async fn insert_document(
    pool: &PgPool,
    id: &str,
    title: &str,
    file_path: &str,
    file_hash: &str,
    document_type: &str,
    // ## Rust Learning: Option<&str> vs Option<String>
    //
    // We take `Option<&str>` (a borrowed reference) rather than
    // `Option<String>` (an owned value) because the caller already
    // has the string — no need to clone it just to pass it here.
    // sqlx's `.bind()` accepts `Option<&str>` via its `Encode` trait
    // implementation, so this works directly with the query builder.
    mime_type: Option<&str>,
    original_format: Option<&str>,
) -> Result<(), PipelineRepoError> {
    sqlx::query(
        r#"INSERT INTO documents (id, title, file_path, file_hash, document_type, status, mime_type, original_format)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
    )
    .bind(id)
    .bind(title)
    .bind(file_path)
    .bind(file_hash)
    .bind(document_type)
    .bind(STATUS_NEW)
    .bind(mime_type)
    .bind(original_format)
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
                d.pages_needing_ocr, d.total_chars,
                d.mime_type, d.original_format
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
                d.pages_needing_ocr, d.total_chars,
                d.mime_type, d.original_format
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
    /// Raw JSONB from the new override columns. We deliberately stay
    /// at `serde_json::Value` here (rather than `Json<HashMap<...>>`)
    /// so the converter can attach the document_id to a typed
    /// `Deserialization` error if the JSON's shape doesn't match the
    /// expected map. `Json<T>`'s decode error wouldn't carry that
    /// context and would silently surface as a generic sqlx error.
    chunking_config: Option<serde_json::Value>,
    context_config: Option<serde_json::Value>,
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
                temperature::float8 AS temperature, run_pass2, \
                chunking_config, context_config \
         FROM pipeline_config WHERE document_id = $1",
    )
    .bind(document_id)
    .fetch_optional(db)
    .await?;

    let result = match row {
        Some(r) => {
            // Decode the two JSONB override maps with no-silent-fails:
            // a malformed body raises `Deserialization` carrying the
            // document_id and column so an auditor can locate the bad
            // row directly. `None` (NULL column) means "no override;
            // resolve_config will fall back to the profile's map."
            let chunking_config = decode_jsonb_map(
                document_id,
                "chunking_config",
                r.chunking_config,
            )?;
            let context_config = decode_jsonb_map(
                document_id,
                "context_config",
                r.context_config,
            )?;
            PipelineConfigOverrides {
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
                chunking_config,
                context_config,
            }
        }
        None => PipelineConfigOverrides::default(),
    };

    Ok(result)
}

/// Decode an `Option<serde_json::Value>` from a `pipeline_config` JSONB
/// column into the typed override shape (`Option<HashMap<String, Value>>`).
///
/// The two override columns (`chunking_config`, `context_config`) share
/// this exact shape, so the conversion is factored out. NULL → `Ok(None)`
/// (no override). A non-NULL value that doesn't deserialize into a map →
/// `Err(Deserialization)` with both `document_id` and `column` named in
/// the message — never silent `None`. The application layer treats `None`
/// as "inherit from profile"; a silent fall-through on bad data would
/// mask a corrupted row as a working one.
///
/// ## Rust Learning: factor on shape, not on column name
///
/// We pass the column name as a `&str` argument rather than writing two
/// near-identical decoders or templating the function over a const. The
/// caller already knows the column it's reading; threading it through
/// gives the error message all the context it needs without a generic
/// const-name parameter.
/// True when the `PipelineConfigOverrides` payload carries at least one
/// non-`None` field — i.e. the PATCH actually requests a change.
///
/// Factored out of `patch_pipeline_config_overrides` so the contract
/// can be unit-tested. The risk this guards against: a future field
/// added to `PipelineConfigOverrides` whose `is_some()` clause is
/// forgotten here would produce a silent no-op for any PATCH that
/// touches only that new field. The
/// `patch_with_only_chunking_config_does_not_short_circuit` test below
/// pins that down for the chunking_config path; analogous tests should
/// be added when a future field is introduced.
fn has_any_override(overrides: &PipelineConfigOverrides) -> bool {
    overrides.profile_name.is_some()
        || overrides.extraction_model.is_some()
        || overrides.pass2_extraction_model.is_some()
        || overrides.template_file.is_some()
        || overrides.system_prompt_file.is_some()
        || overrides.chunking_mode.is_some()
        || overrides.chunk_size.is_some()
        || overrides.chunk_overlap.is_some()
        || overrides.max_tokens.is_some()
        || overrides.temperature.is_some()
        || overrides.run_pass2.is_some()
        || overrides.chunking_config.is_some()
        || overrides.context_config.is_some()
}

fn decode_jsonb_map(
    document_id: &str,
    column: &str,
    raw: Option<serde_json::Value>,
) -> Result<Option<HashMap<String, serde_json::Value>>, PipelineRepoError> {
    match raw {
        None => Ok(None),
        Some(v) => serde_json::from_value(v).map(Some).map_err(|e| {
            PipelineRepoError::Deserialization(format!(
                "pipeline_config.{column} for document_id={document_id} is not a valid map: {e}"
            ))
        }),
    }
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
    // Short-circuit when there is nothing to update. Factored out so a
    // unit test can pin down the contract: every overridable field on
    // `PipelineConfigOverrides` MUST contribute to this check, or a
    // PATCH whose only field is the missing one would silently no-op.
    let any_field = has_any_override(overrides);
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

    // Convert the typed `Option<HashMap<...>>` overrides into the JSONB
    // shape sqlx wants for binding. Two states matter:
    //   - `None` here → bind NULL → COALESCE keeps the existing column
    //     value (the "no override on this PATCH" path).
    //   - `Some(map)` → bind the JSON body → COALESCE picks the new
    //     value (full whole-map replacement at the COLUMN level — the
    //     key-level merge on top of the profile happens later, at
    //     resolve_config time).
    //
    // `serde_json::to_value` over `HashMap<String, Value>` cannot fail
    // structurally (string keys, JSON-shaped values both round-trip
    // by construction). If a future change introduces a type that can
    // fail to serialize (e.g., `f64` with `NaN`), add a Serialization
    // variant to PipelineRepoError and propagate the error here. For
    // now, an `unwrap_or_else` would be a "this path is unreachable"
    // statement — we use `?` via `transpose()` so the type checker
    // proves the same thing without an unwrap.
    let chunking_config_json = overrides
        .chunking_config
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| PipelineRepoError::Database(format!(
            "structurally-impossible serialize of chunking_config for document_id={document_id}: {e}"
        )))?;
    let context_config_json = overrides
        .context_config
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| PipelineRepoError::Database(format!(
            "structurally-impossible serialize of context_config for document_id={document_id}: {e}"
        )))?;

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
           run_pass2 = COALESCE($12, run_pass2), \
           chunking_config = COALESCE($13, chunking_config), \
           context_config = COALESCE($14, context_config) \
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
    .bind(chunking_config_json)
    .bind(context_config_json)
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

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── decode_jsonb_map: round-trip + error semantics ──────────────
    //
    // The conversion from raw JSONB (`Option<serde_json::Value>`) to
    // the typed `Option<HashMap<String, Value>>` is the core of the
    // read-path's no-silent-fail contract. These tests pin it down
    // without needing a live database.

    #[test]
    fn decode_jsonb_map_returns_none_when_column_is_null() {
        // The "no override; inherit from profile" path. Must be `Ok(None)`,
        // never an error and never a silent default-empty-map.
        let result = decode_jsonb_map("doc-x", "chunking_config", None);
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn decode_jsonb_map_returns_typed_map_for_well_formed_json() {
        let raw = serde_json::json!({"units_per_chunk": 3, "strategy": "qa_pair"});
        let result = decode_jsonb_map("doc-x", "chunking_config", Some(raw))
            .expect("well-formed JSONB must decode");
        let map = result.expect("decoded map must be Some");
        assert_eq!(map.get("units_per_chunk").and_then(|v| v.as_i64()), Some(3));
        assert_eq!(map.get("strategy").and_then(|v| v.as_str()), Some("qa_pair"));
    }

    #[test]
    fn decode_jsonb_map_errors_on_non_object_jsonb() {
        // Spec test #3: malformed JSONB (here: a JSON number where an
        // object is required) must NOT silently become None. Returns
        // PipelineRepoError::Deserialization with both the document_id
        // and the column name in the message so an auditor can find
        // the row directly.
        let raw = serde_json::json!(42);
        let err = decode_jsonb_map("doc-malformed", "chunking_config", Some(raw))
            .expect_err("non-object JSONB must error, not silently None");
        match err {
            PipelineRepoError::Deserialization(msg) => {
                assert!(
                    msg.contains("doc-malformed"),
                    "error must name the document_id; got: {msg}"
                );
                assert!(
                    msg.contains("chunking_config"),
                    "error must name the column; got: {msg}"
                );
            }
            other => panic!("expected Deserialization, got {other:?}"),
        }
    }

    #[test]
    fn decode_jsonb_map_errors_on_jsonb_array_instead_of_object() {
        // Variant of the prior test: an array decoding into a map is
        // also a shape mismatch. The spec wants any non-object value
        // surfaced as Deserialization, not silent None.
        let raw = serde_json::json!(["not", "a", "map"]);
        let err = decode_jsonb_map("doc-y", "context_config", Some(raw))
            .expect_err("JSON array must error when a map is expected");
        assert!(matches!(err, PipelineRepoError::Deserialization(_)));
    }

    // ── has_any_override: pins the short-circuit contract ──────────

    #[test]
    fn has_any_override_returns_false_for_empty_overrides() {
        assert!(!has_any_override(&PipelineConfigOverrides::default()));
    }

    /// Spec decision #4: a PATCH whose only field is `chunking_config`
    /// must NOT short-circuit. Pre-Instruction-C, `any_field` did not
    /// include `chunking_config.is_some()` in its OR — so this test
    /// would have returned `false` and the UPDATE would have silently
    /// no-op'd, leaving the operator's override unpersisted.
    #[test]
    fn patch_with_only_chunking_config_does_not_short_circuit() {
        let mut over = HashMap::new();
        over.insert("units_per_chunk".to_string(), serde_json::json!(3));
        let overrides = PipelineConfigOverrides {
            chunking_config: Some(over),
            ..Default::default()
        };
        assert!(
            has_any_override(&overrides),
            "chunking_config override must trigger the UPDATE path"
        );
    }

    /// Same as above but for context_config.
    #[test]
    fn patch_with_only_context_config_does_not_short_circuit() {
        let mut over = HashMap::new();
        over.insert("traversal_depth".to_string(), serde_json::json!(5));
        let overrides = PipelineConfigOverrides {
            context_config: Some(over),
            ..Default::default()
        };
        assert!(has_any_override(&overrides));
    }

    /// `Some(empty_map)` is the explicit "I want to override but with
    /// no keys" signal — it IS a real override (operationally distinct
    /// from None, see PipelineConfigOverrides::chunking_config doc) and
    /// must trigger the UPDATE so the COLUMN gets set to `'{}'::jsonb`.
    #[test]
    fn patch_with_only_empty_chunking_config_map_still_persists() {
        let overrides = PipelineConfigOverrides {
            chunking_config: Some(HashMap::new()),
            ..Default::default()
        };
        assert!(
            has_any_override(&overrides),
            "Some(empty) is still an override and must reach the UPDATE"
        );
    }

    // ── Live-DB integration tests (#[ignore]) ─────────────────────
    //
    // The repository's other CRUD paths are not unit-tested with a
    // live database in this codebase. Round-tripping the new JSONB
    // columns through PostgreSQL is exercised in DEV via the SQL
    // verification block in this commit's instruction (see
    // /home/roman/Downloads/CC_INSTRUCTION_C_chunking_context_config_overrides.md).
    // Marking the spec's requested DB tests as `#[ignore]` here so they
    // surface in `cargo test -- --ignored` for any future contributor
    // who sets up a test DB fixture, but they don't gate the normal
    // test run that has no PG connection.

    #[ignore = "requires a live test database fixture (none in repo today)"]
    #[test]
    fn pipeline_config_chunking_override_round_trips_through_db() {
        // Stub: insert pipeline_config with chunking_config = Some(map),
        // call get_pipeline_config_overrides, assert deep equality.
        // Implement when a #[fixture]-style PG pool helper lands.
    }

    #[ignore = "requires a live test database fixture (none in repo today)"]
    #[test]
    fn pipeline_config_chunking_null_round_trips_through_db_as_none() {
        // Stub: insert with NULL chunking_config column, read back,
        // assert PipelineConfigOverrides.chunking_config == None.
    }

    #[ignore = "requires a live test database fixture (none in repo today)"]
    #[test]
    fn pipeline_config_malformed_chunking_jsonb_returns_deserialization_error() {
        // Stub: write a JSONB number into the chunking_config column
        // via raw SQL (bypassing the typed write path), then call
        // get_pipeline_config_overrides — expect
        // PipelineRepoError::Deserialization carrying the document_id.
        // The pure-unit test `decode_jsonb_map_errors_on_non_object_jsonb`
        // above already covers this contract at the conversion layer;
        // this stub exists for the future end-to-end verification.
    }
}
