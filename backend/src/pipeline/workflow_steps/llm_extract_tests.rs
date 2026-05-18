//! Unit tests for [`super::classify_llm_extract_error`],
//! [`super::classify_dyn_llm_error`], and the
//! [`super::build_pass1_result_summary`] /
//! [`super::build_pass2_result_summary`] /
//! [`super::build_pass2_not_configured_summary`] audit-shape
//! builders.
//!
//! Lives in a sibling file (rather than a `mod tests { ... }` block
//! inside `llm_extract.rs`) so the runtime file stays under the
//! 300-line module-size budget. Wired into the runtime module via
//! `#[cfg(test)] #[path = "llm_extract_tests.rs"] mod tests;` — the
//! same idiom `pipeline/registry.rs` uses for `registry_tests.rs`
//! and `extract_text.rs` uses for `extract_text_tests.rs`.

use super::*;

/// Returns `true` when `e` is the Terminal branch of HandlerError.
fn display_message(e: &HandlerError) -> String {
    let inner: &dyn Error = e.as_ref();
    format!("{inner}")
}

fn is_terminal(e: &HandlerError) -> bool {
    display_message(e).starts_with("Terminal error")
}

// ── Terminal variants ───────────────────────────────────────

#[test]
fn classify_document_not_found_is_terminal() {
    let err = LlmExtractError::DocumentNotFound {
        document_id: "doc-x".into(),
    };
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
    assert!(is_terminal(&c), "DocumentNotFound must be terminal");
    let msg = display_message(&c);
    assert!(msg.contains("doc-x"), "msg must name doc_id: {msg}");
    assert!(
        msg.contains("upload completed"),
        "msg must hint recovery: {msg}"
    );
}

#[test]
fn classify_no_pipeline_config_is_terminal() {
    let err = LlmExtractError::NoPipelineConfig {
        document_id: "doc-x".into(),
    };
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
    assert!(is_terminal(&c));
    let msg = display_message(&c);
    assert!(
        msg.contains("config-creation"),
        "msg must point at config step: {msg}"
    );
}

#[test]
fn classify_profile_load_failed_is_terminal() {
    let err = LlmExtractError::ProfileLoadFailed {
        message: "Profile file not found: /etc/profiles/missing.yaml".into(),
    };
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
    assert!(is_terminal(&c));
    let msg = display_message(&c);
    assert!(
        msg.contains("profile YAML"),
        "msg must mention profile YAML: {msg}"
    );
    assert!(msg.contains("redeploy"), "msg must hint deploy: {msg}");
}

#[test]
fn classify_model_not_found_is_terminal() {
    let err = LlmExtractError::ModelNotFound {
        model_id: "claude-deprecated".into(),
    };
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
    assert!(is_terminal(&c));
    let msg = display_message(&c);
    assert!(
        msg.contains("claude-deprecated"),
        "msg must name model: {msg}"
    );
    assert!(
        msg.contains("llm_models"),
        "msg must point at the table: {msg}"
    );
}

#[test]
fn classify_provider_construction_failed_is_terminal() {
    let err = LlmExtractError::ProviderConstructionFailed {
        message: "ANTHROPIC_API_KEY unset".into(),
    };
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
    assert!(is_terminal(&c));
    let msg = display_message(&c);
    assert!(
        msg.contains("ANTHROPIC_API_KEY") || msg.contains("LLM_PROVIDER"),
        "msg must name the env vars to check: {msg}"
    );
}

#[test]
fn classify_no_pass2_template_is_terminal() {
    let err = LlmExtractError::NoPass2Template {
        profile_name: "no_pass2_template_profile".into(),
    };
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass2", &err);
    assert!(is_terminal(&c));
    let msg = display_message(&c);
    assert!(
        msg.contains("no_pass2_template_profile"),
        "msg must name the profile: {msg}"
    );
    assert!(
        msg.contains("run_pass2"),
        "msg must mention run_pass2: {msg}"
    );
}

#[test]
fn classify_no_completed_pass1_is_terminal() {
    let err = LlmExtractError::NoCompletedPass1 {
        document_id: "doc-x".into(),
    };
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass2", &err);
    assert!(is_terminal(&c));
    let msg = display_message(&c);
    assert!(
        msg.contains("Pass-1"),
        "msg must mention pass-1 prerequisite: {msg}"
    );
}

