//! Verify step: runs canonical text verification on extraction items.
//!
//! Searches the document's stored text (`document_text` table) for each
//! extraction item's grounding snippet. Format-agnostic: text PDFs,
//! scanned PDFs (OCR), and future formats all verify against the same
//! canonical text the LLM saw during extraction.
//!
//! The categorization and schema-loading helpers live in
//! `api::pipeline::verify` and are reused from there. The canonical text
//! search logic lives in `api::pipeline::canonical_verifier`.

use std::collections::HashMap;
use std::error::Error;
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::api::pipeline::verify::{
    build_para_to_item_id, validate_derived_provenance, DerivedValidation, EntityVerificationConfig,
};
use colossus_pipeline::cancel::CancellationToken;
use colossus_pipeline::progress::ProgressReporter;
use colossus_pipeline::{Step, StepResult};

use crate::api::pipeline::canonical_verifier::{find_in_canonical_text, CanonicalMatchType};
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

    /// Retained for the Display-hygiene regression test
    /// (`verify_error_pdf_not_found_display_excludes_path_body`),
    /// which asserts that `path` is a struct field but never interpolated
    /// into the Display output (Kazlauskas Guideline 6). The canonical-text
    /// flow no longer constructs this variant.
    #[allow(dead_code)]
    #[error("PDF not found for document '{doc_id}'")]
    PdfNotFound { doc_id: String, path: String },

    #[error("No canonical text found for document '{doc_id}' — was ExtractText run?")]
    NoCanonicalText { doc_id: String },

    #[error("Failed to load grounding modes for document '{doc_id}': {message}")]
    GroundingModes { doc_id: String, message: String },

    #[error("Database operation failed for document '{doc_id}'")]
    Db { doc_id: String, message: String },
}

/// Outcome of a successful pass through [`run_verify`].
///
/// Consumed by:
/// - The legacy [`Verify::execute`] thin wrapper — re-emits the
///   11-key audit JSON via `progress.set_step_result(...)` (the two
///   derived keys `grounded` and `ungrounded` are computed at
///   JSON-build time from `exact + normalized` and
///   `not_found + missing_quote`).
/// - The Restate workflow handler (`step_verify`) — builds a journal
///   summary string from these counters.
#[derive(Debug, Clone, Default)]
pub struct VerifyResult {
    pub total_items: usize,
    pub exact: usize,
    pub normalized: usize,
    pub not_found: usize,
    /// Derived-mode items whose v5.1 §5.4 provenance validation passed.
    pub derived: usize,
    /// Derived-mode items that failed v5.1 §5.4 provenance validation
    /// (`grounding_status='derived_invalid'`, with diagnostic in
    /// `verification_reason`).
    pub derived_invalid: usize,
    pub unverified: usize,
    pub missing_quote: usize,
    pub grounding_pct: f64,
}

// ─────────────────────────────────────────────────────────────────────────
// Step impl
// ─────────────────────────────────────────────────────────────────────────

#[async_trait]
impl Step<DocProcessing> for Verify {
    const DEFAULT_RETRY_LIMIT: i32 = 2;
    const DEFAULT_RETRY_DELAY_SECS: u64 = 5;
    const DEFAULT_TIMEOUT_SECS: Option<u64> = Some(180);

