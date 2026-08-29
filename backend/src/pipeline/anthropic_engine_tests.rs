//! Constructor, request-shape, and error-mapping tests for the streaming
//! Anthropic adapter.
//!
//! ## No live API calls, by ruling (2026-08-28)
//!
//! Nothing here opens a socket. The request body is asserted as a value; the
//! response side is asserted in `anthropic_stream_tests` and
//! `anthropic_transport_tests` against transcripts. Live validation happens on a
//! real document run that Roman starts himself.
//!
//! The env-mutating cases live in ONE test function on purpose. `std::env` is
//! process-global and cargo runs `#[test]` functions on a thread pool, so two
//! tests both touching `ANTHROPIC_API_KEY` race — a flake this repo has already
//! paid for once (P1-7). Collapsing them into one sequential function is the
//! structural fix. Tests that use unique env-var names are safe in parallel and
//! stay separate.

use super::*;

// ## Rust Learning: `unsafe fn set_var`
//
// `std::env::set_var` became `unsafe` in Rust 2024 because mutating the process
// environment while other threads read it is a data race. Within a single test
// function we know no other thread is reading our chosen var, so the `unsafe` is
// discharged by isolation, not by static analysis. Each case restores what it
// found so the rest of the binary sees a clean slate.
#[tokio::test]
async fn from_env_and_extract_batch_against_env_state() {
    let prior_api_key = std::env::var(ANTHROPIC_API_KEY_ENV).ok();
    let prior_idle = std::env::var(IDLE_TIMEOUT_SECS_ENV).ok();
    let prior_base = std::env::var(BASE_URL_ENV).ok();

    // Case 1: ANTHROPIC_API_KEY unset → a Configuration error that NAMES the key.
    unsafe {
        std::env::remove_var(ANTHROPIC_API_KEY_ENV);
    }
    match AnthropicStreamingEngine::from_env() {
        Err(ExtractionEngineError::Configuration(msg)) => {
            assert!(
                msg.contains(ANTHROPIC_API_KEY_ENV),
                "the error must name the missing env var, got: {msg}"
            );
        }
        Err(other) => panic!("expected Configuration error, got {other:?}"),
        Ok(_) => panic!("expected an error when the API key is unset"),
    }

    // Case 2: a dummy key is enough to construct — we are not calling the API.
    unsafe {
        std::env::set_var(ANTHROPIC_API_KEY_ENV, "sk-ant-placeholder-for-test");
    }
    let engine =
        AnthropicStreamingEngine::from_env().expect("from_env must succeed with a dummy API key");
    assert_eq!(
        engine.idle_timeout,
        Duration::from_secs(DEFAULT_IDLE_TIMEOUT_SECS),
        "an unset idle timeout must take the documented default"
    );
    assert!(
        engine.messages_url.ends_with(MESSAGES_PATH),
        "the endpoint must be the base URL plus the Messages path, got: {}",
        engine.messages_url
    );

    // Case 3: `extract_batch(&[], 0)` must clamp the zero concurrency value and
    // return an empty Vec rather than hanging on `buffer_unordered(0)`. Folded
    // in here so ANTHROPIC_API_KEY mutations stay single-threaded.
    assert!(engine.extract_batch(&[], 0).await.is_empty());

    // Case 4: a base URL with a trailing slash must not produce a doubled one.
    unsafe {
        std::env::set_var(BASE_URL_ENV, "https://anthropic.example.internal/");
    }
    let engine = AnthropicStreamingEngine::from_env().expect("a base URL override must build");
    assert_eq!(
        engine.messages_url,
        format!("https://anthropic.example.internal{MESSAGES_PATH}")
    );

    // Case 5: a malformed idle timeout does NOT break construction — the
    // warn!-then-default fallback fires, and the default is the SAFE direction
    // (a stall is still detected).
    unsafe {
        std::env::set_var(IDLE_TIMEOUT_SECS_ENV, "two minutes");
    }
    let engine =
        AnthropicStreamingEngine::from_env().expect("an unparseable idle timeout must not panic");
    assert_eq!(
        engine.idle_timeout,
        Duration::from_secs(DEFAULT_IDLE_TIMEOUT_SECS)
    );

    unsafe {
        match prior_api_key {
            Some(v) => std::env::set_var(ANTHROPIC_API_KEY_ENV, v),
            None => std::env::remove_var(ANTHROPIC_API_KEY_ENV),
        }
        match prior_idle {
            Some(v) => std::env::set_var(IDLE_TIMEOUT_SECS_ENV, v),
            None => std::env::remove_var(IDLE_TIMEOUT_SECS_ENV),
        }
        match prior_base {
            Some(v) => std::env::set_var(BASE_URL_ENV, v),
            None => std::env::remove_var(BASE_URL_ENV),
        }
    }
}

