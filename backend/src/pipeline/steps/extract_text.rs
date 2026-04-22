//! backend/src/pipeline/steps/extract_text.rs
//!
//! ExtractText pipeline step — extracts per-page text from a PDF into the
//! `document_text` table, with OCR fallback for scanned pages. Wraps the
//! existing sync `colossus_pdf::PdfTextExtractor` via `spawn_blocking` and
//! delegates to `api::pipeline::ocr::ocr_page_with_config` for the OCR path.
//!
//! ## Research-grounded design notes
//!
//! - **OcrConfig resolution precedence (v5_2 Part 10.1 + Part 14):**
//!   `pipeline_config.step_config["ExtractText"]` → `PIPELINE_OCR_*` env vars
//!   → compiled defaults. Missing keys fall through per-field, not
//!   all-or-nothing — a JSONB object that overrides only `ocr_dpi` still
//!   picks up `char_threshold`, `lang`, and `oem` from env vars / defaults.
//!
//! - **Idempotency is free on the DB side.**
//!   [`pipeline_repository::insert_document_text`] uses
//!   `ON CONFLICT (document_id, page_number) DO UPDATE`. Re-running the step
//!   writes identical rows. No cleanup-then-write compromise is needed
//!   (unlike P4-5 Ingest, which had to cleanup first because its helpers
//!   use CREATE rather than MERGE).
//!
//! - **Temp PNG cleanup is RAII via `tempfile::TempDir`** inside
//!   `ocr::run_ocr_subprocesses`. `Drop` runs on every return path (Ok,
//!   `?`-propagated Err, panic unwind). `on_cancel` does NOT need to clean
//!   temp files — it only needs to delete partial `document_text` rows so
//!   the document's visible state reverts to pre-ExtractText.
//!
//! - **Child-process kill-on-drop** (per `tokio::process` docs): spawned
//!   processes continue running after the `Child` handle is dropped by
//!   default. When the executor cancels this step via `tokio::select!`, the
//!   step future drops mid-await. `ocr.rs`'s shared subprocess helper sets
//!   `.kill_on_drop(true)` on both `pdftoppm` and `tesseract` so an
//!   in-flight OCR dies with the step rather than running on as a zombie.
//!
//! - **Legacy `pipeline_steps` writes preserved** during the Phase 4/5
//!   transition — the frontend execution-history panel reads that table.
//!   Same transitional pattern as P4-5's `STATUS_INGESTED` write. Tracked
//!   for Phase 5+ removal.
//!
//! ## Rust Learning: per-field fallthrough config resolution
//!
//! The three config layers (defaults → env → step_config) are applied in
//! increasing-priority order by mutating a single `OcrConfig` value, with
//! each `if let` guard only overwriting a field when that layer's key is
//! actually present. This means a JSONB override of `ocr_dpi` alone cleanly
//! composes with `PIPELINE_OCR_LANG` from the environment and the compiled
//! default for `char_threshold` — no all-or-nothing replacement semantics.

use std::error::Error;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use colossus_pipeline::cancel::CancellationToken;
use colossus_pipeline::progress::ProgressReporter;
use colossus_pipeline::{Step, StepResult};

use crate::api::pipeline::extract_text::detect_document_type;
use crate::api::pipeline::ocr::{self, OcrError};
use crate::api::pipeline::upload::schema_for_document_type;
use crate::pipeline::context::AppContext;
use crate::pipeline::steps::llm_extract::LlmExtract;
use crate::pipeline::task::DocProcessing;
use crate::repositories::pipeline_repository;

// ── OcrConfig ───────────────────────────────────────────────────

