//! Unit tests for [`super::classify_llm_extract_error`] and
//! [`super::classify_dyn_llm_error`].
//!
//! One test per `LlmExtractError` variant, asserting terminal-vs-retryable
//! through the SDK's `Display` impl ("Terminal error" vs "Retryable error"
//! prefix on `HandlerError::as_ref()`).
//!
//! Lives in a sibling file (rather than a `mod tests { ... }` block inside
//! `llm_extract_classify.rs`) so the runtime file stays under the 300-line
//! module-size budget — the same idiom `pipeline/registry.rs` uses for
//! `registry_tests.rs`.

use super::*;

// The truncation fixtures below build the gate's real message rather than a
// hand-written lookalike — a hand-written one could drift from what the gate
// actually emits and the test would keep passing against a message nobody sends.
use crate::pipeline::truncation;

/// The retry cap these tests classify under, unless a test states another.
///
/// Bound to the real shipped default rather than a literal `0`, so if
/// `LLM_RETRY_MAX`'s default is ever changed the tests move with it instead of
/// silently continuing to assert a policy the product no longer has.
const SHIPPED_RETRY_MAX: u32 = crate::llm_retry_policy::DEFAULT_MAX_RETRIES;

/// A cap an operator might set after deciding automatic retries are worth the
/// money — the "raising it needs no code change" half of the ruling.
const RAISED_RETRY_MAX: u32 = 3;

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
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err, SHIPPED_RETRY_MAX);
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
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err, SHIPPED_RETRY_MAX);
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
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err, SHIPPED_RETRY_MAX);
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
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err, SHIPPED_RETRY_MAX);
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
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err, SHIPPED_RETRY_MAX);
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
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass2", &err, SHIPPED_RETRY_MAX);
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
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass2", &err, SHIPPED_RETRY_MAX);
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
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err, SHIPPED_RETRY_MAX);
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
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err, SHIPPED_RETRY_MAX);
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
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err, SHIPPED_RETRY_MAX);
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
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err, SHIPPED_RETRY_MAX);
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
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass2", &err, SHIPPED_RETRY_MAX);
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
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err, SHIPPED_RETRY_MAX);
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
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err, SHIPPED_RETRY_MAX);
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
    let c = classify_llm_extract_error("doc-y", "llm_extract_pass2", &err, SHIPPED_RETRY_MAX);
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

#[test]
fn classify_response_truncated_is_terminal() {
    // The whole point of the 2026-08-27 change. A truncation is deterministic:
    // the retry sends the same prompt to the same model under the same cap and
    // is cut off in the same place, so Restate must not retry it.
    let source = colossus_extract::PipelineError::LlmProvider(truncation::truncation_message(
        &truncation::CallShape {
            stop_reason: Some(truncation::STOP_REASON_MAX_TOKENS),
            output_tokens: Some(32_000),
            configured_max_tokens: 32_000,
            model: "claude-opus-4-6",
            anatomy: "response anatomy: 1 content blocks (text ×1); \
                      output_tokens=32000; stop_reason=max_tokens",
        },
    ));
    let err = LlmExtractError::ResponseTruncated { source };
    let c = classify_llm_extract_error("doc-motion", "llm_extract_pass1", &err, SHIPPED_RETRY_MAX);
    assert!(is_terminal(&c), "ResponseTruncated must be terminal: {c:?}");

    let msg = display_message(&c);
    // The operator keeps the message that tells them what to do...
    assert!(
        msg.contains("claude-opus-4-6") && msg.contains("32000"),
        "the gate's message must survive classification whole: {msg}"
    );
    assert!(
        msg.contains("max_tokens") && msg.contains("profile"),
        "the remedy must still be stated: {msg}"
    );
    assert!(msg.contains("doc-motion"), "msg must name doc_id: {msg}");
    // ...minus the promise of a retry that will not happen.
    assert!(
        !msg.contains("Will retry"),
        "a terminal failure must not tell the operator it will retry: {msg}"
    );
}

