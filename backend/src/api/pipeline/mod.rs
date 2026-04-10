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
pub mod completeness_validation;
mod config_endpoints;
pub(crate) mod constants;
mod document_response;
mod delete;
mod errors;
mod extract;
mod extract_text;
mod file;
mod history;
mod index;
mod items;
mod metrics;
mod ocr;
mod ingest;
mod ingest_helpers;
mod ingest_resolver;
pub mod report;
mod review;
pub mod state_machine;
mod upload;
pub mod users;
pub mod verify;
mod workload;

pub use completeness::completeness_handler;
pub use delete::delete_document;
pub use history::history_handler;
pub use extract::extract_handler;
pub use extract_text::extract_text;
pub use index::index_handler;
pub use ingest::ingest_handler;
pub use report::report_handler;
pub use upload::upload_document;
pub use verify::verify_handler;

use axum::{
    extract::State,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Serialize;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::repositories::pipeline_repository::{self, DocumentRecord};
use crate::state::AppState;

/// Self-contained pipeline router.
///
/// ## Rust Learning: Composable Routers
///
/// This router uses relative paths (e.g., `/documents/:id/ingest`).
/// The application decides where to mount it via `Router::nest()`.
/// In colossus-legal: `.nest("/admin/pipeline", pipeline::router())`
/// This pattern makes the pipeline module reusable across Colossus
/// projects without modifying any pipeline code.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/documents", get(list_documents_handler).post(upload_document))
        .route("/documents/errors", get(errors::errors_handler))
        .route("/documents/:id", delete(delete_document))
        .route("/documents/:id/extract-text", post(extract_text))
        .route("/documents/:id/extract", post(extract_handler))
        .route("/documents/:id/verify", post(verify_handler))
        .route("/documents/:id/ingest", post(ingest_handler))
        .route("/documents/:id/index", post(index_handler))
        .route("/documents/:id/completeness", get(completeness_handler))
        .route("/documents/:id/report", get(report_handler))
        .route("/documents/:id/actions", get(state_machine::get_document_actions))
        .route("/documents/:id/history", get(history_handler))
        .route("/documents/:id/items", get(items::list_items_handler))
        .route("/documents/:id/approve-all", post(review::bulk_approve_handler))
        .route("/items/:id/approve", post(review::approve_handler))
        .route("/items/:id/reject", post(review::reject_handler))
        .route("/items/:id", put(review::edit_handler))
        .route("/metrics", get(metrics::metrics_handler))
        .route("/models", get(config_endpoints::list_models))
        .route("/schemas", get(config_endpoints::list_schemas))
        .route("/templates", get(config_endpoints::list_templates))
        .route("/documents/:id/assign", put(users::assign_reviewer_handler))
        .route("/documents/:id/file", get(file::file_handler))
        .route("/reviewers/workload", get(workload::workload_handler))
}

/// GET /documents — list all pipeline documents with computed fields.
///
/// Open to all authenticated users (no admin check). Every user can see the
/// document list; only processing endpoints (extract, ingest, etc.) require admin.
/// Response includes `visible_tabs`, `can_view`, and `status_group` computed
/// from the user's role and the document's status — so the frontend never
/// needs to compare status strings or check user roles.
async fn list_documents_handler(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<Vec<document_response::DocumentResponse>>, AppError> {
    let docs = pipeline_repository::list_all_documents(&state.pipeline_pool)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?;

    let enriched = docs.into_iter()
        .map(|doc| document_response::enrich_document(doc, &user))
        .collect();

    Ok(Json(enriched))
}

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
