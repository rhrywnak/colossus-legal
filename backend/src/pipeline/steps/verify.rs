//! Verify step: runs PageGrounder verification on extraction items.
//!
//! The logic is the pipeline-framework adaptation of
//! `api::pipeline::verify::run_verify` — same categorization + grounding
//! algorithm, but drives from `&PgPool` + `&AppContext` rather than
//! `&AppState`. The pure helpers (categorization, schema loading,
//! PDF grounding) live in `api::pipeline::verify` and are reused from
//! there; only the snippet-assembly and result-distribution loops are
//! written here, as they are iteration/wiring rather than domain logic.

use std::collections::HashMap;
use std::error::Error;
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use colossus_extract::GroundingMode;
use colossus_pipeline::cancel::CancellationToken;
use colossus_pipeline::progress::ProgressReporter;
use colossus_pipeline::{Step, StepResult};

use crate::api::pipeline::verify as verify_api;
use crate::pipeline::context::AppContext;
use crate::pipeline::steps::auto_approve::AutoApprove;
use crate::pipeline::task::DocProcessing;
use crate::repositories::pipeline_repository::{self, documents};

/// Verify step state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Verify {
    pub document_id: String,
}

// ─────────────────────────────────────────────────────────────────────────
// VerifyError
// ─────────────────────────────────────────────────────────────────────────

/// Failure modes for the Verify step.
///
/// Display strings omit `{source}` (Kazlauskas Guideline 6); message-bearing
/// variants stringify the upstream error at the conversion site.
#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("Document '{doc_id}' not found")]
    DocumentNotFound { doc_id: String },

    #[error("PDF not found for document '{doc_id}'")]
    PdfNotFound { doc_id: String, path: String },

    #[error("PDF grounding failed for document '{doc_id}'")]
    GroundingFailed { doc_id: String, message: String },

    #[error("Database operation failed for document '{doc_id}'")]
    Db { doc_id: String, message: String },
}

/// Internal summary returned by `run_verify` and consumed by `execute`.
struct VerifyResult {
    total_items: usize,
    exact: usize,
    normalized: usize,
    not_found: usize,
    derived: usize,
    unverified: usize,
    missing_quote: usize,
    grounding_pct: f64,
}

// ─────────────────────────────────────────────────────────────────────────
// Step impl
// ─────────────────────────────────────────────────────────────────────────

#[async_trait]
impl Step<DocProcessing> for Verify {
    const DEFAULT_RETRY_LIMIT: i32 = 2;
    const DEFAULT_RETRY_DELAY_SECS: u64 = 5;
    const DEFAULT_TIMEOUT_SECS: Option<u64> = Some(180);

