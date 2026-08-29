//! Behavioural tests for the truncation detector.
//!
//! The fixtures are the measured shape of the defect: run 171 of
//! `doc-penzien-coa-brief-03-14-2011` — 32,000 output tokens against a 32,000
//! cap — and a normal run beside it.

use super::*;

fn shape<'a>(stop: Option<&'a str>, out: Option<u64>, cap: u32) -> CallShape<'a> {
    CallShape {
        stop_reason: stop,
        output_tokens: out,
        configured_max_tokens: cap,
        model: "claude-opus-5",
        // A representative healthy-shaped anatomy. The detector does not read
        // it — it only reaches the MESSAGE — so the fixture keeps one value and
        // the tests that care assert on it directly.
        anatomy: "response anatomy: 1 content blocks (text ×1); \
                  output_tokens=32000; stop_reason=max_tokens",
    }
}

// ── The detector ─────────────────────────────────────────────────────────────

#[test]
fn the_run_171_shape_is_detected_as_truncated() {
    // Exactly what the census measured, now caught.
    assert!(is_truncated(&shape(
        Some(STOP_REASON_MAX_TOKENS),
        Some(32_000),
        32_000
    )));
}

#[test]
fn a_normal_end_of_turn_response_passes_unchanged() {
    assert!(!is_truncated(&shape(
        Some("end_turn"),
        Some(16_199),
        32_000
    )));
}

#[test]
fn an_unreported_stop_reason_is_not_truncation() {
    // A local vLLM model, or a future adapter. "Not reported" and "reported
    // something that is not max_tokens" are both non-truncation, and neither may
    // fail a call — see the doc comment on `is_truncated`.
    assert!(!is_truncated(&shape(None, Some(2_048), 2_048)));
}

#[test]
fn hitting_the_cap_without_the_stop_reason_is_NOT_truncation() {
    // The load-bearing negative. An extraction that legitimately consumed its
    // whole budget and finished must not fail. Guessing from the token count is
    // precisely the heuristic this detector refuses.
    assert!(!is_truncated(&shape(
        Some("end_turn"),
        Some(32_000),
        32_000
    )));
}

#[test]
fn a_tool_use_stop_is_not_truncation() {
    assert!(!is_truncated(&shape(Some("tool_use"), Some(900), 32_000)));
}

// ── The message ──────────────────────────────────────────────────────────────

#[test]
fn the_message_names_the_model_the_cap_and_what_was_produced() {
    let msg = truncation_message(&shape(Some(STOP_REASON_MAX_TOKENS), Some(32_000), 32_000));
    assert!(msg.contains("claude-opus-5"), "names the model: {msg}");
    assert!(msg.contains("32000"), "names the cap and the count: {msg}");
    assert!(msg.contains("TRUNCATED"), "says what happened: {msg}");
    assert!(
        msg.contains("max_tokens"),
        "names the setting to change: {msg}"
    );
    assert!(
        msg.contains("profile"),
        "says where to change it — the remedy is a profile edit: {msg}"
    );
}

#[test]
fn unreported_output_tokens_read_as_not_reported_never_as_zero() {
    // Standing Rule 1: "the provider said 0" and "the provider said nothing" are
    // different operator problems and must not read alike.
    let msg = truncation_message(&shape(Some(STOP_REASON_MAX_TOKENS), None, 64_000));
    assert!(msg.contains("not reported"), "{msg}");
    assert!(
        !msg.contains("Produced 0 "),
        "must not present an absent count as zero: {msg}"
    );
}

// ── The recogniser (terminal-vs-retryable, 2026-08-27) ───────────────────────

#[test]
fn the_message_says_what_the_response_consisted_of() {
    // Added 2026-08-28. A truncation and an all-reasoning-blocks response are
    // neighbours — both are the token budget going somewhere other than the
    // answer — and which one happened decides the REMEDY. Raising max_tokens on
    // a response that spent its ceiling thinking buys a longer, equally empty
    // run; the anatomy is what tells the two apart at a glance.
    let msg = truncation_message(&shape(Some(STOP_REASON_MAX_TOKENS), Some(32_000), 32_000));
    assert!(
        msg.contains("response anatomy:"),
        "the failure must say what arrived, not only that it stopped: {msg}"
    );
    assert!(
        msg.contains("LLM_EXTRACTION_EFFORT"),
        "and must name the OTHER remedy when the blocks point at thinking: {msg}"
    );
    // The original remedy survives — this is an addition, not a replacement.
    assert!(
        msg.contains("max_tokens") && msg.contains("profile"),
        "{msg}"
    );
}

