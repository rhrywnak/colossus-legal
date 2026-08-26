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
    let provider = include_str!("rig_provider.rs");
    assert!(
        provider.contains("stop_reason"),
        "rig_provider must carry stop_reason out of the API response"
    );
}