/// OCR configuration resolved from `step_config` JSONB → env vars → defaults.
///
/// Per v5_2 Part 10.1 (step_config JSONB shape) and Part 14 (env var names).
/// Fields are applied per-key, so partial overrides compose cleanly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcrConfig {
    /// Which OCR engine to use: "surya" (GPU service, default) or "surya-cpu" (local, slow).
    pub ocr_engine: String,
    /// Pages whose native-extraction text has fewer non-whitespace chars
    /// than this are treated as scanned and routed through OCR.
    pub char_threshold: usize,
    /// Rendering resolution for pdftoppm (dots per inch).
    pub dpi: u32,
    /// Tesseract language code (e.g. "eng"). Passed to `tesseract -l`.
    pub lang: String,
    /// Tesseract OCR engine mode. Passed to `tesseract --oem`.
    pub oem: u32,
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            ocr_engine: "surya".to_string(),
            char_threshold: 50,
            dpi: 300,
            lang: "eng".to_string(),
            oem: 1,
        }
    }
}

impl OcrConfig {
    /// Resolve OCR config for a document by layering three sources in
    /// increasing-priority order: compiled defaults → `PIPELINE_OCR_*` env
    /// vars → `pipeline_config.step_config['ExtractText']` JSONB.
    ///
    /// Missing keys fall through per-field — a JSONB object that overrides
    /// only `ocr_dpi` still picks up `char_threshold`, `lang`, and `oem`
    /// from env vars / defaults.
    pub async fn resolve(db: &PgPool, document_id: &str) -> Self {
        let mut cfg = Self::default();

        // Layer 1: env vars (parse failures are logged and ignored).
        if let Ok(v) = std::env::var("PIPELINE_OCR_ENGINE") {
            cfg.ocr_engine = v;
        }
        if let Ok(v) = std::env::var("PIPELINE_OCR_CHAR_THRESHOLD") {
            if let Ok(n) = v.parse() {
                cfg.char_threshold = n;
            }
        }
        if let Ok(v) = std::env::var("PIPELINE_OCR_DPI") {
            if let Ok(n) = v.parse() {
                cfg.dpi = n;
            }
        }
        if let Ok(v) = std::env::var("PIPELINE_OCR_LANG") {
            cfg.lang = v;
        }
        if let Ok(v) = std::env::var("PIPELINE_OCR_OEM") {
            if let Ok(n) = v.parse() {
                cfg.oem = n;
            }
        }

        // Layer 2: step_config JSONB overrides (highest priority).
        let step_cfg: Option<serde_json::Value> = sqlx::query_scalar(
            "SELECT step_config -> 'ExtractText' FROM pipeline_config \
             WHERE document_id = $1 LIMIT 1",
        )
        .bind(document_id)
        .fetch_optional(db)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(
                doc_id = %document_id, error = %e,
                "OcrConfig::resolve: pipeline_config read failed — using env/defaults"
            );
            None
        });

        if let Some(cfg_json) = step_cfg.filter(|v| !v.is_null()) {
            if let Some(s) = cfg_json.get("ocr_engine").and_then(|v| v.as_str()) {
                cfg.ocr_engine = s.to_string();
            }
            if let Some(n) = cfg_json.get("ocr_char_threshold").and_then(|v| v.as_u64()) {
                cfg.char_threshold = n as usize;
            }
            if let Some(n) = cfg_json.get("ocr_dpi").and_then(|v| v.as_u64()) {
                cfg.dpi = n as u32;
            }
            if let Some(s) = cfg_json.get("ocr_lang").and_then(|v| v.as_str()) {
                cfg.lang = s.to_string();
            }
            if let Some(n) = cfg_json.get("ocr_oem").and_then(|v| v.as_u64()) {
                cfg.oem = n as u32;
            }
        }

        cfg
    }
}

// ── Error type (Kazlauskas G6 — Display omits {source}) ─────────

/// Failure modes for the ExtractText step.
///
/// Display strings deliberately omit `{source}` so log output does not
/// duplicate the inner message (Kazlauskas Guideline 6). Source chains are
/// preserved via `#[source]` where applicable.
#[derive(Debug, thiserror::Error)]
pub enum ExtractTextError {
    #[error("document not found: {doc_id}")]
    DocumentNotFound { doc_id: String },

    #[error("PDF file not found on disk: {path}")]
    PdfNotFound { path: String },

    #[error("PDF text extraction failed: {message}")]
    PdfExtractionFailed { message: String },

