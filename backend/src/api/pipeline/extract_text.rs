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
use crate::repositories::pipeline_repository::{self, steps};
use crate::state::AppState;

use super::ocr;
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
    let start = std::time::Instant::now();
    tracing::info!(user = %user.username, doc_id = %doc_id, "POST extract-text");

    let step_id = steps::record_step_start(
        &state.pipeline_pool, &doc_id, "extract_text", &user.username, &serde_json::json!({}),
    ).await.map_err(|e| AppError::Internal { message: format!("Step logging: {e}") })?;

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

    // 5. Check OCR tool availability (non-fatal — only matters if pages need OCR)
    let ocr_available = match ocr::check_ocr_tools_available().await {
        Ok(()) => true,
        Err(e) => {
            tracing::warn!("OCR tools not available, scanned pages will have no text: {e}");
            false
        }
    };

    // 6. OCR fallback for pages with insufficient text, then insert into DB
    let page_count = pages.len();
    let total_pages = page_count as u32;
    let mut total_chars: usize = 0;
    let mut pages_native: usize = 0;
    let mut pages_ocr: usize = 0;

    for page in &pages {
        let non_ws = page.text.chars().filter(|c| !c.is_whitespace()).count();
        let text_to_store = if non_ws < ocr::OCR_CHAR_THRESHOLD {
            // Page looks scanned — attempt OCR if tools are available
            if ocr_available {
                tracing::info!(
                    doc_id = %doc_id, page = page.page_number, non_ws_chars = non_ws,
                    "Page {}: only {} non-whitespace chars, attempting OCR",
                    page.page_number, non_ws
                );
                match ocr::ocr_page(&full_path, page.page_number, total_pages).await {
                    Ok(ocr_text) if !ocr_text.trim().is_empty() => {
                        pages_ocr += 1;
                        ocr_text
                    }
                    Ok(_) => {
                        tracing::warn!(
                            doc_id = %doc_id, page = page.page_number,
                            "OCR returned empty text, keeping original"
                        );
                        pages_native += 1;
                        page.text.clone()
                    }
                    Err(e) => {
                        tracing::warn!(
                            doc_id = %doc_id, page = page.page_number,
                            "OCR failed, keeping original text: {e}"
                        );
                        pages_native += 1;
                        page.text.clone()
                    }
                }
            } else {
                pages_native += 1;
                page.text.clone()
            }
        } else {
            pages_native += 1;
            page.text.clone()
        };

        total_chars += text_to_store.len();

        pipeline_repository::insert_document_text(
            &state.pipeline_pool,
            &doc_id,
            page.page_number as i32,
            &text_to_store,
        )
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to insert text for page {}: {e}", page.page_number),
        })?;
    }

    // 7. Update document status
    pipeline_repository::update_document_status(&state.pipeline_pool, &doc_id, "TEXT_EXTRACTED")
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update document status: {e}"),
        })?;

    tracing::info!(
        doc_id = %doc_id, page_count, total_chars,
        pages_native, pages_ocr,
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
            "pages_native": pages_native,
            "pages_ocr": pages_ocr,
        })),
    )
    .await;

    steps::record_step_complete(
        &state.pipeline_pool, step_id, start.elapsed().as_secs_f64(),
        &serde_json::json!({
            "page_count": page_count,
            "total_chars": total_chars,
            "pages_native": pages_native,
            "pages_ocr": pages_ocr,
        }),
    ).await.ok();

    Ok(Json(ExtractTextResponse {
        document_id: doc_id,
        status: "TEXT_EXTRACTED".to_string(),
        page_count,
        total_chars,
    }))
}