#[test]
fn read_secs_env_returns_default_when_unset() {
    const TEST_ENV: &str = "ANTHROPIC_ENGINE_TEST_DEFAULT_FALLBACK_PROBE";
    unsafe {
        std::env::remove_var(TEST_ENV);
    }
    assert_eq!(read_secs_env(TEST_ENV, 42), 42);
}

#[test]
fn read_secs_env_parses_valid_value() {
    const TEST_ENV: &str = "ANTHROPIC_ENGINE_TEST_VALID_PARSE_PROBE";
    unsafe {
        std::env::set_var(TEST_ENV, "123");
    }
    assert_eq!(read_secs_env(TEST_ENV, 42), 123);
    unsafe {
        std::env::remove_var(TEST_ENV);
    }
}

#[test]
fn read_string_env_treats_an_empty_value_as_unset() {
    const TEST_ENV: &str = "ANTHROPIC_ENGINE_TEST_EMPTY_STRING_PROBE";
    unsafe {
        std::env::set_var(TEST_ENV, "   ");
    }
    // A half-finished `.env` edit must not produce an empty base URL, which
    // could never form a valid request.
    assert_eq!(read_string_env(TEST_ENV, "fallback"), "fallback");
    unsafe {
        std::env::remove_var(TEST_ENV);
    }
}

// ── The request body ────────────────────────────────────────────

#[test]
fn every_request_asks_for_a_stream() {
    // The one-line summary of this whole change. A regression that dropped this
    // key would silently reinstate the ~10-minute wall that failed the
    // 36-page transcript on 2026-08-28, and nothing else in the suite would
    // notice.
    let body = build_request_body(None, "prompt", "claude-opus-5", 64_000, None, None);
    assert_eq!(body["stream"], serde_json::json!(true));
    assert_eq!(body["model"], serde_json::json!("claude-opus-5"));
    assert_eq!(body["max_tokens"], serde_json::json!(64_000));
    assert_eq!(body["messages"][0]["role"], serde_json::json!("user"));
    assert_eq!(body["messages"][0]["content"], serde_json::json!("prompt"));
}

#[test]
fn a_system_prompt_uses_the_native_top_level_field() {
    // Pass 2 and the Theme Scan depend on this: concatenating the system prompt
    // into the user turn instead would change what the model is being asked.
    let body = build_request_body(Some("you are a paralegal"), "prompt", "m", 100, None, None);
    assert_eq!(body["system"], serde_json::json!("you are a paralegal"));
}

#[test]
fn an_absent_temperature_omits_the_key_entirely() {
    // Domain note: Claude Opus 4.7 and later REJECT a temperature key rather
    // than ignoring it, so "no temperature" must mean an absent field — not
    // `null`, and not a substituted default.
    let body = build_request_body(None, "prompt", "m", 100, None, None);
    assert!(
        body.get("temperature").is_none(),
        "temperature must be absent, got: {body}"
    );

    let body = build_request_body(None, "prompt", "m", 100, Some(0.0), None);
    assert_eq!(body["temperature"], serde_json::json!(0.0));
}

// ── The effort dial ─────────────────────────────────────────────

#[test]
fn an_extraction_request_carries_output_config_effort() {
    // The 2026-08-28 fix on the wire. `effort` nests inside `output_config`; a
    // top-level `effort` is not a recognised parameter and would be IGNORED,
    // which would look identical in the code and bring the 727-second thinking
    // pass straight back.
    use crate::domain::llm_effort::Effort;

    let body = build_request_body(
        None,
        "prompt",
        "claude-opus-5",
        64_000,
        None,
        Some(Effort::Low),
    );
    assert_eq!(body["output_config"]["effort"], serde_json::json!("low"));
    assert!(
        body.get("effort").is_none(),
        "effort must NOT be top-level — it would be silently ignored: {body}"
    );
}

