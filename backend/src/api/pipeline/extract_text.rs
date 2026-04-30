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
use crate::models::document_status::{STATUS_TEXT_EXTRACTED, STATUS_UPLOADED};
use crate::repositories::audit_repository::log_admin_action;
use crate::repositories::pipeline_repository::{self, steps};
use crate::state::AppState;

use super::ocr;
use super::ExtractTextResponse;

/// Core logic for text extraction — callable from handler AND process endpoint.
///
/// Extracts text from the document's PDF page by page, stores in `document_text`,
/// auto-detects document type, records pipeline step, and updates status.
/// Does NOT check document status — caller is responsible for validation.
pub(crate) async fn run_extract_text(
    state: &AppState,
    doc_id: &str,
    username: &str,
) -> Result<ExtractTextResponse, AppError> {
    let start = std::time::Instant::now();

    let step_id = steps::record_step_start(
        &state.pipeline_pool,
        doc_id,
        "extract_text",
        username,
        &serde_json::json!({}),
    )
    .await
    .map_err(|e| AppError::Internal {
        message: format!("Step logging: {e}"),
    })?;

    // 1. Fetch document record
    let document = pipeline_repository::get_document(&state.pipeline_pool, doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    // 2. Build full path and verify PDF exists
    let full_path = format!(
        "{}/{}",
        state.config.document_storage_path.trim_end_matches('/'),
        document.file_path
    );

    if !tokio::fs::try_exists(&full_path).await.unwrap_or(false) {
        return Err(AppError::NotFound {
            message: format!("Document file not found: {}", document.file_path),
        });
    }

    // Determine the document format for extractor routing.
    //
    // Primary source: the `original_format` column set at upload time
    // by format detection (magic bytes). Fallback for pre-migration
    // documents: detect from the file on disk. Final fallback: assume PDF
    // (the only format supported before multi-format ingestion).
    //
    // ## Rust Learning: `Option::unwrap_or_else` with a closure
    //
    // `unwrap_or_else` takes a closure that runs only if the Option is None.
    // Unlike `unwrap_or` (which evaluates the default eagerly), the closure
    // here avoids running `detect_format` when the DB value is present.
    // This is the idiomatic Rust pattern for "try the fast path first,
    // fall back to a computation if needed."
    let doc_format = document
        .original_format
        .as_deref()
        .map(String::from)
        .unwrap_or_else(|| {
            match colossus_pdf::detect_format(std::path::Path::new(&full_path)) {
                Ok(fmt) => match fmt {
                    colossus_pdf::DocumentFormat::Pdf => "pdf".to_string(),
                    colossus_pdf::DocumentFormat::Docx => "docx".to_string(),
                    colossus_pdf::DocumentFormat::PlainText => "txt".to_string(),
                },
                Err(e) => {
                    tracing::warn!(
                        doc_id = %doc_id,
                        error = %e,
                        "Format detection failed in extract_text — assuming PDF"
                    );
                    "pdf".to_string()
                }
            }
        });

    tracing::info!(doc_id = %doc_id, format = %doc_format, "ExtractText: routing by format");

    // 3. Extract text — route to the correct extractor based on format.
    //
    // PDF path: existing PdfTextExtractor + Surya OCR for scanned pages.
    //   This preserves all current behavior including page-level OCR fallback.
    //
    // DOCX/TXT path: use colossus-pdf's DocumentExtractor trait. These
    //   extractors are synchronous (run in spawn_blocking) and produce
    //   Vec<ExtractedPage>. No OCR needed — DOCX and TXT are always text.
    //
    // After extraction, ALL formats run through text normalization to fix
    // known artifacts (e.g., "10.During" → "10. During" from PDF conversion).

    let page_count: usize;
    let mut total_chars: usize = 0;
    let mut pages_native: usize = 0;
    let mut pages_ocr: usize = 0;
    let mut first_page_text = String::new();

    // Recommended normalization rules from colossus-pdf. Computed once and
    // borrowed in each per-page `normalize_text` call so the rule set isn't
    // re-allocated for every page.
    let normalize_rules = colossus_pdf::NormalizationRule::all();

    if doc_format == "pdf" {
        // ── PDF extraction path (existing behavior, unchanged) ──────

        let pdf_path = full_path.clone();
        let pages =
            tokio::task::spawn_blocking(move || -> Result<Vec<colossus_pdf::PageText>, String> {
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

        // Check Surya OCR availability
        let ocr_available = match ocr::check_surya_available(&state.http_client).await {
            Ok(()) => true,
            Err(e) => {
                tracing::warn!(
                    doc_id = %doc_id,
                    error = %e,
                    "Surya OCR service not available — scanned pages will fail if no native text exists. \
                     Fix: ensure SURYA_OCR_URL points to a running Surya service."
                );
                false
            }
        };

        // Batch-OCR scanned pages via Surya
        let scanned_page_numbers: Vec<u32> = pages
            .iter()
            .filter(|p| {
                p.text.chars().filter(|c| !c.is_whitespace()).count() < ocr::OCR_CHAR_THRESHOLD
            })
            .map(|p| p.page_number)
            .collect();

        let surya_results: std::collections::HashMap<u32, String> =
            if !scanned_page_numbers.is_empty() && ocr_available {
                match ocr::ocr_full_document_surya(
                    &state.http_client,
                    &full_path,
                    Some(&scanned_page_numbers),
                )
                .await
                {
                    Ok(response) => {
                        tracing::info!(
                            doc_id = %doc_id,
                            pages_ocr = response.pages_processed,
                            elapsed = response.elapsed_secs,
                            "Surya OCR returned {} pages",
                            response.pages.len()
                        );
                        response
                            .pages
                            .into_iter()
                            .map(|p| (p.page_number, p.text))
                            .collect()
                    }
                    Err(e) => {
                        tracing::warn!(
                            doc_id = %doc_id, error = %e,
                            "Surya OCR failed — scanned pages will have no usable text"
                        );
                        std::collections::HashMap::new()
                    }
                }
            } else {
                std::collections::HashMap::new()
            };

        page_count = pages.len();

        // Store pages with OCR fallback and text normalization
        for page in &pages {
            let non_ws = page.text.chars().filter(|c| !c.is_whitespace()).count();
            let raw_text = if non_ws < ocr::OCR_CHAR_THRESHOLD {
                if let Some(surya_text) = surya_results.get(&page.page_number) {
                    if !surya_text.trim().is_empty() {
                        pages_ocr += 1;
                        surya_text.clone()
                    } else {
                        tracing::warn!(
                            doc_id = %doc_id, page = page.page_number,
                            "Surya returned empty text; keeping native"
                        );
                        pages_native += 1;
                        page.text.clone()
                    }
                } else {
                    pages_native += 1;
                    page.text.clone()
                }
            } else {
                pages_native += 1;
                page.text.clone()
            };

            // Apply text normalization to fix known artifacts (e.g., PDF spacing).
            // Normalization runs on ALL text regardless of source (native or OCR).
            let text_to_store = colossus_pdf::normalize_text(&raw_text, &normalize_rules);

            if page.page_number == 1 {
                first_page_text = text_to_store.clone();
            }

            total_chars += text_to_store.len();

            pipeline_repository::insert_document_text(
                &state.pipeline_pool,
                doc_id,
                page.page_number as i32,
                &text_to_store,
            )
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to insert text for page {}: {e}", page.page_number),
            })?;
        }
    } else {
        // ── DOCX / TXT extraction path (new) ────────────────────────
        //
        // ## Rust Learning: trait objects via `extractor_for_format`
        //
        // `extractor_for_format` returns `Box<dyn DocumentExtractor>` — a
        // heap-allocated trait object. The caller doesn't know (or care)
        // whether it got a DocxExtractor or PlainTextExtractor. It calls
        // `extractor.extract()` through dynamic dispatch (vtable lookup).
        // This is the Strategy pattern implemented via Rust traits.

        // Parse the format string back to the DocumentFormat enum.
        let format_enum = match doc_format.as_str() {
            "docx" => colossus_pdf::DocumentFormat::Docx,
            "txt" => colossus_pdf::DocumentFormat::PlainText,
            other => {
                return Err(AppError::Internal {
                    message: format!(
                        "Unsupported format '{other}' for document '{doc_id}'. \
                         Expected 'pdf', 'docx', or 'txt'."
                    ),
                });
            }
        };

        let extractor = colossus_pdf::extractor_for_format(format_enum);
        let extract_path = full_path.clone();

        // DocumentExtractor::extract is synchronous — run in spawn_blocking
        // to avoid blocking the tokio runtime thread.
        let extracted_pages = tokio::task::spawn_blocking(move || {
            extractor.extract(std::path::Path::new(&extract_path))
        })
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Text extraction task panicked: {e}"),
        })?
        .map_err(|e| AppError::Internal {
            message: format!("Failed to extract text from {doc_format} file: {e}"),
        })?;

        page_count = extracted_pages.len();
        pages_native = page_count; // DOCX/TXT are always native text, never OCR

        for page in &extracted_pages {
            // Apply text normalization — same rules as PDF path.
            let text_to_store = colossus_pdf::normalize_text(&page.text_content, &normalize_rules);

            if page.page_number == 1 {
                first_page_text = text_to_store.clone();
            }

            total_chars += text_to_store.len();

            pipeline_repository::insert_document_text(
                &state.pipeline_pool,
                doc_id,
                page.page_number,
                &text_to_store,
            )
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to insert text for page {}: {e}", page.page_number),
            })?;
        }
    }

    // 5b. Fail fast if ALL pages have zero usable text. Continuing would waste
    //     LLM API spend on empty input and produce a "complete" document with
    //     zero entities. For PDFs this typically means Surya was unavailable
    //     on a scanned PDF or the PDF has no extractable content at all. For
    //     DOCX/TXT it means the file was empty or the extractor returned only
    //     whitespace.
    if total_chars == 0 {
        let detail = if doc_format == "pdf" {
            "Check SURYA_OCR_URL configuration and ensure the Surya service is running, \
             or verify the PDF actually contains text or scanned images."
        } else {
            "Verify the file is not empty and contains readable text."
        };
        return Err(AppError::Internal {
            message: format!(
                "No usable text extracted from {page_count} pages of document '{doc_id}' \
                 (format: {doc_format}). {detail}"
            ),
        });
    }

    // 6. Auto-detect document type if current type is "auto" or "unknown"
    let detected_type = detect_document_type(&first_page_text);
    if document.document_type == "auto" || document.document_type == "unknown" {
        sqlx::query("UPDATE documents SET document_type = $1, updated_at = NOW() WHERE id = $2")
            .bind(detected_type)
            .bind(doc_id)
            .execute(&state.pipeline_pool)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to update document_type: {e}"),
            })?;

        // When document type was auto-detected, also update pipeline_config
        // to use the correct schema for the detected type.
        // This ensures pipeline_config.schema_file is always consistent with
        // documents.document_type after extract_text runs. The schema is
        // read from the detected type's profile YAML — same source of truth
        // upload uses.
        let detected_schema = super::upload::schema_file_for_document_type(
            &state.config.processing_profile_dir,
            detected_type,
        );
        sqlx::query("UPDATE pipeline_config SET schema_file = $1 WHERE document_id = $2")
            .bind(&detected_schema)
            .bind(doc_id)
            .execute(&state.pipeline_pool)
            .await
            .map_err(|e| AppError::Internal {
                message: format!("Failed to update pipeline_config schema_file: {e}"),
            })?;
        tracing::info!(
            doc_id = %doc_id, detected_type, schema = %detected_schema,
            "Updated pipeline_config.schema_file after auto-detection"
        );
    }

    // DO NOT set document status here.
    //
    // ## Why individual steps must not set document status
    //
    // run_extract_text is called both:
    // (a) directly from extract_text handler (the standalone /extract-text endpoint), and
    // (b) from run_pipeline in process.rs (the automated one-button pipeline).
    //
    // When called from run_pipeline, setting status = "TEXT_EXTRACTED" here is
    // incorrect. The orchestrator (run_pipeline) owns all status transitions.
    // Setting an intermediate status from inside a step creates two problems:
    //
    // 1. The document briefly shows "TEXT_EXTRACTED" status in the UI while it is
    //    still being processed — confusing to the user.
    //
    // 2. If a later step fails, the spawn block sets status = "FAILED". But
    //    the intermediate "TEXT_EXTRACTED" update has already been committed to the
    //    database. There is a window where status = "TEXT_EXTRACTED" is visible
    //    between the text extraction completing and the failure being recorded.
    //
    // When called from extract_text handler directly (standalone admin use),
    // the caller sets the appropriate status after this function returns.
    //
    // The correct architecture: run_* functions return results. Callers
    // decide what status to set based on those results.

    let step_summary = serde_json::json!({
        "page_count": page_count,
        "total_chars": total_chars,
        "pages_native": pages_native,
        "pages_ocr": pages_ocr,
        "detected_type": detected_type,
    });

    tracing::info!(
        doc_id = %doc_id, page_count, total_chars,
        pages_native, pages_ocr, detected_type,
        "Text extraction complete"
    );

    log_admin_action(
        &state.audit_repo,
        username,
        "pipeline.document.extract_text",
        Some("document"),
        Some(doc_id),
        Some(step_summary.clone()),
    )
    .await;

    if let Err(e) = steps::record_step_complete(
        &state.pipeline_pool,
        step_id,
        start.elapsed().as_secs_f64(),
        &step_summary,
    )
    .await
    {
        tracing::error!(
            document_id = %doc_id,
            step_id = step_id,
            error = %e,
            "Failed to record extract_text step completion — audit trail gap"
        );
    }

    Ok(ExtractTextResponse {
        document_id: doc_id.to_string(),
        status: STATUS_TEXT_EXTRACTED.to_string(),
        page_count,
        total_chars,
    })
}