#[test]
fn a_truncated_provider_error_becomes_the_truncated_variant_not_a_call_failure() {
    // The constructor is what stands between the gate and the classifier. This
    // asserts the routing end to end at the type level: gate message in, TERMINAL
    // classification out — with no string matching anywhere but inside
    // `truncation::is_truncation_failure`.
    let truncated = colossus_extract::PipelineError::LlmProvider(truncation::truncation_message(
        &truncation::CallShape {
            stop_reason: Some(truncation::STOP_REASON_MAX_TOKENS),
            output_tokens: Some(64_000),
            configured_max_tokens: 64_000,
            model: "claude-opus-4-6",
            anatomy: "response anatomy: 1 content blocks (text ×1); \
                      output_tokens=32000; stop_reason=max_tokens",
        },
    ));
    let err = LlmExtractError::from_provider_failure(truncated);
    assert!(
        matches!(err, LlmExtractError::ResponseTruncated { .. }),
        "a gate failure must be typed as a truncation"
    );
    assert!(is_terminal(&classify_llm_extract_error(
        "doc-motion",
        "llm_extract_pass1",
        &err,
        SHIPPED_RETRY_MAX
    )));

    // And the converse: an ordinary provider failure keeps its retryable path.
    let transient =
        colossus_extract::PipelineError::LlmProvider("connection reset by peer".to_string());
    let err = LlmExtractError::from_provider_failure(transient);
    assert!(
        matches!(err, LlmExtractError::LlmCallFailed { .. }),
        "a transient provider failure must NOT be typed as a truncation"
    );
    // Under the shipped policy BOTH are terminal — but for different reasons,
    // and the messages must say so. A truncation is terminal because retrying
    // cannot work; a call failure is terminal because the operator has said not
    // to spend the money without being asked. Raise the cap and only the second
    // one moves.
    let at_zero = classify_llm_extract_error("doc-motion", "llm_extract_pass1", &err, 0);
    assert!(
        is_terminal(&at_zero),
        "a call failure at cap 0 must be terminal"
    );
    let raised =
        classify_llm_extract_error("doc-motion", "llm_extract_pass1", &err, RAISED_RETRY_MAX);
    assert!(
        !is_terminal(&raised),
        "a raised cap must hand the retry decision back to the engine"
    );
}

// ── Retryable variants ──────────────────────────────────────

/// Build an `LlmCallFailed` carrying a real `PipelineError`.
fn a_call_failure() -> LlmExtractError {
    use colossus_extract::ExtractionSchema;
    let pe = ExtractionSchema::from_file(std::path::Path::new("/nonexistent.json"))
        .expect_err("a missing schema file always fails");
    LlmExtractError::LlmCallFailed { source: pe }
}

#[test]
fn classify_llm_call_failed_is_terminal_under_the_shipped_zero_cap() {
    // The 2026-08-28 ruling. This arm used to be unconditionally retryable, and
    // that is how a 600s timeout on a healthy generation got paid for twice: the
    // step failed, Restate re-ran it, and the second call hit the same wall.
    let err = a_call_failure();
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err, SHIPPED_RETRY_MAX);
    assert!(
        is_terminal(&c),
        "under LLM_RETRY_MAX=0 an LLM-call failure must be TERMINAL so the engine \
         cannot re-invoke it: {c:?}"
    );
    let msg = display_message(&c);
    assert!(
        msg.contains("LLM_RETRY_MAX"),
        "the operator must be told which key governs this: {msg}"
    );
    assert!(
        msg.contains("Re-process"),
        "the operator must be told what resumes the run: {msg}"
    );
    assert!(
        !msg.contains("Will retry") && !msg.contains("will retry"),
        "a terminal failure must not promise a retry that will not happen: {msg}"
    );
}

#[test]
fn classify_llm_call_failed_is_retryable_once_the_cap_is_raised() {
    // The other half of "configurable without a code change": the classification
    // is a FUNCTION of the cap, so an operator who sets LLM_RETRY_MAX=3 gets the
    // engine's retry behaviour back without anyone editing this file.
    let err = a_call_failure();
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err, RAISED_RETRY_MAX);
    assert!(!is_terminal(&c), "a raised cap must be retryable: {c:?}");
    let msg = display_message(&c);
    assert!(
        msg.contains("3"),
        "the message must name the cap in force: {msg}"
    );
}

#[test]
fn a_truncation_stays_terminal_no_matter_how_high_the_cap_is_raised() {
    // Ruled 2026-08-27 and unchanged by the retry work: no number of retries
    // against the same max_tokens ceiling changes the outcome, so the cap has no
    // say here. If this ever fails, raising LLM_RETRY_MAX has quietly re-armed
    // the retry loop that census R-3 disarmed.
    let source = colossus_extract::PipelineError::LlmProvider(truncation::truncation_message(
        &truncation::CallShape {
            stop_reason: Some(truncation::STOP_REASON_MAX_TOKENS),
            output_tokens: Some(64_000),
            configured_max_tokens: 64_000,
            model: "claude-opus-5",
            anatomy: "response anatomy: 1 content blocks (text ×1); \
                      output_tokens=64000; stop_reason=max_tokens",
        },
    ));
    let err = LlmExtractError::ResponseTruncated { source };
    for cap in [0, 1, RAISED_RETRY_MAX, 99] {
        let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err, cap);
        assert!(is_terminal(&c), "truncation must be terminal at cap {cap}");
    }
}