    /// Thin wrapper over [`run_verify`] — the clean business core
    /// that the Restate workflow handler also calls.
    ///
    /// Adds on top of the core:
    /// 1. **Pre / post cancel checks.** The legacy worker wraps the
    ///    step body in `tokio::select!` with a `cancel_watcher`;
    ///    these explicit checks short-circuit deterministic boundaries.
    /// 2. **`progress.set_step_result(...)` audit JSON.** Re-emits
    ///    the 11-key shape the pre-refactor body wrote inline so
    ///    `pipeline_steps.result_summary` stays byte-identical.
    /// 3. **FSM routing.** Pass-2 always hands off to AutoApprove;
    ///    Restate sequences steps directly.
    ///
    /// The UI progress write (`update_processing_progress`) now
    /// lives INSIDE `run_verify` so both the legacy and Restate
    /// paths surface the post-verify grounding percentage to the
    /// Documents-tab poll loop.
    async fn execute(
        self,
        db: &PgPool,
        context: &AppContext,
        cancel: &CancellationToken,
        progress: &ProgressReporter,
    ) -> Result<StepResult<DocProcessing>, Box<dyn Error + Send + Sync>> {
        let start = Instant::now();

        if cancel.is_cancelled().await {
            return Err("Cancelled before verify".into());
        }

        let result = run_verify(&self.document_id, db, context).await?;

        if cancel.is_cancelled().await {
            return Err("Cancelled after verify".into());
        }

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

        progress.set_step_result(serde_json::json!({
            "grounded": result.exact + result.normalized,
            "ungrounded": result.not_found + result.missing_quote,
            "total": result.total_items,
            "exact": result.exact,
            "normalized": result.normalized,
            "not_found": result.not_found,
            "derived": result.derived,
            "derived_invalid": result.derived_invalid,
            "unverified": result.unverified,
            "missing_quote": result.missing_quote,
            "grounding_pct": result.grounding_pct,
        }));

        Ok(StepResult::Next(DocProcessing::AutoApprove(AutoApprove {
            document_id: self.document_id,
        })))
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Core implementation (Restate-callable)
// ─────────────────────────────────────────────────────────────────────────

/// Run the Verify step — canonical-text grounding verification.
///
/// Searches the document's stored canonical text (the `document_text`
/// table populated by ExtractText) for each extraction item's
/// grounding snippet, classifies the match (`exact` / `normalized` /
/// `not_found` / `derived` / `derived_invalid` / `unverified` /
/// `missing_quote`), and writes the result to each item's
/// `grounding_status` column.
///
/// ## Idempotency
///
/// No explicit short-circuit guard. The per-item
/// `update_item_grounding` writes are idempotent at the DB level — a
/// second invocation re-checks every item and writes the same value
/// it wrote the first time. Restate replay is safe; the cost is a
/// redundant linear sweep over items + canonical-text search.
///
/// ## Cancellation
///
/// Does not poll a `CancellationToken`. The legacy worker wraps the
/// `Step::execute` wrapper in `tokio::select!` with a
/// `cancel_watcher`; the Restate path kills the awaiting future via
/// SDK abort. Either way the function aborts at the next `.await`.
pub async fn run_verify(
    document_id: &str,
    db: &PgPool,
    context: &AppContext,
) -> Result<VerifyResult, VerifyError> {
    let doc_id = document_id;

    // 1. Fetch document (existence guard — content is no longer read here;
    //    canonical text comes from document_text in step 4).
    let _document = pipeline_repository::get_document(db, doc_id)
        .await
        .map_err(|e| VerifyError::Db {
            doc_id: doc_id.to_string(),
            message: format!("get_document: {e}"),
        })?
        .ok_or_else(|| VerifyError::DocumentNotFound {
            doc_id: doc_id.to_string(),
        })?;

    // 2. Load grounding config (mode + provenance_required per
    //    entity_type). Failure is fatal: without the schema we'd
    //    default every entity to Verbatim, silently corrupting
    //    Party / LegalCount / Harm grounding.
    let grounding_config: HashMap<String, EntityVerificationConfig> =
        verify_api::load_grounding_config(db, context.registry.schema_dir(), doc_id)
            .await
            .map_err(|message| VerifyError::GroundingModes {
                doc_id: doc_id.to_string(),
                message,
            })?;

    // 3. Fetch all items.
    let items = pipeline_repository::get_all_items(db, doc_id)
        .await
        .map_err(|e| VerifyError::Db {
            doc_id: doc_id.to_string(),
            message: format!("get_all_items: {e}"),
        })?;

    // 4. Load canonical text from document_text table.
    let document_text_rows = pipeline_repository::get_document_text(db, doc_id)
        .await
        .map_err(|e| VerifyError::Db {
            doc_id: doc_id.to_string(),
            message: format!("get_document_text: {e}"),
        })?;

    if document_text_rows.is_empty() {
        return Err(VerifyError::NoCanonicalText {
            doc_id: doc_id.to_string(),
        });
    }

    // 5. Convert to (page_number, text_content) tuples for the verifier.
    let document_pages: Vec<(u32, String)> = document_text_rows
        .into_iter()
        .map(|row| (row.page_number as u32, row.text_content))
        .collect();

    // 6. Categorize items by grounding mode.
    let categorization = verify_api::categorize_items_for_grounding(&items, &grounding_config);

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

    // 8. Search each snippet against canonical text and update DB.
    let (mut exact, mut normalized, mut not_found) = (0usize, 0usize, 0usize);
    for (i, snippet) in snippets.iter().enumerate() {
        let meta = &snippet_items[i];
        let result = find_in_canonical_text(snippet, &document_pages);

        let (status_str, page) = match result.match_type {
            CanonicalMatchType::Exact => {
                exact += 1;
                ("exact", result.page_number.map(|p| p as i32))
            }
            CanonicalMatchType::Normalized => {
                normalized += 1;
                ("normalized", result.page_number.map(|p| p as i32))
            }
            CanonicalMatchType::NotFound => {
                not_found += 1;
                ("not_found", None)
            }
        };
        pipeline_repository::update_item_grounding(db, meta.item_id, status_str, page, None)
            .await
            .map_err(|e| VerifyError::Db {
                doc_id: doc_id.to_string(),
                message: format!("update_item_grounding (item {}): {e}", meta.item_id),
            })?;
    }

    // 10. Validate Derived-mode items per v5.1 §5.4. Mirrors the
    //     api-side path in `api::pipeline::verify::run_verify`. The
    //     two ingest entry points must produce identical state, so
    //     the validation logic is shared via `validate_derived_provenance`.
    let para_to_item_id = build_para_to_item_id(&items);
    let mut derived_invalid_count = 0usize;
    let mut derived_valid_count = 0usize;
    for item_id in &categorization.derived_item_ids {
        let item = items
            .iter()
            .find(|i| i.id == *item_id)
            .expect("derived_item_ids only contains ids drawn from items");
        let provenance_required = grounding_config
            .get(&item.entity_type)
            .map(|c| c.provenance_required)
            .unwrap_or(false);
        let validation = validate_derived_provenance(item, &para_to_item_id, provenance_required);
        let (status_str, reason) = match validation {
            DerivedValidation::Valid => {
                derived_valid_count += 1;
                ("derived", None)
            }
            DerivedValidation::Invalid(r) => {
                derived_invalid_count += 1;
                ("derived_invalid", Some(r))
            }
        };
        pipeline_repository::update_item_grounding(
            db,
            *item_id,
            status_str,
            None,
            reason.as_deref(),
        )
        .await
        .map_err(|e| VerifyError::Db {
            doc_id: doc_id.to_string(),
            message: format!("update_item_grounding derived (item {item_id}): {e}"),
        })?;
    }

    // 11. Mark None-mode items (no grounding required).
    for item_id in &categorization.none_item_ids {
        pipeline_repository::update_item_grounding(db, *item_id, "unverified", None, None)
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
        pipeline_repository::update_item_grounding(db, *item_id, "missing_quote", None, None)
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

    // best-effort: progress update. Surfaces the post-verify
    // grounding percentage to the Documents-tab poll loop. Lives
    // here (not in the legacy wrapper) so both legacy and Restate
    // paths write the UI progress columns. `.ok()` discards the
    // sqlx::Error — a failed progress write must never fail the
    // verify step.
    documents::update_processing_progress(
        db,
        doc_id,
        "Verify",
        &format!("{grounding_pct:.0}% grounded"),
        0,
        0,
        0,
        0,
    )
    .await
    .ok();

    Ok(VerifyResult {
        total_items,
        exact,
        normalized,
        not_found,
        // `derived` here is post-validation valid count, not the
        // bucket size. Pre-v5.1 this would have read
        // `categorization.derived_item_ids.len()` (every item in
        // the bucket). Now invalid items are split out into
        // `derived_invalid` and counted separately.
        derived: derived_valid_count,
        derived_invalid: derived_invalid_count,
        unverified: categorization.none_item_ids.len(),
        missing_quote: categorization.missing_quote_item_ids.len(),
        grounding_pct,
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