/// POST /api/admin/pipeline/documents/:id/extract-text
///
/// HTTP handler — thin wrapper around `run_extract_text`.
/// Checks admin auth and status guard, then delegates to core logic.
pub async fn extract_text(
    user: AuthUser,
    State(state): State<AppState>,
    Path(doc_id): Path<String>,
) -> Result<Json<ExtractTextResponse>, AppError> {
    require_admin(&user)?;
    tracing::info!(user = %user.username, doc_id = %doc_id, "POST extract-text");

    // Status guard — only allow from UPLOADED or TEXT_EXTRACTED (re-extract)
    let document = pipeline_repository::get_document(&state.pipeline_pool, &doc_id)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("DB error: {e}"),
        })?
        .ok_or_else(|| AppError::NotFound {
            message: format!("Document '{doc_id}' not found"),
        })?;

    if document.status != STATUS_UPLOADED && document.status != STATUS_TEXT_EXTRACTED {
        return Err(AppError::Conflict {
            message: format!(
                "Cannot extract text: document status is '{}', expected '{STATUS_UPLOADED}' or '{STATUS_TEXT_EXTRACTED}'",
                document.status
            ),
            details: serde_json::json!({ "status": document.status }),
        });
    }

    let result = run_extract_text(&state, &doc_id, &user.username).await?;

    // extract_text handler is the standalone admin endpoint, not the automated pipeline.
    // When called directly, we set TEXT_EXTRACTED status here — the orchestrator pattern.
    // run_extract_text no longer sets this status itself (see comment in run_extract_text).
    pipeline_repository::update_document_status(&state.pipeline_pool, &doc_id, STATUS_TEXT_EXTRACTED)
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Failed to update status: {e}"),
        })?;

    Ok(Json(result))
}

