//! Restate workflow step: extract text from the uploaded document.
//!
//! Wraps the shared
//! [`extract_text_to_db`](crate::pipeline::steps::extract_text::extract_text_to_db)
//! helper with the idempotency guard, Postgres status write, and
//! Restate error classification specific to the workflow path.
//!
//! ## Idempotency
//!
//! On every invocation we first call
//! `pipeline_repository::get_document_text` for the doc id. If it
//! returns a non-empty vector, the extraction has already been run for
//! this document and we short-circuit without touching the file or the
//! OCR service. Restate replays the `ctx.run` closure on workflow
//! recovery, so this short-circuit is what makes the second (and Nth)
//! invocation cheap.
//!
//! ## Error classification (Restate semantics)
//!
//! Restate distinguishes:
//!
//! - **Retryable** errors — Restate retries the step with exponential
//!   backoff. Any `Result<_, E>` whose `E` converts into a regular
//!   [`HandlerError`] is treated as retryable. We use this for
//!   transient failures like database timeouts.
//! - **Terminal** errors — Restate stops retrying and fails the
//!   workflow. Wrap the message in [`TerminalError::new(...)`] and
//!   `.into()` it into a `HandlerError`. We use this for permanent
//!   failures (document not found, file not on disk, an empty
//!   document) where retrying would never succeed.
//!
//! The [`classify_extract_error`] helper inspects an
//! [`ExtractTextError`] variant and decides which side to send it on.

use std::sync::Arc;

use restate_sdk::errors::{HandlerError, TerminalError};

use super::{record_step_lifecycle, StepOutcome, STEP_EXTRACT_TEXT};
use crate::models::document_status::STATUS_TEXT_EXTRACTED;
use crate::pipeline::context::AppContext;
use crate::pipeline::steps::extract_text::{extract_text_to_db, ExtractTextError};
use crate::repositories::pipeline_repository;

/// Restate workflow step: extract text from the uploaded document.
///
/// Performs:
/// 1. Idempotency check — if `document_text` already has rows for
///    `doc_id`, return early with a `skipped_...` summary.
/// 2. The shared extraction path
///    ([`extract_text_to_db`](crate::pipeline::steps::extract_text::extract_text_to_db)).
/// 3. Update `documents.status` to `"TEXT_EXTRACTED"` (the canonical
///    uppercase value from [`crate::models::document_status`]).
///
/// Returns a short summary string suitable for journaling. Restate
/// captures the return value into its workflow journal on first run;
/// replay reads the journaled value back without re-invoking the
/// closure, so the body must be deterministic and the return value
/// must be small.
///
/// ## Rust Learning: `Arc<AppContext>` vs `&AppContext`
///
/// The Restate `ctx.run` closure is `'static` and `Send` — it must
/// own everything it captures. The workflow handler holds an
/// `Arc<AppContext>`; cloning the `Arc` before moving it into the
/// closure is cheap (one atomic increment) and lets the helper see
/// the full context. The signature here takes `&Arc<AppContext>`
/// (rather than `&AppContext`) so the caller can keep the original
/// `Arc` for later steps without re-cloning it. Inside this function
/// we deref to `&AppContext` (`app.as_ref()`) when calling helpers
/// that don't need the refcount.
#[tracing::instrument(skip(app), fields(doc_id = %doc_id, step = STEP_EXTRACT_TEXT))]
pub async fn step_extract_text(
    app: &Arc<AppContext>,
    doc_id: &str,
) -> Result<String, HandlerError> {
    // Audit-row lifecycle: one `pipeline_steps` row per invocation.
    // `record_step_lifecycle` handles the start/finish/failure writes
    // around the body future and surfaces the body's `HandlerError`
    // unchanged on the failure path (Restate's retry behaviour stays
    // driven by the body's `classify_*_error`).
    record_step_lifecycle(
        &app.pipeline_pool,
        doc_id,
        STEP_EXTRACT_TEXT,
        step_extract_text_body(app, doc_id),
    )
    .await
}

