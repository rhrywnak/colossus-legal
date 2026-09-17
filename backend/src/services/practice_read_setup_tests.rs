//! What a read ASKS FOR: the budget, and where it comes from.
//!
//! Pure — no provider, no store. The spec is the last place the settings row is
//! still a number this build owns; after `resolve_model` it is the wire's.

use super::read_task_spec;
use crate::domain::llm_params::ParamValue;
use crate::domain::practice_params::PracticeReadParams;
use crate::pipeline::anthropic_engine::build_request_body;

/// The read's output ceiling is the settings row, whatever the row says.
///
/// ## Why this is worth a test of its own
///
/// The first v4 read on DEV abstained because the ceiling was 1024 and Opus 5's
/// adaptive thinking spent all of it before any text was emitted. The row was
/// raised to 4096; a literal reappearing here would make the Settings page a lie
/// — an operator could raise the row, watch nothing change, and have no way to
/// tell from the outside which number was actually sent.
#[test]
fn the_read_budget_comes_from_the_settings_row() {
    let mut read = PracticeReadParams::for_test();

    assert_eq!(
        read_task_spec(&read).max_tokens,
        ParamValue::Set(4096),
        "the fixture's own row is what the spec must carry"
    );

    // And it FOLLOWS the row rather than happening to agree with it.
    read.max_tokens = 64;
    assert_eq!(read_task_spec(&read).max_tokens, ParamValue::Set(64));
    read.max_tokens = 8192;
    assert_eq!(read_task_spec(&read).max_tokens, ParamValue::Set(8192));
}

/// The same answer earns the same read: temperature 0, and no per-call timeout.
///
/// Domain note, carried from the module: this temperature does NOT reach the wire
/// — the model row's `temperature_mode` is `omit`. The assertion pins what this
/// build ASKS for, so the day that mode changes, the ask is already right.
#[test]
fn a_read_is_deterministic_and_takes_the_deployments_timeout() {
    let spec = read_task_spec(&PracticeReadParams::for_test());

    assert_eq!(spec.temperature, ParamValue::Set(0.0));
    assert_eq!(spec.timeout_secs, ParamValue::Unset);
}

/// The request a read actually composes for Opus 5, from the two rows.
///
/// ## Why the wire body and not just the two fields
///
/// This is the shape the 2026-09-17 failure had wrong, and it had it wrong in a
/// way nothing downstream could see: `max_tokens` was 1024 and there was no
/// `output_config` at all, so the API applied its own default effort — `high` —
/// and Opus 5's adaptive thinking spent the cap before the verdict was written.
/// Both halves are visible here in one object: the budget the row now carries,
/// and an `effort` nested INSIDE `output_config`, which is the only place the API
/// reads it.
#[test]
fn a_composed_read_request_carries_the_budget_and_the_effort_the_rows_set() {
    let read = PracticeReadParams::for_test();
    let ParamValue::Set(max_tokens) = read_task_spec(&read).max_tokens else {
        panic!("a read always names its own ceiling");
    };

    let body = build_request_body(
        Some("system"),
        "user",
        "claude-opus-5",
        max_tokens,
        // Omitted on the wire: the model row's `temperature_mode` is `omit`.
        None,
        read.effort,
    );

    assert_eq!(body["max_tokens"], 4096);
    assert_eq!(
        body["output_config"]["effort"], "low",
        "effort nests inside output_config — a top-level key is silently ignored"
    );
}

/// `absent` composes a request with no `output_config` at all.
///
/// The pre-2026-09-17 behaviour, still reachable from the Settings page without a
/// deploy — and distinguishable from `high`, which is what the API would then
/// apply on its own.
#[test]
fn an_absent_effort_composes_a_request_with_no_output_config() {
    let mut read = PracticeReadParams::for_test();
    read.effort = None;

    let body = build_request_body(
        Some("system"),
        "user",
        "claude-opus-5",
        4096,
        None,
        read.effort,
    );

    assert!(
        body.get("output_config").is_none(),
        "no key at all is a different request from one naming a level: {body}"
    );
}