#[test]
fn classify_no_text_pages_is_terminal() {
    let err = LlmExtractError::NoTextPages {
        document_id: "doc-x".into(),
    };
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
    assert!(is_terminal(&c));
    let msg = display_message(&c);
    assert!(
        msg.contains("extract_text"),
        "msg must point at extract_text: {msg}"
    );
}

#[test]
fn classify_schema_load_failed_is_terminal() {
    // Use a real PipelineError construction path via from_file on
    // a missing file. The construction details aren't critical to
    // the classification — we just need the variant.
    // Simulate it: build via the source error's Display being the
    // important part for the message.
    // We'll construct with a minimal stand-in PipelineError via
    // the existing path. Falls back to a synthetic if needed.
    use colossus_extract::ExtractionSchema;
    let schema_err = ExtractionSchema::from_file(std::path::Path::new(
        "/nonexistent/path/should/never/exist.json",
    ))
    .expect_err("missing schema file should fail to load");
    let err = LlmExtractError::SchemaLoadFailed {
        schema_file: "missing.json".into(),
        source: schema_err,
    };
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
    assert!(is_terminal(&c));
    let msg = display_message(&c);
    assert!(
        msg.contains("missing.json"),
        "msg must name the schema: {msg}"
    );
}

#[test]
fn classify_response_not_json_is_terminal() {
    // ResponseNotJson carries an inner serde_json::Error. We
    // generate one via a parse failure.
    let serde_err = serde_json::from_str::<serde_json::Value>("not-json-text")
        .expect_err("malformed JSON must error");
    let err = LlmExtractError::ResponseNotJson {
        preview: "garbage llm output".into(),
        source: serde_err,
    };
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
    assert!(is_terminal(&c));
    let msg = display_message(&c);
    assert!(msg.contains("non-JSON"), "msg must say what's wrong: {msg}");
    assert!(
        msg.contains("garbage llm output"),
        "msg must include preview: {msg}"
    );
}

#[test]
fn classify_entity_serialization_failed_is_terminal() {
    let serde_err = serde_json::from_str::<serde_json::Value>("not-json-text")
        .expect_err("malformed JSON must error");
    let err = LlmExtractError::EntitySerializationFailed {
        entity_index: 7,
        source: serde_err,
    };
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
    assert!(is_terminal(&c));
    let msg = display_message(&c);
    assert!(
        msg.contains("programming bug"),
        "msg must call out the bug class: {msg}"
    );
}

#[test]
fn classify_relationship_serialization_failed_is_terminal() {
    let serde_err = serde_json::from_str::<serde_json::Value>("not-json-text")
        .expect_err("malformed JSON must error");
    let err = LlmExtractError::RelationshipSerializationFailed {
        rel_index: 3,
        source: serde_err,
    };
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass2", &err);
    assert!(is_terminal(&c));
}

#[test]
fn classify_prompt_build_failed_is_terminal() {
    // PromptBuildFailed carries a colossus_extract::PipelineError. We
    // synthesize one through the same source error path the schema
    // test uses.
    use colossus_extract::ExtractionSchema;
    let pe = ExtractionSchema::from_file(std::path::Path::new("/nonexistent/prompt/schema.json"))
        .expect_err("missing schema should fail");
    let err = LlmExtractError::PromptBuildFailed { source: pe };
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
    assert!(is_terminal(&c));
    let msg = display_message(&c);
    assert!(
        msg.contains("template"),
        "msg must point at template: {msg}"
    );
}

// ── Operator-initiated cancellation ─────────────────────────