#[test]
fn a_streamed_response_cut_off_at_the_ceiling_still_classifies_terminal() {
    // The end-to-end proof for the transport change, from the wire up.
    //
    // Before 2026-08-28 `stop_reason` came out of a single response body. It now
    // arrives in a `message_delta` event partway through a stream, and the whole
    // truncation gate hangs off that one field. This walks the real path — SSE
    // transcript → accumulator → the gate's detector → the gate's message → the
    // typed variant → the classifier — and asserts the answer is unchanged.
    //
    // No API call: the transcript is a string literal. Money has been burned
    // twice this week on retried calls; none of it was burned here.
    use crate::pipeline::anthropic_stream::{MessageAccumulator, Progress};

    const CUT_OFF_AT_THE_CEILING: &str = concat!(
        r#"data: {"type":"message_start","message":{"id":"msg_stream","usage":{"input_tokens":41255,"output_tokens":1}}}"#,
        "\n\n",
        r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
        "\n\n",
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"partial extraction, cut off mid-arr"}}"#,
        "\n\n",
        r#"data: {"type":"message_delta","delta":{"stop_reason":"max_tokens"},"usage":{"output_tokens":64000}}"#,
        "\n\n",
        r#"data: {"type":"message_stop"}"#,
        "\n\n",
    );

    let mut accumulator = MessageAccumulator::new();
    for payload in CUT_OFF_AT_THE_CEILING.split("\n\n") {
        let Some(json) = payload.strip_prefix("data: ") else {
            continue;
        };
        if accumulator
            .push(json)
            .expect("the transcript is well-formed")
            == Progress::Done
        {
            break;
        }
    }
    let message = accumulator.finish().expect("the stream completed");

    // The transport carried the field out of `message_delta`, and — since
    // 2026-08-28 — the block counts out of `content_block_start`, so the gate's
    // message can say what the response consisted of as well as that it stopped.
    let anatomy = crate::pipeline::response_anatomy::anatomy_line(
        &message.block_counts,
        message.output_tokens,
        Some(message.stop_reason.as_str()),
    );
    let shape = truncation::CallShape {
        stop_reason: Some(message.stop_reason.as_str()),
        output_tokens: message.output_tokens,
        configured_max_tokens: 64_000,
        model: "claude-opus-5",
        anatomy: &anatomy,
    };
    assert!(
        truncation::is_truncated(&shape),
        "a streamed max_tokens stop must still trip the gate"
    );

    // ...and the rest of the chain is unchanged from the non-streaming days.
    let err = LlmExtractError::from_provider_failure(colossus_extract::PipelineError::LlmProvider(
        truncation::truncation_message(&shape),
    ));
    assert!(
        matches!(err, LlmExtractError::ResponseTruncated { .. }),
        "a streamed truncation must be typed as a truncation, not a call failure"
    );
    let c = classify_llm_extract_error(
        "doc-transcript",
        "llm_extract_pass1",
        &err,
        SHIPPED_RETRY_MAX,
    );
    assert!(
        is_terminal(&c),
        "a streamed truncation must still classify TERMINAL: {c:?}"
    );
    let msg = display_message(&c);
    assert!(
        msg.contains("64000") && msg.contains("claude-opus-5"),
        "the operator still gets the cap and the model: {msg}"
    );
}

#[test]
fn classify_semaphore_closed_is_retryable() {
    let err = LlmExtractError::SemaphoreClosed;
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err, SHIPPED_RETRY_MAX);
    assert!(!is_terminal(&c), "SemaphoreClosed must be retryable");
}

#[test]
fn classify_insert_run_failed_is_retryable() {
    let err = LlmExtractError::InsertRunFailed {
        message: "connection refused".into(),
    };
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err, SHIPPED_RETRY_MAX);
    assert!(!is_terminal(&c));
}

#[test]
fn classify_complete_run_failed_is_retryable() {
    let err = LlmExtractError::CompleteRunFailed {
        message: "tx timeout".into(),
    };
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err, SHIPPED_RETRY_MAX);
    assert!(!is_terminal(&c));
}

#[test]
fn classify_store_failed_is_retryable() {
    let err = LlmExtractError::StoreFailed {
        message: "deadlock detected".into(),
    };
    let c = classify_llm_extract_error("doc-x", "llm_extract_pass1", &err, SHIPPED_RETRY_MAX);
    assert!(!is_terminal(&c));
}

// ── Unknown error type (downcast miss) ──────────────────────

#[test]
fn classify_dyn_unknown_error_is_retryable() {
    // A non-LlmExtractError boxed error — e.g. a sqlx::Error
    // promoted to Box<dyn Error>. The downcast misses and we
    // fall back to retryable to avoid locking up on a transient
    // we couldn't classify.
    let boxed: Box<dyn Error + Send + Sync> = "sudden infrastructure blip".into();
    let c = classify_dyn_llm_error("doc-x", "llm_extract_pass1", boxed, SHIPPED_RETRY_MAX);
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
