//! Pipeline admin endpoints: document upload and PDF text extraction.
//!
//! These endpoints operate against the pipeline database (`colossus_legal_v2`)
//! via `state.pipeline_pool`. They are the first two steps of the extraction
//! pipeline: get the PDF in, then extract its text page by page.
//!
//! ## Rust Learning: Module directory layout
//!
//! When a module grows past 300 lines, Rust lets you split it into a directory:
//! `pipeline.rs` becomes `pipeline/mod.rs` + `pipeline/upload.rs` + etc.
//! The `mod.rs` file re-exports the public items so callers don't change.

mod anthropic;
mod completeness;
mod completeness_helpers;
mod extract;
mod extract_text;
mod index;
mod ingest;
mod ingest_helpers;
pub mod report;
mod upload;
pub mod verify;

pub use completeness::completeness_handler;
pub use extract::extract_handler;
pub use extract_text::extract_text;
pub use index::index_handler;
pub use ingest::ingest_handler;
pub use report::report_handler;
pub use upload::upload_document;
pub use verify::verify_handler;

use serde::Serialize;

use crate::error::AppError;
use crate::repositories::pipeline_repository::DocumentRecord;

/// Maximum upload size: 50 MB.
pub(crate) const MAX_FILE_SIZE: usize = 50 * 1024 * 1024;

// ── Shared Response DTOs ─────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct UploadDocumentResponse {
    pub document: DocumentRecord,
}

#[derive(Debug, Serialize)]
pub struct ExtractTextResponse {
    pub document_id: String,
    pub status: String,
    pub page_count: usize,
    pub total_chars: usize,
}

#[derive(Debug, Serialize)]
pub struct ExtractResponse {
    pub document_id: String,
    pub status: String,
    pub run_id: i32,
    pub model: String,
    pub entity_count: usize,
    pub relationship_count: usize,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub elapsed_secs: f64,
}

// ── Shared Helpers ───────────────────────────────────────────────

/// Read a multipart text field's value as a String.
pub(crate) async fn field_text(
    name: &str,
    field: axum::extract::multipart::Field<'_>,
) -> Result<String, AppError> {
    field.text().await.map_err(|e| AppError::BadRequest {
        message: format!("Failed to read field '{name}': {e}"),
        details: serde_json::json!({}),
    })
}

/// Return a required field's value or a 400 error.
pub(crate) fn require_field(name: &str, value: Option<String>) -> Result<String, AppError> {
    value.ok_or_else(|| AppError::BadRequest {
        message: format!("Missing required field: '{name}'"),
        details: serde_json::json!({}),
    })
}
