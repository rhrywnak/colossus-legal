//! Document-table record types and CRUD.
//!
//! Owns the `DocumentRecord` row type returned by every `documents`
//! SELECT, the `DocumentTextRecord` row type for `document_text`, and
//! the insert / update / read paths for both tables.
//!
//! The process-endpoint progress writers (`update_processing_progress`,
//! the cancellation flag toggle, etc.) live in [`super::documents`] —
//! that file is the writeback side of these reads, factored separately
//! because its column set is the Processing-tab UI surface and evolves
//! on a different cadence than the canonical CRUD here.

use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::models::document_status::{RUN_STATUS_COMPLETED, STATUS_NEW, STEP_STATUS_FAILED};

use super::review;
use super::PipelineRepoError;

// ── Record types ─────────────────────────────────────────────────

/// A document record from the `documents` table.
///
/// `#[serde(deny_unknown_fields)]` is defensive against the day this
/// struct is ever deserialized from JSON (e.g. an admin import endpoint
/// or a cached blob): a stale or typo'd field name will fail loudly
/// instead of silently dropping the value. `sqlx::FromRow` decodes from
/// database rows by column name and ignores serde attributes entirely,
/// so the attribute has zero effect on the existing DB read path.
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
#[serde(deny_unknown_fields)]
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
///
/// See [`DocumentRecord`] for the rationale on `deny_unknown_fields` —
/// it is defensive against future JSON deserialization and has no
/// effect on the existing sqlx-FromRow database read path.
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
#[serde(deny_unknown_fields)]
pub struct DocumentTextRecord {
    pub document_id: String,
    pub page_number: i32,
    pub text_content: String,
}

// ── CRUD ─────────────────────────────────────────────────────────

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

/// Count items NOT in [`review::GROUNDED_STATUSES`] for the document and
/// persist the count to `documents.entities_flagged`. Called by Ingest
/// so the Processing-tab grounding stat has a real denominator —
/// the column was previously declared, projected, and rendered but
/// never written, which forced the UI rate to a hardcoded 100%.
///
/// Returns the count for logging.
pub async fn refresh_document_flagged_count(
    pool: &PgPool,
    document_id: &str,
) -> Result<i32, PipelineRepoError> {
    let count = review::count_flagged_items_for_document(pool, document_id).await? as i32;
    sqlx::query("UPDATE documents SET entities_flagged = $2, updated_at = NOW() WHERE id = $1")
        .bind(document_id)
        .bind(count)
        .execute(pool)
        .await?;
    Ok(count)
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