#[test]
fn the_message_a_truncation_produces_is_recognised_as_one() {
    // THE round trip, and the reason the message may be reworded safely: the
    // gate builds the string, the classifier reads it back, and both go through
    // `TRUNCATION_SIGNATURE`. If a future edit reworded the message by hand and
    // left the recogniser behind, truncation would silently become retryable
    // again — which is exactly the defect this test exists to prevent.
    let msg = truncation_message(&shape(Some(STOP_REASON_MAX_TOKENS), Some(32_000), 32_000));
    let err = PipelineError::LlmProvider(msg);
    assert!(
        is_truncation_failure(&err),
        "the gate's own message must be recognised by the classifier: {err}"
    );
}

#[test]
fn an_ordinary_provider_failure_is_not_a_truncation() {
    // The load-bearing negative on this side: a dropped connection, a 500, an
    // auth failure. These are genuinely transient and MUST stay retryable —
    // misclassifying them as terminal would turn a blip into a failed run.
    let err = PipelineError::LlmProvider("model claude-opus-4-6: connection reset".to_string());
    assert!(!is_truncation_failure(&err));
}

#[test]
fn a_rate_limit_is_never_a_truncation() {
    // 429 has its own typed variant and its own retry loop in `llm_retry`.
    // Narrowing on the variant before looking at any text is what guarantees
    // the two policies cannot cross.
    let err = PipelineError::RateLimited {
        retry_after_secs: 45,
    };
    assert!(!is_truncation_failure(&err));
}

#[test]
fn the_signature_is_not_matched_outside_the_provider_variant() {
    // A document's own text can say anything, including this sentence. Only an
    // error raised BY the gate — an `LlmProvider` error — may be classified as
    // a truncation; the same words arriving in an extraction error are data.
    let borrowed = truncation_message(&shape(Some(STOP_REASON_MAX_TOKENS), Some(8), 8));
    assert!(
        !is_truncation_failure(&PipelineError::Extraction(borrowed)),
        "the variant must be part of the match, not just the text"
    );
}

// ── The wiring, asserted against the source ──────────────────────────────────

#[test]
fn the_bridge_checks_before_it_builds_a_response_on_both_entry_points() {
    // The detector is worthless if it is not consulted, and it must be consulted
    // BEFORE the text reaches any parser — `repair_json` is what makes a
    // truncated response look complete, so a check after parsing would be a
    // check after the damage. Both `invoke` and `invoke_with_system` are entry
    // points; pass 1 uses one and pass 2 the other, so a guard on only one would
    // leave half the pipeline unprotected.
    let bridge = include_str!("rig_llm_bridge.rs");
    let calls = bridge
        .matches("self.guard_truncation(&result, max_tokens)?")
        .count();
    assert_eq!(
        calls, 2,
        "both invoke and invoke_with_system must call the guard"
    );
    assert!(
        bridge.contains("truncation::is_truncated"),
        "the guard must consult the detector rather than re-deciding"
    );

    // And the provider must actually capture the field, or the detector only
    // ever sees None and can never fire.
    // And the adapter must actually capture the field, or the detector only ever
    // sees None and can never fire. Since 2026-08-28 the transport streams, and
    // `stop_reason` arrives in the `message_delta` event rather than in a
    // response body — so BOTH halves are pinned: the accumulator must read it
    // out of the stream, and the adapter must carry it onto the call result.
    let accumulator = include_str!("anthropic_stream.rs");
    assert!(
        accumulator.contains("stop_reason"),
        "anthropic_stream must read stop_reason out of the message_delta event"
    );
    let adapter = include_str!("anthropic_engine.rs");
    assert!(
        adapter.contains("stop_reason: Some(message.stop_reason)"),
        "anthropic_engine must carry stop_reason onto the LlmCallResult"
    );
}

#[test]
fn both_passes_classify_the_failure_through_the_shared_constructor() {
    // Pass 1 and pass 2 wrap a failed provider call in their own file. If either
    // one names `LlmCallFailed` directly again, ITS truncations go back to being
    // retryable while the other pass's do not — a split-brain no unit test of
    // the classifier would catch, because the classifier would still be right.
    // The constructor is the single decision point; this asserts both call sites
    // still go through it.
    for (name, src) in [
        ("llm_extract", include_str!("steps/llm_extract.rs")),
        (
            "llm_extract_pass2",
            include_str!("steps/llm_extract_pass2.rs"),
        ),
    ] {
        assert!(
            src.contains("LlmExtractError::from_provider_failure(e)"),
            "{name} must wrap provider failures via the shared constructor"
        );
        assert!(
            !src.contains("LlmExtractError::LlmCallFailed { source: e }"),
            "{name} must not re-introduce the unclassified wrap"
        );
    }
}