    /// Emitted when the per-page loop completes but every page stored zero
    /// characters. Historically the pipeline continued into LlmExtract on
    /// empty text, wasting LLM spend and producing a "complete" document
    /// with zero entities. Failing here stops the pipeline and surfaces
    /// an actionable error to the user.
    #[error(
        "No usable text extracted from {page_count} pages of document '{doc_id}' \
         (OCR available: {ocr_available})"
    )]
    NoUsableText {
        doc_id: String,
        page_count: usize,
        ocr_available: bool,
    },

    #[error("OCR tools unavailable")]
    OcrToolsMissing {
        #[source]
        source: OcrError,
    },

    #[error("database write failed: {message}")]
    DbWrite { message: String },

    #[error("cancelled")]
    Cancelled,
}

// ── Step struct ─────────────────────────────────────────────────

/// The ExtractText step variant's payload.
///
/// The `document_id` field is what gets threaded through the pipeline: the
/// variant is constructed at upload time and every subsequent step fetches
/// its own working data from the database keyed on this id.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtractText {
    pub document_id: String,
}

#[async_trait]
impl Step<DocProcessing> for ExtractText {
    const DEFAULT_RETRY_LIMIT: i32 = 2;
    const DEFAULT_RETRY_DELAY_SECS: u64 = 5;
    const DEFAULT_TIMEOUT_SECS: Option<u64> = Some(180);

    async fn execute(
        self,
        db: &PgPool,
        context: &AppContext,
        cancel: &CancellationToken,
        progress: &ProgressReporter,
    ) -> Result<StepResult<DocProcessing>, Box<dyn Error + Send + Sync>> {
        self.run_extract_text(db, context, cancel, progress)
            .await
            .map_err(|e| -> Box<dyn Error + Send + Sync> { Box::new(e) })
    }

    async fn on_cancel(
        self,
        db: &PgPool,
        _context: &AppContext,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Remove partial per-page inserts so a retry starts clean. The
        // `ON CONFLICT DO UPDATE` in insert_document_text would also make
        // retries correct, but deleting rows makes the document's state
        // visibly "pre-ExtractText" — matching the semantic of a cancel.
        sqlx::query("DELETE FROM document_text WHERE document_id = $1")
            .bind(&self.document_id)
            .execute(db)
            .await
            .map_err(|e| -> Box<dyn Error + Send + Sync> {
                Box::new(ExtractTextError::DbWrite {
                    message: format!("on_cancel delete document_text: {e}"),
                })
            })?;
        Ok(())
    }
}

// ── Core implementation ─────────────────────────────────────────