#[test]
fn a_scan_request_carries_no_output_config_at_all() {
    // The other half of the ruling: absent is a real state. Sending `"high"`
    // explicitly would look identical today and would quietly PIN the scans if
    // Anthropic ever moved the default, which is a quality change nobody asked
    // for made silently on the way past.
    let body = build_request_body(None, "prompt", "claude-opus-5", 8_000, None, None);
    assert!(
        body.get("output_config").is_none(),
        "no effort means no output_config key: {body}"
    );
}

#[test]
fn every_effort_level_reaches_the_wire_as_its_documented_string() {
    // The API rejects anything outside these five, and a rejection arrives as an
    // HTTP 400 in the middle of a paid run.
    use crate::domain::llm_effort::Effort;

    for level in Effort::ALL {
        let body = build_request_body(None, "p", "claude-opus-5", 100, None, Some(level));
        assert_eq!(
            body["output_config"]["effort"],
            serde_json::json!(level.as_wire()),
            "level {level} must reach the wire verbatim"
        );
    }
}

#[test]
fn effort_does_not_disturb_the_rest_of_the_body() {
    // Standing guard on the streaming fix: `output_config` is an addition, and a
    // regression that dropped `stream` while adding it would reinstate the
    // ~10-minute wall from the 2026-08-28 morning incident while fixing the
    // afternoon one.
    use crate::domain::llm_effort::Effort;

    let body = build_request_body(
        Some("sys"),
        "prompt",
        "claude-opus-5",
        64_000,
        Some(0.0),
        Some(Effort::Low),
    );
    assert_eq!(body["stream"], serde_json::json!(true));
    assert_eq!(body["system"], serde_json::json!("sys"));
    assert_eq!(body["temperature"], serde_json::json!(0.0));
    assert_eq!(body["max_tokens"], serde_json::json!(64_000));
}

// ── Error mapping ───────────────────────────────────────────────

#[test]
fn both_rejection_kinds_keep_the_typed_shape_and_the_retry_after() {
    use crate::pipeline::anthropic_transport::RejectionKind;

    // 429 and 529 map onto the one variant the retry loop acts on, because they
    // mean the same thing operationally: refused before generation, nothing
    // billed, free to ask again.
    for kind in [RejectionKind::RateLimited, RejectionKind::Overloaded] {
        let mapped = map_transport_error(
            TransportError::Rejected {
                kind,
                retry_after_secs: Some(30),
            },
            "claude-opus-5",
        );
        match mapped {
            ExtractionEngineError::RateLimited {
                model,
                retry_after_secs,
            } => {
                assert_eq!(model, "claude-opus-5");
                assert_eq!(retry_after_secs, Some(30));
            }
            other => panic!("expected RateLimited for {kind:?}, got {other:?}"),
        }
    }
}

#[test]
fn a_mid_stream_error_event_is_a_call_failure_and_never_a_rejection() {
    // The exemption must not leak. An `overloaded_error` that arrived inside an
    // open stream reaches the engine as a `Stream` error, and it must map to
    // `LlmCallFailed` — the retry loop keys on the OTHER variant, so mapping it
    // here would silently buy free retries for a call that may have billed.
    let mapped = map_transport_error(
        TransportError::Stream(
            crate::pipeline::anthropic_stream::StreamError::ProviderEvent {
                kind: "overloaded_error".to_string(),
                message: "Overloaded".to_string(),
            },
        ),
        "claude-opus-5",
    );
    assert!(
        matches!(mapped, ExtractionEngineError::LlmCallFailed { .. }),
        "a mid-stream error must not become a rate-limit rejection: {mapped:?}"
    );
}

#[test]
fn a_stall_is_a_call_failure_that_names_the_model_and_the_window() {
    let mapped = map_transport_error(
        TransportError::IdleTimeout {
            idle_secs: 120,
            events_seen: 41,
        },
        "claude-opus-5",
    );
    match mapped {
        ExtractionEngineError::LlmCallFailed { model, source } => {
            assert_eq!(model, "claude-opus-5");
            let text = source.to_string();
            assert!(text.contains("120"), "the window must survive: {text}");
            assert!(
                text.contains(IDLE_TIMEOUT_SECS_ENV),
                "the operator must be told which key tunes it: {text}"
            );
        }
        other => panic!("expected LlmCallFailed, got {other:?}"),
    }
}
