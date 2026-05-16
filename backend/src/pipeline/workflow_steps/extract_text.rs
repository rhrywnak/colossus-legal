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
#[tracing::instrument(skip(app), fields(doc_id = %doc_id, step = "extract_text"))]
pub async fn step_extract_text(
    app: &Arc<AppContext>,
    doc_id: &str,
) -> Result<String, HandlerError> {
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
        return Ok(format!("skipped_already_extracted_{page_count}_pages"));
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
    Ok(summary)
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

// ─────────────────────────────────────────────────────────────────
// Unit tests for `classify_extract_error`.
//
// The terminal-vs-retryable decision is operator-observable: a
// terminal classification stops Restate's retry loop and fails the
// workflow; a retryable one triggers exponential backoff. A
// misclassification introduced by a future edit has no compile-time
// backstop without these tests — one test per variant pins the
// contract.
//
// `HandlerError`'s inner enum is `pub(crate)` to the restate_sdk
// crate, so we cannot pattern-match on the Terminal/Retryable
// variants directly. We assert through the `Display` impl instead,
// which prefixes "Terminal error" or "Retryable error" depending on
// the inner variant (see restate_sdk::errors::HandlerErrorInner's
// Display impl, restate-sdk-0.6/src/errors.rs:29-38).
// ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    use crate::pipeline::config::ProcessingProfileLoadError;
    use crate::pipeline::steps::extract_text::ExtractTextError;

    /// Returns `true` when `e` is the Terminal branch of HandlerError.
    ///
    /// `HandlerError` itself does not implement `Display` (only its
    /// `pub(crate)` inner enum does), so we route through `as_ref()`
    /// — `HandlerError: AsRef<dyn StdError>`, and every `StdError`
    /// implements `Display`. The inner `HandlerErrorInner::Display`
    /// formats terminal errors as `"Terminal error [code]: message"`
    /// and retryable ones as `"Retryable error: ..."`. We pin the
    /// classification by checking the prefix.
    fn display_message(e: &HandlerError) -> String {
        let inner: &dyn std::error::Error = e.as_ref();
        format!("{inner}")
    }

    fn is_terminal(e: &HandlerError) -> bool {
        display_message(e).starts_with("Terminal error")
    }

    #[test]
    fn classify_document_not_found_is_terminal() {
        let err = ExtractTextError::DocumentNotFound {
            doc_id: "doc-abc".into(),
        };
        let classified = classify_extract_error("doc-abc", err);
        assert!(
            is_terminal(&classified),
            "DocumentNotFound must classify as terminal — retrying won't make \
             a missing row reappear. Got: {:?}",
            classified
        );
        // The operator-facing message must name the doc_id and point
        // at the recovery action (confirming upload completed).
        let msg = display_message(&classified);
        assert!(msg.contains("doc-abc"), "msg must name doc_id: {msg}");
        assert!(
            msg.contains("not found"),
            "msg must say what's wrong: {msg}"
        );
    }

    #[test]
    fn classify_file_not_found_is_terminal() {
        let err = ExtractTextError::FileNotFound {
            path: "/data/docs/missing.pdf".into(),
        };
        let classified = classify_extract_error("doc-x", err);
        assert!(
            is_terminal(&classified),
            "FileNotFound must classify as terminal — retrying won't put the \
             file back on disk. Got: {:?}",
            classified
        );
        let msg = display_message(&classified);
        assert!(
            msg.contains("/data/docs/missing.pdf"),
            "msg must include the path: {msg}"
        );
        assert!(
            msg.contains("DOCUMENT_STORAGE_PATH"),
            "msg must point at the env var to check: {msg}"
        );
    }

    #[test]
    fn classify_no_usable_text_is_terminal() {
        // Construct a NoUsableText with the same fields the real path
        // emits — the Display impl includes the page/OCR counters.
        let err = ExtractTextError::NoUsableText {
            doc_id: "doc-empty".into(),
            page_count: 5,
            pages_native: 5,
            pages_ocr: 0,
            scanned_count: 0,
            ocr_available: true,
            ocr_error_suffix: String::new(),
        };
        let classified = classify_extract_error("doc-empty", err);
        assert!(
            is_terminal(&classified),
            "NoUsableText must classify as terminal — an empty PDF won't \
             gain content on retry. Got: {:?}",
            classified
        );
    }

    #[test]
    fn classify_ocr_tools_missing_is_terminal() {
        let err = ExtractTextError::OcrToolsMissing {
            source: crate::api::pipeline::ocr::OcrError::ToolNotFound(
                "pdftoppm not on PATH".into(),
            ),
        };
        let classified = classify_extract_error("doc-x", err);
        assert!(
            is_terminal(&classified),
            "OcrToolsMissing must classify as terminal — missing binaries \
             are a deployment fix, not a retry. Got: {:?}",
            classified
        );
        let msg = display_message(&classified);
        assert!(
            msg.contains("pdftoppm") || msg.contains("tesseract"),
            "msg must name the tools to install: {msg}"
        );
    }

    #[test]
    fn classify_profile_load_is_terminal() {
        let err = ExtractTextError::ProfileLoad {
            source: ProcessingProfileLoadError::FileNotFound {
                path: "/etc/profiles/missing.yaml".into(),
            },
        };
        let classified = classify_extract_error("doc-x", err);
        assert!(
            is_terminal(&classified),
            "ProfileLoad must classify as terminal — fixing YAML is a \
             deploy step. Got: {:?}",
            classified
        );
        let msg = display_message(&classified);
        assert!(
            msg.contains("profile"),
            "msg must mention the profile: {msg}"
        );
        assert!(
            msg.contains("redeploy"),
            "msg must hint at fix+redeploy: {msg}"
        );
    }

    #[test]
    fn classify_extraction_failed_is_retryable() {
        // spawn_blocking failures and other transient PDF/DOCX errors
        // come through as ExtractionFailed. The retry path is correct
        // for these — a thread-pool exhaustion or a flaky native call
        // may resolve on the next attempt.
        let err = ExtractTextError::ExtractionFailed {
            message: "pdf spawn_blocking join: panic".into(),
        };
        let classified = classify_extract_error("doc-x", err);
        assert!(
            !is_terminal(&classified),
            "ExtractionFailed must classify as retryable — a transient PDF \
             extractor crash may succeed on retry. Got: {:?}",
            classified
        );
        let msg = display_message(&classified);
        assert!(
            msg.contains("Will retry"),
            "msg must signal retry intent for operator clarity: {msg}"
        );
    }

    #[test]
    fn classify_db_write_is_retryable() {
        let err = ExtractTextError::DbWrite {
            message: "connection timeout".into(),
        };
        let classified = classify_extract_error("doc-x", err);
        assert!(
            !is_terminal(&classified),
            "DbWrite must classify as retryable — pool/connection blips \
             often resolve on the next attempt. Got: {:?}",
            classified
        );
    }

    #[test]
    fn classify_cancelled_is_terminal() {
        // Cancelled is unreachable on the Restate path (no
        // CancellationToken threaded in), but if it ever surfaces we
        // must NOT retry an operator-initiated cancellation.
        let err = ExtractTextError::Cancelled;
        let classified = classify_extract_error("doc-x", err);
        assert!(
            is_terminal(&classified),
            "Cancelled must classify as terminal — operator intent should \
             never be retried into a different outcome. Got: {:?}",
            classified
        );
        let msg = display_message(&classified);
        assert!(
            msg.contains("Investigate"),
            "msg must flag the unexpected path: {msg}"
        );
    }
}