#[test]
fn classify_cancelled_is_terminal_and_not_retryable() {
    // The cooperative-cancellation poller short-circuited the chunk
    // loop after the operator hit Cancel. MUST be terminal — a
    // retryable classification would bounce the cancelled invocation
    // through Restate's retry loop and undo the whole point of
    // polling `documents.is_cancelled` between chunks.
    let err = LlmExtractError::Cancelled {
        document_id: "doc-x".into(),
        chunks_completed: 3,
        chunks_total: 14,
    };
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
    assert!(is_terminal(&c), "Cancelled MUST be terminal, not retryable");
    let msg = display_message(&c);
    assert!(
        msg.contains("doc-x"),
        "msg must name doc_id for the audit log: {msg}"
    );
    assert!(
        msg.contains("3/14") || (msg.contains("3") && msg.contains("14")),
        "msg must record how far the run got before cancel: {msg}"
    );
    assert!(
        msg.contains("operator"),
        "msg must identify the cause as operator action: {msg}"
    );
    assert!(
        !msg.contains("Will retry"),
        "Cancelled must NOT carry the retry hint: {msg}"
    );
}

#[test]
fn classify_cancelled_at_pass2_entry_records_zero_chunks() {
    // Pass-2 polls the flag once at function entry (single-call, no
    // chunking). Both `chunks_completed` and `chunks_total` are `0`,
    // distinguishing "cancelled at pass-2 entry" from "cancelled
    // mid-chunk" in the audit log.
    let err = LlmExtractError::Cancelled {
        document_id: "doc-y".into(),
        chunks_completed: 0,
        chunks_total: 0,
    };
    let c = classify_llm_extract_error("doc-y", "llm_extract_pass2", &err);
    assert!(is_terminal(&c));
    let msg = display_message(&c);
    assert!(
        msg.contains("0/0"),
        "pass-2 entry cancel must show 0/0: {msg}"
    );
    assert!(
        msg.contains("llm_extract_pass2"),
        "step_name must propagate: {msg}"
    );
}

// ── Retryable variants ──────────────────────────────────────

#[test]
fn classify_llm_call_failed_is_retryable() {
    use colossus_extract::ExtractionSchema;
    let pe = ExtractionSchema::from_file(std::path::Path::new("/nonexistent.json"))
        .expect_err("missing should fail");
    let err = LlmExtractError::LlmCallFailed { source: pe };
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
    assert!(!is_terminal(&c), "LlmCallFailed must be retryable: {c:?}");
    let msg = display_message(&c);
    assert!(msg.contains("Will retry"), "msg must signal retry: {msg}");
}

#[test]
fn classify_semaphore_closed_is_retryable() {
    let err = LlmExtractError::SemaphoreClosed;
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
    assert!(!is_terminal(&c), "SemaphoreClosed must be retryable");
}

#[test]
fn classify_insert_run_failed_is_retryable() {
    let err = LlmExtractError::InsertRunFailed {
        message: "connection refused".into(),
    };
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
    assert!(!is_terminal(&c));
}

#[test]
fn classify_complete_run_failed_is_retryable() {
    let err = LlmExtractError::CompleteRunFailed {
        message: "tx timeout".into(),
    };
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
    assert!(!is_terminal(&c));
}

#[test]
fn classify_store_failed_is_retryable() {
    let err = LlmExtractError::StoreFailed {
        message: "deadlock detected".into(),
    };
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err);
    assert!(!is_terminal(&c));
}

// ── `build_*_result_summary` shape contracts ────────────────

#[test]
fn build_pass1_result_summary_emits_11_keys_with_nulls_on_skip() {
    // skipped_already_complete=true path: all numeric fields and
    // strings should be None → JSON null. This pins the
    // shape-stays-the-same contract even on the no-work path.
    let result = crate::pipeline::steps::llm_extract::Pass1ExtractionResult {
        entity_count: None,
        relationship_count: None,
        input_tokens: None,
        output_tokens: None,
        run_pass2: true,
        skipped_already_complete: true,
        chunk_count: None,
        chunks_succeeded: None,
        chunks_failed: None,
        profile: None,
        model: None,
        chunking_mode: None,
        system_prompt_file: None,
    };
    let summary = super::build_pass1_result_summary(&result);
    // All 11 keys must be present, all set to JSON null.
    for key in [
        "entity_count",
        "relationship_count",
        "input_tokens",
        "output_tokens",
        "chunk_count",
        "chunks_succeeded",
        "chunks_failed",
        "profile",
        "model",
        "chunking_mode",
        "system_prompt_file",
    ] {
        assert!(
            summary.get(key).is_some(),
            "key '{key}' must be present in pass1 result_summary"
        );
        assert!(
            summary[key].is_null(),
            "key '{key}' on the skip path must be JSON null, got {:?}",
            summary[key]
        );
    }
    let obj = summary
        .as_object()
        .expect("result_summary must be a JSON object");
    assert_eq!(obj.len(), 11);
}

