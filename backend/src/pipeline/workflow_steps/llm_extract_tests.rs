//! Unit tests for the [`super::build_pass1_result_summary`] /
//! [`super::build_pass2_result_summary`] /
//! [`super::build_pass2_not_configured_summary`] audit-shape builders.
//!
//! The terminal-vs-retryable classification tests moved to
//! `llm_extract_classify_tests.rs` on 2026-08-28, when the classifier itself
//! moved to its own module.

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
fn build_pass2_result_summary_emits_pass_literal_2_and_11_keys() {
    let result = crate::pipeline::steps::llm_extract_pass2::Pass2ExtractionResult {
        relationship_count: 14,
        local_entities: 8,
        cross_doc_entities: 2,
        authored_context_entities: 5,
        input_tokens: 2_100,
        output_tokens: 450,
        profile: Some("complaint".to_string()),
        model: Some("claude-opus-4-7".to_string()),
        pass2_template_file: Some("pass2_complaint.md".to_string()),
        skipped_already_complete: false,
        edge_bar: crate::pipeline::edge_bar::EdgeBarCounts {
            accepted: 14,
            exact_duplicates: 1,
            deduped: 2,
            rejected_by_pattern: 0,
            pattern_warnings: 3,
        },
    };
    let summary = super::build_pass2_result_summary(&result);
    // The literal `pass: 2` is the audit-trail contract — not
    // a result-struct field. Pinning it here guards against
    // accidental removal.
    // The edge bar's tally rides the same summary. Pinned per-field rather than
    // as one blob so a dropped counter names itself in the failure.
    assert_eq!(summary["edge_bar"]["accepted"], serde_json::json!(14));
    assert_eq!(
        summary["edge_bar"]["exact_duplicates"],
        serde_json::json!(1)
    );
    assert_eq!(summary["edge_bar"]["deduped"], serde_json::json!(2));
    assert_eq!(
        summary["edge_bar"]["rejected_by_pattern"],
        serde_json::json!(0)
    );
    assert_eq!(
        summary["edge_bar"]["pattern_warnings"],
        serde_json::json!(3)
    );
    assert_eq!(
        summary["pass"],
        serde_json::json!(2),
        "pass: 2 literal must be present in pass-2 result_summary"
    );
    assert_eq!(summary["relationship_count"], serde_json::json!(14));
    assert_eq!(summary["local_entities"], serde_json::json!(8));
    assert_eq!(summary["cross_doc_entities"], serde_json::json!(2));
    assert_eq!(summary["authored_context_entities"], serde_json::json!(5));
    assert_eq!(
        summary["pass2_template_file"],
        serde_json::json!("pass2_complaint.md")
    );
    let obj = summary
        .as_object()
        .expect("result_summary must be a JSON object");
    // 11 since 2026-08-25: the ten original keys plus `edge_bar`. The count is
    // pinned deliberately — a new key on this payload is an audit-trail change
    // and should have to be stated here, not arrive by accident.
    assert_eq!(obj.len(), 11);
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