/// Body of [`step_extract_text`] — the original step logic, returning
/// [`StepOutcome`] so the wrapping handler can record the
/// `pipeline_steps` row with the right `result_summary` shape.
///
/// Returns three distinct outcomes via [`StepOutcome`]:
///
/// - **Skipped** (idempotency guard fired): `skipped_early = true`,
///   `result_summary` is `{"skipped": true, "reason": "already_extracted",
///   "page_count": N}` — distinct from the success shape so an
///   operator inspecting the row can tell the two cases apart.
/// - **Success**: `skipped_early = false`, `result_summary` is the
///   6-key shape the legacy `progress.set_step_result(...)` emits
///   (`page_count`, `total_chars`, `pages_native`, `pages_ocr`,
///   `detected_type`, `ocr_engine`).
/// - **Failure**: returns `Err(HandlerError)` — the wrapper records
///   the failure row.
#[tracing::instrument(skip(app), fields(doc_id = %doc_id))]
async fn step_extract_text_body(
    app: &Arc<AppContext>,
    doc_id: &str,
) -> Result<StepOutcome, HandlerError> {
    // [1] Idempotency guard. `document_text` is the authoritative
    //     "extraction already done" signal — a non-empty vector here
    //     means a prior invocation wrote pages, so we skip without
    //     touching the file or the OCR service.
    let existing = pipeline_repository::get_document_text(&app.pipeline_pool, doc_id)
        .await
        .map_err(|e| {
            // DB read failure here is transient (network blip, pool
            // exhaustion). Let Restate retry — surface as a regular
            // (non-terminal) HandlerError. The message includes the
            // doc_id so a single log line points an operator at the
            // affected document directly.
            HandlerError::from(format!(
                "step_extract_text: failed to read document_text for '{doc_id}' \
                 during idempotency check: {e}. Will retry."
            ))
        })?;

    if !existing.is_empty() {
        let page_count = existing.len();
        tracing::info!(
            doc_id = %doc_id,
            page_count,
            "step_extract_text: skip — already extracted"
        );
        // Distinct shape from the success path — see
        // [`build_skipped_result_summary`] for the audit-trail
        // rationale and contract.
        return Ok(StepOutcome {
            summary: format!("skipped_already_extracted_{page_count}_pages"),
            result_summary: build_skipped_result_summary(page_count),
            skipped_early: true,
        });
    }

    // [2] Delegate to the shared helper. The legacy Worker step and
    //     this Restate handler share the body so the extraction logic
    //     stays single-source.
    let result = extract_text_to_db(&app.pipeline_pool, app.as_ref(), doc_id)
        .await
        .map_err(|e| classify_extract_error(doc_id, e))?;

    // [3] Postgres status write. Mirrors the Restate state write the
    //     calling workflow performs (`ctx.set(STATUS_STATE_KEY, ...)`)
    //     so the documents tab and the Restate journal agree on the
    //     terminal-per-step status. NOT idempotent at the rows-affected
    //     level — `update_document_status` returns NotFound if the
    //     document row has been deleted between steps. We treat that
    //     as terminal (the document is gone; retrying won't bring it
    //     back).
    pipeline_repository::update_document_status(&app.pipeline_pool, doc_id, STATUS_TEXT_EXTRACTED)
        .await
        .map_err(|e| match e {
            pipeline_repository::PipelineRepoError::NotFound(_) => TerminalError::new(format!(
                "step_extract_text: documents row for '{doc_id}' \
             disappeared while updating status. Cannot proceed; \
             confirm the document still exists in the documents table."
            ))
            .into(),
            other => HandlerError::from(format!(
                "step_extract_text: failed to update status for '{doc_id}': {other}. \
             Will retry."
            )),
        })?;

    let summary = format!(
        "extracted_{}_pages_{}_chars",
        result.page_count, result.total_chars
    );
    tracing::info!(
        doc_id = %doc_id,
        page_count = result.page_count,
        total_chars = result.total_chars,
        pages_native = result.pages_native,
        pages_ocr = result.pages_ocr,
        detected_type = ?result.detected_document_type,
        "step_extract_text: complete"
    );
    // Audit JSON shape matches `pipeline/steps/extract_text.rs:397`
    // (the legacy `progress.set_step_result(...)` call) byte-for-byte
    // so external audit tooling sees the same column content from both
    // paths. See [`build_success_result_summary`].
    Ok(StepOutcome {
        summary,
        result_summary: build_success_result_summary(&result),
        skipped_early: false,
    })
}

/// Build the 3-key skipped-path `result_summary` JSON written when
/// the idempotency guard found existing `document_text` rows.
///
/// The `"skipped": true` sentinel lets audit tooling and the
/// Execution History panel distinguish "we did work" from "we
/// short-circuited" without parsing the summary string. Extracted
/// from the inline call site so the contract can be unit-tested
/// without standing up a database.
fn build_skipped_result_summary(page_count: usize) -> serde_json::Value {
    serde_json::json!({
        "skipped": true,
        "reason": "already_extracted",
        "page_count": page_count,
    })
}