#[test]
fn build_pass1_result_summary_passes_concrete_values_through() {
    let result = crate::pipeline::steps::llm_extract::Pass1ExtractionResult {
        entity_count: Some(42),
        relationship_count: Some(8),
        input_tokens: Some(1_500),
        output_tokens: Some(600),
        run_pass2: false,
        skipped_already_complete: false,
        chunk_count: Some(3),
        chunks_succeeded: Some(3),
        chunks_failed: Some(0),
        profile: Some("complaint".to_string()),
        model: Some("claude-sonnet-4-6".to_string()),
        chunking_mode: Some("structured".to_string()),
        system_prompt_file: Some("legal_v1.md".to_string()),
    };
    let summary = super::build_pass1_result_summary(&result);
    assert_eq!(summary["entity_count"], serde_json::json!(42));
    assert_eq!(summary["chunks_succeeded"], serde_json::json!(3));
    assert_eq!(summary["profile"], serde_json::json!("complaint"));
    assert_eq!(summary["model"], serde_json::json!("claude-sonnet-4-6"));
}

#[test]
fn build_pass2_result_summary_emits_pass_literal_2_and_9_keys() {
    let result = crate::pipeline::steps::llm_extract_pass2::Pass2ExtractionResult {
        relationship_count: 14,
        local_entities: 8,
        cross_doc_entities: 2,
        input_tokens: 2_100,
        output_tokens: 450,
        profile: Some("complaint".to_string()),
        model: Some("claude-opus-4-7".to_string()),
        pass2_template_file: Some("pass2_complaint.md".to_string()),
        skipped_already_complete: false,
    };
    let summary = super::build_pass2_result_summary(&result);
    // The literal `pass: 2` is the audit-trail contract — not
    // a result-struct field. Pinning it here guards against
    // accidental removal.
    assert_eq!(
        summary["pass"],
        serde_json::json!(2),
        "pass: 2 literal must be present in pass-2 result_summary"
    );
    assert_eq!(summary["relationship_count"], serde_json::json!(14));
    assert_eq!(summary["local_entities"], serde_json::json!(8));
    assert_eq!(summary["cross_doc_entities"], serde_json::json!(2));
    assert_eq!(
        summary["pass2_template_file"],
        serde_json::json!("pass2_complaint.md")
    );
    let obj = summary
        .as_object()
        .expect("result_summary must be a JSON object");
    assert_eq!(obj.len(), 9);
}

#[test]
fn build_pass2_not_configured_summary_distinct_from_already_complete() {
    let summary = super::build_pass2_not_configured_summary();
    assert_eq!(summary["skipped"], serde_json::json!(true));
    assert_eq!(
        summary["reason"],
        serde_json::json!("run_pass2_not_configured"),
        "reason sentinel must distinguish from pass-1's 'already_extracted' \
         and from the post-orchestrator already-complete path"
    );
    let obj = summary
        .as_object()
        .expect("result_summary must be a JSON object");
    assert_eq!(
        obj.len(),
        2,
        "not-configured summary must have exactly 2 keys"
    );
}

// ── Unknown error type (downcast miss) ──────────────────────

#[test]
fn classify_dyn_unknown_error_is_retryable() {
    // A non-LlmExtractError boxed error — e.g. a sqlx::Error
    // promoted to Box<dyn Error>. The downcast misses and we
    // fall back to retryable to avoid locking up on a transient
    // we couldn't classify.
    let boxed: Box<dyn Error + Send + Sync> = "sudden infrastructure blip".into();
    let c = classify_dyn_llm_error("doc-x", "llm_extract_pass1", boxed);
    assert!(
        !is_terminal(&c),
        "unknown error must default to retryable: {c:?}"
    );
    let msg = display_message(&c);
    assert!(
        msg.contains("unclassified"),
        "msg must signal unknown type: {msg}"
    );
}
