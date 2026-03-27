//! POST /api/admin/pipeline/documents/:id/extract-text
//!
//! Opens the document's PDF via colossus-pdf, extracts text page by page,
//! and stores each page in the `document_text` table.
//!
//! ## Rust Learning: spawn_blocking for sync libraries
//!
//! colossus-pdf uses pdf_oxide which is synchronous. Calling sync I/O from
//! an async handler would block the tokio runtime thread. We use
//! `tokio::task::spawn_blocking` to move the PDF work to a dedicated thread
//! pool, then `.await` the result back in async land.

use axum::{
    extract::{Path, State},
    Json,
};

use crate::auth::{require_admin, AuthUser};
use crate::error::AppError;
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository;
use crate::state::AppState;

use super::ExtractTextResponse;

/// POST /api/admin/pipeline/documents/:id/extract-text
///
/// Reads the document's PDF, extracts text page by page using colossus-pdf,
/// stores each page in the `document_text` table, and updates the document
/// status to "TEXT_EXTRACTED".
pub async fn extract_text(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Json<ExtractTextResponse>, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, doc_id = %doc_id, "POST extract-text");

    // 1. Fetch document record
    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal { message: format!("DB error: {e}") })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    // 2. Check status — only allow extraction from UPLOADED or TEXT_EXTRACTED (re-extract)
    if document.status != "UPLOADED" && document.status != "TEXT_EXTRACTED" {
        return Err(AppError::Conflict {
            message: format!(
                "Cannot extract text: document status is '{}', expected 'UPLOADED' or 'TEXT_EXTRACTED'",
                document.status
            ),
            details: serde_json::json!({ "status": document.status }),
        });
    }

    // 3. Build full path and verify PDF exists
    let full_path = format!(
        "{}/{}",
        state.config.document_storage_path.trim_end_matches('/'),
        document.file_path
    );

    if !tokio::fs::try_exists(&full_path).await.unwrap_or(false) {
        return Err(AppError::NotFound {
            message: format!("PDF file not found: {}", document.file_path),
        });
    }

    // 4. Extract text in a blocking thread (colossus-pdf is sync)
    let pdf_path = full_path.clone();
    let pages = tokio::task::spawn_blocking(move || -> Result<Vec<colossus_pdf::PageText>, String> {
        let mut extractor = colossus_pdf::PdfTextExtractor::open(&pdf_path)
            .map_err(|e| format!("Failed to open PDF: {e}"))?;
        extractor
            .extract_all_pages()
            .map_err(|e| format!("Failed to extract pages: {e}"))
    })
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Text extraction task panicked: {e}"),
    })?
    .map_err(|e| AppError::Internal { message: e })?;

    // 5. Insert each page's text into the database
    let page_count = pages.len();
    let mut total_chars: usize = 0;

    for page in &pages {
        pipeline_repository::insert_document_text(
            &state.pipeline_pool,
            &doc_id,
            page.page_number as i32,
            &page.text,
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to insert text for page {}: {e}", page.page_number),
        })?;
        total_chars += page.char_count;
    }

    // 6. Update document status
    pipeline_repository::update_document_status(&state.pipeline_pool, &doc_id, "TEXT_EXTRACTED")
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update document status: {e}"),
        })?;

    tracing::info!(
        doc_id = %doc_id, page_count, total_chars,
        "Text extraction complete"
    );

    log_admin_action(
        &state.audit_repo,
        &user.username,
        "pipeline.document.extract_text",
        Some("document"),
        Some(&doc_id),
        Some(serde_json::json!({
            "page_count": page_count,
            "total_chars": total_chars,
        })),
    )
    .await;

    Ok(Json(ExtractTextResponse {
        document_id: doc_id,
        status: "TEXT_EXTRACTED".to_string(),
        page_count,
        total_chars,
    }))
}