/// Build the 6-key success-path `result_summary` JSON, matching
/// `pipeline/steps/extract_text.rs:397` byte-for-byte.
///
/// Extracted from the inline call site so the audit-trail contract
/// (specifically the `detected_type` rename from
/// `ExtractTextOutcome::detected_document_type`) can be pinned by a
/// unit test against silent breakage from a future struct-field
/// rename.
fn build_success_result_summary(
    result: &crate::pipeline::steps::extract_text::TextExtractionResult,
) -> serde_json::Value {
    serde_json::json!({
        "page_count": result.page_count,
        "total_chars": result.total_chars,
        "pages_native": result.pages_native,
        "pages_ocr": result.pages_ocr,
        "detected_type": result.detected_document_type,
        "ocr_engine": result.ocr_engine,
    })
}

/// Classify an [`ExtractTextError`] as terminal or retryable for
/// Restate.
///
/// The rule of thumb: anything the *next* retry can't change is
/// terminal. A missing row, a missing file, an empty document — none
/// of those resolve themselves on the next attempt. Anything that
/// could be a transient blip in the infrastructure (DB write timeout,
/// upstream spawn-blocking thread-pool exhaustion) is retryable.
///
/// `OcrToolsMissing` and `ProfileLoad` are deliberately terminal:
/// missing OCR binaries and a missing/malformed profile YAML are both
/// deployment issues that need operator intervention before any retry
/// can succeed. Letting Restate retry these forever would just spam
/// the logs without making progress.
///
/// ## Rust Learning: pattern-matching on enum variants for classification
///
/// `match e { Variant => ..., other => ... }` is the idiomatic Rust
/// way to split an enum into "named variants we know about" and "a
/// catch-all for the rest." Adding a new variant later forces the
/// compiler to surface this match site if you remove the wildcard arm
/// — a useful exhaustiveness check. We keep the wildcard here because
/// the `Cancelled` variant is unreachable on the Restate path (no
/// `CancellationToken` is threaded in) and we'd rather treat any
/// future addition as transient until explicitly classified.
fn classify_extract_error(doc_id: &str, e: ExtractTextError) -> HandlerError {
    match &e {
        ExtractTextError::DocumentNotFound { .. } => TerminalError::new(format!(
            "step_extract_text: document '{doc_id}' not found in database. \
             Confirm the upload completed before invoking the workflow."
        ))
        .into(),
        ExtractTextError::FileNotFound { path } => TerminalError::new(format!(
            "step_extract_text: document file for '{doc_id}' not present at \
             '{path}'. Check DOCUMENT_STORAGE_PATH and documents.file_path."
        ))
        .into(),
        ExtractTextError::NoUsableText { .. } => TerminalError::new(format!(
            "step_extract_text: no usable text extracted from document \
             '{doc_id}'. {e}"
        ))
        .into(),
        ExtractTextError::OcrToolsMissing { .. } => TerminalError::new(format!(
            "step_extract_text: OCR tooling unavailable. {e}. \
             Check pdftoppm and tesseract are installed in the backend image \
             before retrying."
        ))
        .into(),
        ExtractTextError::ProfileLoad { .. } => TerminalError::new(format!(
            "step_extract_text: profile configuration error while extracting \
             '{doc_id}'. {e}. Fix the profile YAML and redeploy before retry."
        ))
        .into(),
        ExtractTextError::ExtractionFailed { .. } | ExtractTextError::DbWrite { .. } => {
            // Transient classes: spawn_blocking failure, DB write error.
            // Retryable — Restate will back off and try again.
            HandlerError::from(format!(
                "step_extract_text: transient failure extracting '{doc_id}'. \
                 {e}. Will retry."
            ))
        }
        ExtractTextError::Cancelled => {
            // Unreachable on the Restate path (no CancellationToken).
            // Treat as terminal — cancellation is operator intent and
            // should not be retried.
            TerminalError::new(format!(
                "step_extract_text: extraction for '{doc_id}' reported \
                 cancelled. This should not occur on the Restate path \
                 (no CancellationToken is threaded). Investigate."
            ))
            .into()
        }
    }
}

// Unit tests for `classify_extract_error` live in
// `extract_text_tests.rs` (kept out-of-line to stay under the
// 300-line module-size budget; matches the
// `pipeline/registry.rs` / `registry_tests.rs` idiom).
#[cfg(test)]
#[path = "extract_text_tests.rs"]
mod tests;
