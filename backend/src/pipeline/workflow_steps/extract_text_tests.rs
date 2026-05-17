//! Unit tests for [`super::classify_extract_error`].
//!
//! Lives in a sibling file (rather than a `mod tests { ... }` block
//! inside `extract_text.rs`) so the runtime file stays under the
//! 300-line module-size budget. Wired into the runtime module via
//! `#[cfg(test)] #[path = "extract_text_tests.rs"] mod tests;` — the
//! same idiom `pipeline/registry.rs` uses for `registry_tests.rs`.
//!
//! The terminal-vs-retryable decision is operator-observable: a
//! terminal classification stops Restate's retry loop and fails the
//! workflow; a retryable one triggers exponential backoff. A
//! misclassification introduced by a future edit has no compile-time
//! backstop without these tests — one test per variant pins the
//! contract.
//!
//! `HandlerError`'s inner enum is `pub(crate)` to the restate_sdk
//! crate, so we cannot pattern-match on the Terminal/Retryable
//! variants directly. We assert through the `Display` impl instead,
//! which prefixes "Terminal error" or "Retryable error" depending on
//! the inner variant (see restate_sdk::errors::HandlerErrorInner's
//! Display impl, restate-sdk-0.6/src/errors.rs:29-38).

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
        source: crate::api::pipeline::ocr::OcrError::ToolNotFound("pdftoppm not on PATH".into()),
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

// ── `build_*_result_summary` shape contracts ────────────────────
//
// The two builder functions encode the audit-trail contract the
// Restate path shares with the legacy Worker (the
// `progress.set_step_result(...)` JSON at
// `pipeline/steps/extract_text.rs:397` and the
// `"skipped": true` sentinel design from the P2-PRE-4 instruction).
// A field-name rename on either side without an audit-trail
// migration would silently break tooling that reads
// `pipeline_steps.result_summary` directly.

#[test]
fn build_skipped_result_summary_emits_three_keys() {
    let summary = super::build_skipped_result_summary(42);
    assert_eq!(summary["skipped"], serde_json::json!(true));
    assert_eq!(
        summary["reason"],
        serde_json::json!("already_extracted"),
        "reason sentinel must distinguish from `run_pass2_not_configured` (pass-2 skip)"
    );
    assert_eq!(summary["page_count"], serde_json::json!(42));
    // Pin the field set so a future addition (e.g. a stray
    // `"detected_type": null`) is caught at test time.
    let obj = summary
        .as_object()
        .expect("skipped result_summary must be a JSON object");
    assert_eq!(
        obj.len(),
        3,
        "skipped result_summary must contain exactly 3 keys, got {obj:?}"
    );
}

#[test]
fn build_success_result_summary_emits_six_keys_with_rename() {
    let result = crate::pipeline::steps::extract_text::TextExtractionResult {
        page_count: 7,
        total_chars: 12_345,
        pages_native: 5,
        pages_ocr: 2,
        detected_document_type: Some("complaint".to_string()),
        ocr_engine: "surya".to_string(),
    };
    let summary = super::build_success_result_summary(&result);
    // Direct mappings.
    assert_eq!(summary["page_count"], serde_json::json!(7));
    assert_eq!(summary["total_chars"], serde_json::json!(12_345));
    assert_eq!(summary["pages_native"], serde_json::json!(5));
    assert_eq!(summary["pages_ocr"], serde_json::json!(2));
    assert_eq!(summary["ocr_engine"], serde_json::json!("surya"));
    // The rename contract: the struct field is `detected_document_type`
    // but the audit JSON key is `detected_type`. A silent rename of
    // either side breaks `pipeline_steps.result_summary` for external
    // tooling.
    assert_eq!(
        summary["detected_type"],
        serde_json::json!("complaint"),
        "detected_document_type → detected_type rename must hold"
    );
    assert!(
        summary.get("detected_document_type").is_none(),
        "the struct field name must NOT appear in the JSON"
    );
    let obj = summary
        .as_object()
        .expect("success result_summary must be a JSON object");
    assert_eq!(
        obj.len(),
        6,
        "success result_summary must contain exactly 6 keys, got {obj:?}"
    );
}

#[test]
fn build_success_result_summary_passes_none_through_as_null() {
    // detected_document_type is Option<String>; when None it must
    // serialize as JSON null (legacy parity).
    let result = crate::pipeline::steps::extract_text::TextExtractionResult {
        page_count: 1,
        total_chars: 0,
        pages_native: 0,
        pages_ocr: 0,
        detected_document_type: None,
        ocr_engine: "surya".to_string(),
    };
    let summary = super::build_success_result_summary(&result);
    assert!(summary["detected_type"].is_null());
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