    async fn execute(
        self,
        db: &PgPool,
        context: &AppContext,
        cancel: &CancellationToken,
        _progress: &ProgressReporter,
    ) -> Result<StepResult<DocProcessing>, Box<dyn Error + Send + Sync>> {
        let start = Instant::now();

        if cancel.is_cancelled().await {
            return Err("Cancelled before verify".into());
        }

        let result = self.run_verify(db, context).await?;

        if cancel.is_cancelled().await {
            return Err("Cancelled after verify".into());
        }

        // UI progress (non-critical).
        documents::update_processing_progress(
            db,
            &self.document_id,
            "Verify",
            &format!("{:.0}% grounded", result.grounding_pct),
            0,
            0,
            0,
            0,
        )
        .await
        .ok();

        let duration_secs = start.elapsed().as_secs_f64();
        tracing::info!(
            doc_id = %self.document_id,
            duration_secs,
            total_items = result.total_items,
            exact = result.exact,
            normalized = result.normalized,
            not_found = result.not_found,
            derived = result.derived,
            unverified = result.unverified,
            missing_quote = result.missing_quote,
            grounding_pct = result.grounding_pct,
            "Verify step complete"
        );

        Ok(StepResult::Next(DocProcessing::AutoApprove(AutoApprove {
            document_id: self.document_id,
        })))
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Core implementation
// ─────────────────────────────────────────────────────────────────────────

impl Verify {
    async fn run_verify(
        &self,
        db: &PgPool,
        context: &AppContext,
    ) -> Result<VerifyResult, VerifyError> {
        let doc_id = self.document_id.as_str();

        // 1. Fetch document.
        let document = pipeline_repository::get_document(db, doc_id)
            .await
            .map_err(|e| VerifyError::Db {
                doc_id: doc_id.to_string(),
                message: format!("get_document: {e}"),
            })?
            .ok_or_else(|| VerifyError::DocumentNotFound {
                doc_id: doc_id.to_string(),
            })?;

        // 2. Load grounding modes. Empty-on-failure fallback is the one
        //    place in this step where a graceful degradation is intentional,
        //    mirroring the HTTP handler's behavior for backward compatibility.
        let grounding_modes: HashMap<String, GroundingMode> =
            verify_api::load_grounding_modes(db, &context.schema_dir, doc_id).await;

        // 3. Fetch all items.
        let items = pipeline_repository::get_all_items(db, doc_id)
            .await
            .map_err(|e| VerifyError::Db {
                doc_id: doc_id.to_string(),
                message: format!("get_all_items: {e}"),
            })?;

        // 4. Build PDF path.
        let full_path = format!(
            "{}/{}",
            context.document_storage_path.trim_end_matches('/'),
            document.file_path
        );

        // 5. Verify the PDF file exists on disk.
        if !tokio::fs::try_exists(&full_path).await.unwrap_or(false) {
            return Err(VerifyError::PdfNotFound {
                doc_id: doc_id.to_string(),
                path: document.file_path.clone(),
            });
        }

        // 6. Categorize items by grounding mode.
        let categorization = verify_api::categorize_items_for_grounding(&items, &grounding_modes);

        // 7. Flatten verbatim/name/heading categories into parallel
        //    `snippets` and `snippet_items` vectors for PageGrounder.
        let mut snippets: Vec<String> = Vec::new();
        let mut snippet_items: Vec<verify_api::SnippetMeta> = Vec::new();
        for (item_id, quote) in &categorization.verbatim_items {
            snippets.push(quote.clone());
            snippet_items.push(verify_api::SnippetMeta {
                item_id: *item_id,
                kind: verify_api::SnippetKind::Verbatim,
            });
        }
        for (item_id, name) in &categorization.name_match_items {
            snippets.push(name.clone());
            snippet_items.push(verify_api::SnippetMeta {
                item_id: *item_id,
                kind: verify_api::SnippetKind::NameMatch,
            });
        }
        for (item_id, heading) in &categorization.heading_match_items {
            snippets.push(heading.clone());
            snippet_items.push(verify_api::SnippetMeta {
                item_id: *item_id,
                kind: verify_api::SnippetKind::HeadingMatch,
            });
        }

        // 8. Run PageGrounder on a blocking worker thread.
        let pdf_path = full_path.clone();
        let snippets_for_blocking = snippets.clone();
        let grounding_results = tokio::task::spawn_blocking(move || {
            verify_api::run_grounding(&pdf_path, &snippets_for_blocking)
        })
        .await
        .map_err(|e| VerifyError::GroundingFailed {
            doc_id: doc_id.to_string(),
            message: format!("grounding task panicked: {e}"),
        })?
        .map_err(|e| VerifyError::GroundingFailed {
            doc_id: doc_id.to_string(),
            message: format!("{e:?}"),
        })?;

        // 9. Distribute grounding results and update each item's status.
        let (mut exact, mut normalized, mut not_found) = (0usize, 0usize, 0usize);
        for (i, result) in grounding_results.iter().enumerate() {
            let meta = &snippet_items[i];
            let (status_str, page) = match result.match_type {
                colossus_pdf::MatchType::Exact => {
                    exact += 1;
                    ("exact", result.page_number.map(|p| p as i32))
                }
                colossus_pdf::MatchType::Normalized => {
                    normalized += 1;
                    ("normalized", result.page_number.map(|p| p as i32))
                }
                colossus_pdf::MatchType::NotFound => {
                    not_found += 1;
                    ("not_found", None)
                }
            };
            pipeline_repository::update_item_grounding(db, meta.item_id, status_str, page)
                .await
                .map_err(|e| VerifyError::Db {
                    doc_id: doc_id.to_string(),
                    message: format!("update_item_grounding (item {}): {e}", meta.item_id),
                })?;
        }

        // 10. Mark Derived-mode items (provenance-based, no PDF search).
        for item_id in &categorization.derived_item_ids {
            pipeline_repository::update_item_grounding(db, *item_id, "derived", None)
                .await
                .map_err(|e| VerifyError::Db {
                    doc_id: doc_id.to_string(),
                    message: format!("update_item_grounding derived (item {item_id}): {e}"),
                })?;
        }

        // 11. Mark None-mode items (no grounding required).
        for item_id in &categorization.none_item_ids {
            pipeline_repository::update_item_grounding(db, *item_id, "unverified", None)
                .await
                .map_err(|e| VerifyError::Db {
                    doc_id: doc_id.to_string(),
                    message: format!("update_item_grounding unverified (item {item_id}): {e}"),
                })?;
        }

        // 12. Mark items that should have had a snippet but didn't.
        if !categorization.missing_quote_item_ids.is_empty() {
            tracing::warn!(
                doc_id = %doc_id,
                count = categorization.missing_quote_item_ids.len(),
                "Items missing required grounding snippet"
            );
        }
        for item_id in &categorization.missing_quote_item_ids {
            pipeline_repository::update_item_grounding(db, *item_id, "missing_quote", None)
                .await
                .map_err(|e| VerifyError::Db {
                    doc_id: doc_id.to_string(),
                    message: format!("update_item_grounding missing_quote (item {item_id}): {e}"),
                })?;
        }

        let total_items = items.len();
        let grounding_pct = if total_items > 0 {
            ((exact + normalized) as f64 / total_items as f64 * 100.0).round()
        } else {
            0.0
        };

        Ok(VerifyResult {
            total_items,
            exact,
            normalized,
            not_found,
            derived: categorization.derived_item_ids.len(),
            unverified: categorization.none_item_ids.len(),
            missing_quote: categorization.missing_quote_item_ids.len(),
            grounding_pct,
        })
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_step_constants_match_spec() {
        assert_eq!(Verify::DEFAULT_RETRY_LIMIT, 2);
        assert_eq!(Verify::DEFAULT_RETRY_DELAY_SECS, 5);
        assert_eq!(Verify::DEFAULT_TIMEOUT_SECS, Some(180));
    }

    #[test]
    fn verify_error_document_not_found_display_contains_doc_id() {
        let err = VerifyError::DocumentNotFound {
            doc_id: "missing-99".to_string(),
        };
        assert!(format!("{err}").contains("missing-99"));
    }

    #[test]
    fn verify_error_pdf_not_found_display_excludes_path_body() {
        // G6: path is a struct field but not interpolated into Display.
        let err = VerifyError::PdfNotFound {
            doc_id: "doc-1".to_string(),
            path: "UNIQUE_PATH_TOKEN/file.pdf".to_string(),
        };
        let display = format!("{err}");
        assert!(display.contains("doc-1"), "got: {display}");
        assert!(
            !display.contains("UNIQUE_PATH_TOKEN"),
            "Display must not interpolate non-source fields that were not named in the format string; got: {display}"
        );
    }
}