// ── Document type auto-detection ────────────────────────────────

/// Detect document type from the first page's text using keyword matching.
///
/// ## Rust Learning: &'static str return
///
/// Returning `&'static str` (a string literal) avoids allocating a new String.
/// The string data lives in the compiled binary, so the returned reference
/// is valid for the entire program lifetime.
pub(crate) fn detect_document_type(first_page_text: &str) -> &'static str {
    let upper = first_page_text.to_uppercase();

    if upper.contains("AFFIDAVIT") {
        "affidavit"
    } else if upper.contains("INTERROGATOR") || upper.contains("REQUEST FOR ADMISSION") {
        "discovery_response"
    } else if upper.contains("MOTION FOR") || upper.contains("MOTION TO") {
        "motion"
    } else if upper.contains("OPINION AND ORDER")
        || upper.contains("ORDER OF THE COURT")
        || upper.contains("COURT OF APPEALS")
    {
        "court_ruling"
    } else if upper.contains("BRIEF") || upper.contains("APPELLANT") || upper.contains("APPELLEE") {
        "brief"
    } else if upper.contains("COMPLAINT") {
        "complaint"
    } else {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── detect_document_type tests ──────────────────────────────

    /// Document type detection uses keyword matching on first-page text.
    /// These tests document every keyword and verify the priority order
    /// (affidavit checked before brief, complaint checked last).

    #[test]
    fn test_detects_complaint() {
        assert_eq!(
            detect_document_type("STATE OF MICHIGAN\nCOMPLAINT\nPlaintiff vs Defendant"),
            "complaint"
        );
        assert_eq!(
            detect_document_type("VERIFIED COMPLAINT FOR DAMAGES"),
            "complaint"
        );
    }

    #[test]
    fn test_detects_affidavit() {
        assert_eq!(detect_document_type("AFFIDAVIT OF JOHN SMITH"), "affidavit");
        // Affidavit takes priority over other keywords
        assert_eq!(
            detect_document_type("AFFIDAVIT IN SUPPORT OF MOTION FOR SUMMARY JUDGMENT"),
            "affidavit"
        );
    }

    #[test]
    fn test_detects_discovery_response() {
        assert_eq!(
            detect_document_type("PLAINTIFF'S RESPONSES TO INTERROGATORIES"),
            "discovery_response"
        );
        assert_eq!(
            detect_document_type("REQUEST FOR ADMISSION NUMBER 1"),
            "discovery_response"
        );
    }

    #[test]
    fn test_detects_motion() {
        assert_eq!(
            detect_document_type("MOTION FOR SUMMARY JUDGMENT"),
            "motion"
        );
        assert_eq!(detect_document_type("MOTION TO DISMISS"), "motion");
    }

    #[test]
    fn test_detects_court_ruling() {
        assert_eq!(detect_document_type("OPINION AND ORDER"), "court_ruling");
        assert_eq!(detect_document_type("ORDER OF THE COURT"), "court_ruling");
        assert_eq!(
            detect_document_type("COURT OF APPEALS STATE OF MICHIGAN"),
            "court_ruling"
        );
    }

    #[test]
    fn test_detects_brief() {
        assert_eq!(detect_document_type("APPELLANT'S BRIEF ON APPEAL"), "brief");
        assert_eq!(detect_document_type("BRIEF IN SUPPORT OF MOTION"), "brief");
        assert_eq!(detect_document_type("APPELLEE'S RESPONSE BRIEF"), "brief");
    }

    #[test]
    fn test_unknown_when_no_keywords_match() {
        assert_eq!(detect_document_type(""), "unknown");
        assert_eq!(
            detect_document_type("lorem ipsum dolor sit amet"),
            "unknown"
        );
        assert_eq!(detect_document_type("EXHIBIT A"), "unknown");
    }

    #[test]
    fn test_detection_is_case_insensitive() {
        // The function uppercases input before matching, so lowercase input works.
        assert_eq!(detect_document_type("complaint for damages"), "complaint");
        assert_eq!(detect_document_type("affidavit of facts"), "affidavit");
    }

    #[test]
    fn test_affidavit_priority_over_motion() {
        // "AFFIDAVIT" should win over "MOTION FOR" if both appear.
        // This documents the priority order of the if-else chain.
        assert_eq!(
            detect_document_type("AFFIDAVIT IN SUPPORT OF MOTION FOR SUMMARY JUDGMENT"),
            "affidavit",
            "Affidavit keyword takes priority over motion keyword"
        );
    }

    #[test]
    fn test_complaint_is_last_priority() {
        // Complaint is the last check — other keywords take priority.
        // A complaint that also says "COURT OF APPEALS" would be detected
        // as court_ruling, not complaint. This is correct behavior.
        assert_eq!(
            detect_document_type("COURT OF APPEALS\nCOMPLAINT ON APPEAL"),
            "court_ruling",
            "Court ruling keyword takes priority over complaint keyword"
        );
    }
}