impl ExtractText {
    /// Internal: perform the full text-extraction write path. Called from
    /// [`Step::execute`] via the thin wrapper above.
    async fn run_extract_text(
        &self,
        db: &PgPool,
        context: &AppContext,
        cancel: &CancellationToken,
        progress: &ProgressReporter,
    ) -> Result<StepResult<DocProcessing>, ExtractTextError> {
        let doc_id = self.document_id.as_str();

        if cancel.is_cancelled().await {
            return Err(ExtractTextError::Cancelled);
        }

        // [2] Fetch document; build full path; verify PDF exists.
        let document = pipeline_repository::get_document(db, doc_id)
            .await
            .map_err(|e| ExtractTextError::DbWrite {
                message: format!("get_document: {e}"),
            })?
            .ok_or_else(|| ExtractTextError::DocumentNotFound {
                doc_id: doc_id.to_string(),
            })?;

        let full_path = format!(
            "{}/{}",
            context.document_storage_path.trim_end_matches('/'),
            document.file_path
        );
        if !tokio::fs::try_exists(&full_path).await.unwrap_or(false) {
            return Err(ExtractTextError::PdfNotFound { path: full_path });
        }

        // [3] Resolve OcrConfig (step_config → env → defaults).
        let cfg = OcrConfig::resolve(db, doc_id).await;
        tracing::info!(
            doc_id = %doc_id, ?cfg,
            "ExtractText: resolved OCR config"
        );

        // [4] Extract PDF text in a blocking thread (colossus-pdf is sync).
        let pdf_path = full_path.clone();
        let pages = tokio::task::spawn_blocking(
            move || -> Result<Vec<colossus_pdf::PageText>, String> {
                let mut extractor = colossus_pdf::PdfTextExtractor::open(&pdf_path)
                    .map_err(|e| format!("open: {e}"))?;
                extractor
                    .extract_all_pages()
                    .map_err(|e| format!("extract_all_pages: {e}"))
            },
        )
        .await
        .map_err(|e| ExtractTextError::PdfExtractionFailed {
            message: format!("spawn_blocking join: {e}"),
        })?
        .map_err(|e| ExtractTextError::PdfExtractionFailed { message: e })?;

        if cancel.is_cancelled().await {
            return Err(ExtractTextError::Cancelled);
        }

        // [5] Surya OCR service availability (non-fatal — scanned pages just won't OCR).
        let ocr_available = match ocr::check_surya_available(&context.http_client).await {
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

        // [5b] Batch-OCR scanned pages via Surya service (one HTTP call for all pages).
        //      Unlike the legacy per-page tesseract path, Surya processes the whole
        //      PDF in a single GPU batch. We collect page numbers that fall below
        //      the char threshold, then pull text out of the response by page number.
        let scanned_page_numbers: Vec<u32> = pages
            .iter()
            .filter(|p| {
                p.text.chars().filter(|c| !c.is_whitespace()).count() < cfg.char_threshold
            })
            .map(|p| p.page_number)
            .collect();

        let surya_results: std::collections::HashMap<u32, String> =
            if !scanned_page_numbers.is_empty() && ocr_available {
                match ocr::ocr_full_document_surya(
                    &context.http_client,
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

        // [6] Per-page loop: OCR fallback when native extraction is too
        //     sparse; insert into document_text (ON CONFLICT handles
        //     idempotency). First-page text is captured for auto-detect.
        let page_count = pages.len();
        let mut total_chars: usize = 0;
        let mut pages_native: usize = 0;
        let mut pages_ocr: usize = 0;
        let mut first_page_text = String::new();

        for page in &pages {
            if cancel.is_cancelled().await {
                return Err(ExtractTextError::Cancelled);
            }

            let non_ws = page.text.chars().filter(|c| !c.is_whitespace()).count();
            let text_to_store = if non_ws < cfg.char_threshold {
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

            if page.page_number == 1 {
                first_page_text = text_to_store.clone();
            }
            total_chars += text_to_store.len();

            pipeline_repository::insert_document_text(
                db,
                doc_id,
                page.page_number as i32,
                &text_to_store,
            )
            .await
            .map_err(|e| ExtractTextError::DbWrite {
                message: format!("insert_document_text page {}: {e}", page.page_number),
            })?;
        }

        // [6b] Fail fast if ALL pages have zero usable text. This means either
        //      (a) the document is entirely scanned and OCR failed/was unavailable,
        //      or (b) the PDF has no extractable content. Continuing would waste
        //      LLM API spend on empty input and produce a "complete" document
        //      with zero entities. The failure surfaces in pipeline_steps.error_message.
        if total_chars == 0 {
            return Err(ExtractTextError::NoUsableText {
                doc_id: doc_id.to_string(),
                page_count,
                ocr_available,
            });
        }

        // [7] Auto-detect document type when current type is "auto" / "unknown".
        let detected_type = detect_document_type(&first_page_text);
        if document.document_type == "auto" || document.document_type == "unknown" {
            sqlx::query(
                "UPDATE documents SET document_type = $1, updated_at = NOW() WHERE id = $2",
            )
            .bind(detected_type)
            .bind(doc_id)
            .execute(db)
            .await
            .map_err(|e| ExtractTextError::DbWrite {
                message: format!("update documents.document_type: {e}"),
            })?;

            let detected_schema = schema_for_document_type(detected_type);
            sqlx::query("UPDATE pipeline_config SET schema_file = $1 WHERE document_id = $2")
                .bind(detected_schema)
                .bind(doc_id)
                .execute(db)
                .await
                .map_err(|e| ExtractTextError::DbWrite {
                    message: format!("update pipeline_config.schema_file: {e}"),
                })?;

            tracing::info!(
                doc_id = %doc_id, detected_type, schema = detected_schema,
                "ExtractText: auto-detected type"
            );
        }

        // [8] Progress reporting.
        let summary = serde_json::json!({
            "page_count": page_count,
            "total_chars": total_chars,
            "pages_native": pages_native,
            "pages_ocr": pages_ocr,
            "detected_type": detected_type,
        });
        if let Err(e) = progress.report(summary).await {
            tracing::warn!(
                doc_id = %doc_id, error = %e,
                "ExtractText: progress.report failed (non-fatal)"
            );
        }

        tracing::info!(
            doc_id = %doc_id, page_count, total_chars, pages_native, pages_ocr,
            "ExtractText complete"
        );

        // [9] Advance to LlmExtract.
        Ok(StepResult::Next(DocProcessing::LlmExtract(LlmExtract {
            document_id: self.document_id.clone(),
        })))
    }
}

// ─────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── OcrConfig ─────────────────────────────────────────────────

    #[test]
    fn ocr_config_default_values() {
        assert_eq!(
            OcrConfig::default(),
            OcrConfig {
                ocr_engine: "surya".to_string(),
                char_threshold: 50,
                dpi: 300,
                lang: "eng".to_string(),
                oem: 1,
            }
        );
    }

    #[test]
    fn ocr_config_is_debug_clone_eq() {
        let a = OcrConfig::default();
        let b = a.clone();
        assert_eq!(a, b);
        let _ = format!("{a:?}");
    }

    // ── ExtractTextError ──────────────────────────────────────────

    #[test]
    fn extract_text_error_document_not_found_display() {
        let e = ExtractTextError::DocumentNotFound {
            doc_id: "doc-x".to_string(),
        };
        assert_eq!(format!("{e}"), "document not found: doc-x");
    }

    #[test]
    fn extract_text_error_ocr_tools_missing_source_chain() {
        use std::error::Error as _;
        let e = ExtractTextError::OcrToolsMissing {
            source: OcrError::ToolNotFound("bar".to_string()),
        };
        assert_eq!(format!("{e}"), "OCR tools unavailable");
        let src = e.source().expect("OcrToolsMissing must expose a source");
        assert!(
            format!("{src}").contains("bar"),
            "source Display must contain the inner token; got: {src}"
        );
    }

    // ── Step constants ────────────────────────────────────────────

    // Compile-time assertions: if any of these constants drift from spec,
    // the crate stops compiling.
    const _: () = {
        assert!(<ExtractText as Step<DocProcessing>>::DEFAULT_RETRY_LIMIT == 2);
        assert!(<ExtractText as Step<DocProcessing>>::DEFAULT_RETRY_DELAY_SECS == 5);
    };

    #[test]
    fn timeout_is_180() {
        assert_eq!(
            <ExtractText as Step<DocProcessing>>::DEFAULT_TIMEOUT_SECS,
            Some(180)
        );
    }

    // ── Visibility / wiring guards ────────────────────────────────

    /// Compile-time visibility guard: the pipeline step module must be able
    /// to call `ocr::ocr_page_with_config` with the expected argument types.
    /// If `ocr_page_with_config` ever loses `pub` or changes its signature,
    /// this fails to compile.
    #[allow(dead_code)]
    fn _ocr_page_with_config_signature_check(p: &str, n: u32, c: &OcrConfig) {
        // The call itself is the assertion — no .await, no runtime execution;
        // the resulting future is immediately dropped, so no subprocesses run.
        let _fut = ocr::ocr_page_with_config(p, n, c);
    }

    /// Confirms the visibility bump on
    /// `api::pipeline::extract_text::detect_document_type` from module-private
    /// to `pub(crate)`. If the bump is ever reverted, this stops compiling.
    #[test]
    fn detect_type_affidavit() {
        assert_eq!(
            detect_document_type("AFFIDAVIT OF JOHN SMITH"),
            "affidavit"
        );
    }
}
