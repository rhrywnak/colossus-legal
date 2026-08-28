//! Unit tests for the retry loop: which failures earn a retry, how long each
//! wait is, and which ones stop at a single call.
//!
//! Lives in a sibling file (rather than a `mod tests { ... }` block inside
//! `llm_retry.rs`) so the runtime file stays under the 300-line module-size
//! budget — the same idiom `pipeline/registry.rs` uses for `registry_tests.rs`.
//!
//! No live API calls. Every provider here is a stub, and the timing assertions
//! run on tokio's virtual clock, so a 17-second wait costs microseconds and no
//! tokens.

use super::*;
use crate::domain::llm_params::ResolvedLlmParams;
use async_trait::async_trait;
use std::sync::Mutex;
use tokio::time::Duration;

/// Records the (system, max_tokens) of the last call so a test can assert
/// which dispatch branch `call_with_rate_limit_retry_params` took. Never a
/// network client — every method returns canned text.
#[derive(Default)]
struct RecordingProvider {
    last: Mutex<Option<(Option<String>, u32)>>,
}

#[async_trait]
impl LlmProvider for RecordingProvider {
    async fn invoke(&self, _prompt: &str, max_tokens: u32) -> Result<LlmResponse, PipelineError> {
        *self.last.lock().expect("test mutex") = Some((None, max_tokens));
        Ok(LlmResponse {
            text: "ok".into(),
            input_tokens: None,
            output_tokens: None,
        })
    }
    async fn invoke_with_system(
        &self,
        system: &str,
        _prompt: &str,
        max_tokens: u32,
    ) -> Result<LlmResponse, PipelineError> {
        *self.last.lock().expect("test mutex") = Some((Some(system.to_string()), max_tokens));
        Ok(LlmResponse {
            text: "ok".into(),
            input_tokens: None,
            output_tokens: None,
        })
    }
    fn provider_name(&self) -> &str {
        "recording"
    }
    fn model_name(&self) -> &str {
        "recording-model"
    }
    fn cost_per_input_token(&self) -> Option<f64> {
        None
    }
    fn cost_per_output_token(&self) -> Option<f64> {
        None
    }
    fn supports_structured_output(&self) -> bool {
        false
    }
}

fn params() -> ResolvedLlmParams {
    ResolvedLlmParams {
        temperature: Some(0.0),
        timeout_secs: 600,
        max_tokens: 512,
    }
}

/// A provider that always fails the same way, counting how many times it
/// was asked.
///
/// `failure` is a factory rather than a stored error because `PipelineError`
/// is not `Clone` — every attempt needs a freshly built one.
struct AlwaysFails {
    calls: Mutex<u32>,
    failure: fn() -> PipelineError,
}

impl AlwaysFails {
    fn new(failure: fn() -> PipelineError) -> Self {
        Self {
            calls: Mutex::new(0),
            failure,
        }
    }

    fn calls(&self) -> u32 {
        *self.calls.lock().expect("test mutex")
    }
}

#[async_trait]
impl LlmProvider for AlwaysFails {
    async fn invoke(&self, _prompt: &str, _max: u32) -> Result<LlmResponse, PipelineError> {
        *self.calls.lock().expect("test mutex") += 1;
        Err((self.failure)())
    }
    async fn invoke_with_system(
        &self,
        _system: &str,
        prompt: &str,
        max: u32,
    ) -> Result<LlmResponse, PipelineError> {
        self.invoke(prompt, max).await
    }
    fn provider_name(&self) -> &str {
        "always-fails"
    }
    fn model_name(&self) -> &str {
        "always-fails-model"
    }
    fn cost_per_input_token(&self) -> Option<f64> {
        None
    }
    fn cost_per_output_token(&self) -> Option<f64> {
        None
    }
    fn supports_structured_output(&self) -> bool {
        false
    }
}

/// A rejection carrying the provider's own `retry-after` of 17s.
///
/// This is what the bridge produces for a pre-generation HTTP 429 that
/// included the header.
fn rejected_with_advice() -> PipelineError {
    PipelineError::RateLimited {
        retry_after_secs: llm_retry_policy::advice_from_header(Some(17)),
    }
}

/// A rejection with no `retry-after` — the usual shape of an HTTP 529
/// `overloaded_error`, which falls back to the backoff schedule.
fn rejected_without_advice() -> PipelineError {
    PipelineError::RateLimited {
        retry_after_secs: llm_retry_policy::advice_from_header(None),
    }
}

/// A failure detected AFTER the stream opened — the shape an
/// `overloaded_error` SSE event takes by the time it reaches this loop.
/// Generation may have started, so it may already have billed.
fn mid_stream_overloaded() -> PipelineError {
    PipelineError::LlmProvider(
        "model claude-opus-5: provider sent an error event mid-stream: \
         overloaded_error: Overloaded"
            .to_string(),
    )
}

fn policy(max_retries: u32, rate_limit_max_retries: u32) -> LlmRetryPolicy {
    LlmRetryPolicy {
        max_retries,
        rate_limit_max_retries,
    }
}

// ── The exemption: pre-generation rejections are free, so they retry ──

#[tokio::test(start_paused = true)]
async fn a_429_is_retried_on_its_own_budget_even_though_llm_retry_max_is_zero() {
    // The amendment in one test. `LLM_RETRY_MAX = 0` is the shipped policy
    // and it does NOT suppress this: the request was refused at the front
    // door, nothing was billed, and asking again costs only wall-clock.
    let p = AlwaysFails::new(rejected_with_advice);
    let err = call_with_rate_limit_retry(&p, None, "user", 512, 0, 1, policy(0, 5))
        .await
        .expect_err("still rejected after the budget is spent");
    assert_eq!(
        p.calls(),
        6,
        "one initial call plus LLM_RATE_LIMIT_RETRY_MAX=5 retries"
    );
    let text = err.to_string();
    assert!(
        text.contains("LLM_RATE_LIMIT_RETRY_MAX"),
        "the failure must name the budget that ran out, not the general cap: {text}"
    );
    assert!(
        text.contains("Re-process"),
        "at exhaustion it fails like any other LLM failure: {text}"
    );
}

#[tokio::test(start_paused = true)]
async fn a_529_follows_exactly_the_same_path_as_a_429() {
    // Both statuses are refusals before generation, so the transport maps
    // them onto one variant and this loop cannot tell them apart — which is
    // the intent. The distinction survives in the engine's log line.
    let p = AlwaysFails::new(rejected_without_advice);
    let err = call_with_rate_limit_retry(&p, None, "user", 512, 0, 1, policy(0, 5))
        .await
        .expect_err("still overloaded after the budget is spent");
    assert_eq!(p.calls(), 6);
    assert!(err.to_string().contains("429/529"));
}

#[tokio::test(start_paused = true)]
async fn the_providers_retry_after_is_honoured_rather_than_our_backoff() {
    // Domain note: `retry-after` states when the token bucket will have room
    // for THIS request. Waiting less guarantees another rejection. The
    // virtual clock lets us assert the real elapsed wait — 5 retries × 17s —
    // in microseconds of wall clock.
    let p = AlwaysFails::new(rejected_with_advice);
    let started = tokio::time::Instant::now();
    let _ = call_with_rate_limit_retry(&p, None, "user", 512, 0, 1, policy(0, 5)).await;
    assert_eq!(
        started.elapsed(),
        Duration::from_secs(17 * 5),
        "each of the five waits must be the provider's own 17s"
    );
}

#[tokio::test(start_paused = true)]
async fn an_unadvised_rejection_falls_back_to_the_doubling_schedule() {
    // 1 + 2 + 4 + 8 + 16 = 31s across five retries. The fallback exists
    // because a 529 usually carries no header at all, and waiting a flat
    // minute for the first one would be a needless stall.
    let p = AlwaysFails::new(rejected_without_advice);
    let started = tokio::time::Instant::now();
    let _ = call_with_rate_limit_retry(&p, None, "user", 512, 0, 1, policy(0, 5)).await;
    assert_eq!(started.elapsed(), Duration::from_secs(31));
}

#[tokio::test(start_paused = true)]
async fn the_rejection_budget_is_configurable_without_a_code_change() {
    // LLM_RATE_LIMIT_RETRY_MAX is a parameter, so a deployment can widen or
    // close the exemption entirely.
    for budget in [0u32, 1, 3] {
        let p = AlwaysFails::new(rejected_without_advice);
        let _ = call_with_rate_limit_retry(&p, None, "user", 512, 0, 1, policy(0, budget)).await;
        assert_eq!(
            p.calls(),
            budget + 1,
            "budget {budget} must produce exactly {} calls",
            budget + 1
        );
    }
}

// ── The zero: everything that may have billed stops at one call ──

#[tokio::test(start_paused = true)]
// The capital is the point: the same provider condition IS retried when it
// arrives before generation, and is not once the stream has opened.
#[allow(non_snake_case)]
async fn a_mid_stream_overloaded_error_is_NOT_retried_at_the_zero_cap() {
    // The boundary the amendment turns on. The SAME provider condition —
    // "overloaded" — is free before generation and expensive after it. By
    // the time it arrives as a stream event the tokens may already be
    // billed, so this loop calls exactly once and lets the Restate
    // classifier decide (TERMINAL at LLM_RETRY_MAX=0).
    let p = AlwaysFails::new(mid_stream_overloaded);
    let err = call_with_rate_limit_retry(&p, None, "user", 512, 0, 1, policy(0, 5))
        .await
        .expect_err("the call failed");
    assert_eq!(
        p.calls(),
        1,
        "a mid-stream error must NOT enter the rate-limit retry path"
    );
    assert!(
        err.to_string().contains("overloaded_error"),
        "the original failure must propagate unchanged, not be reworded: {err}"
    );
    assert!(
        !err.to_string().contains("LLM_RATE_LIMIT_RETRY_MAX"),
        "it must not be reported as a rejection-budget exhaustion: {err}"
    );
}

#[tokio::test(start_paused = true)]
async fn a_generic_call_failure_is_never_retried_whatever_the_caps_say() {
    // A raised LLM_RETRY_MAX hands the re-invocation decision to Restate; it
    // does NOT start an in-process retry loop, because a second call hidden
    // inside a step that reported one attempt is exactly the invisible spend
    // the 2026-08-28 ruling was about.
    for caps in [policy(0, 5), policy(3, 5)] {
        let p = AlwaysFails::new(|| PipelineError::LlmProvider("connection reset".into()));
        let _ = call_with_rate_limit_retry(&p, None, "user", 512, 0, 1, caps).await;
        assert_eq!(p.calls(), 1, "caps {caps:?} must still produce one call");
    }
}

#[tokio::test(start_paused = true)]
async fn a_truncation_is_returned_on_the_first_call_and_never_enters_the_budget() {
    // Ruled 2026-08-27 and untouched by the rate-limit exemption. A truncation
    // arrives as the gate's `LlmProvider` message, not as `RateLimited`, so it
    // structurally cannot reach the retry arm — and if it ever did, the retry
    // would run against the same max_tokens ceiling and be cut off in the same
    // place, which is the definition of terminal.
    use crate::pipeline::truncation;

    let p = AlwaysFails::new(|| {
        PipelineError::LlmProvider(truncation::truncation_message(&truncation::CallShape {
            stop_reason: Some(truncation::STOP_REASON_MAX_TOKENS),
            output_tokens: Some(64_000),
            configured_max_tokens: 64_000,
            model: "claude-opus-5",
        }))
    });
    let err = call_with_rate_limit_retry(&p, None, "user", 512, 0, 1, policy(0, 5))
        .await
        .expect_err("a truncated response fails the call");
    assert_eq!(
        p.calls(),
        1,
        "a truncation must not be retried on any budget"
    );
    assert!(
        truncation::is_truncation_failure(&err),
        "the gate's own failure must propagate unchanged: {err}"
    );
}

#[tokio::test(start_paused = true)]
// The capital is the point: this is the assertion that stops the engine from
// re-running a step whose free retries are already spent.
#[allow(non_snake_case)]
async fn an_exhausted_rejection_budget_classifies_TERMINAL_at_the_zero_cap() {
    // The end of the chain the amendment promises: the free retries happen in
    // this loop, and when the loop gives up the step fails exactly like any
    // other LLM failure — TERMINAL at `LLM_RETRY_MAX=0`, waiting for a human.
    // The engine must not then re-run the whole step and spend the budget again.
    use crate::pipeline::steps::llm_extract::LlmExtractError;
    use crate::pipeline::workflow_steps::llm_extract_classify::classify_llm_extract_error;

    let p = AlwaysFails::new(rejected_without_advice);
    let err = call_with_rate_limit_retry(&p, None, "user", 512, 0, 1, policy(0, 2))
        .await
        .expect_err("the budget is spent");

    let typed = LlmExtractError::from_provider_failure(err);
    assert!(
        matches!(typed, LlmExtractError::LlmCallFailed { .. }),
        "an exhausted budget is an ordinary call failure, not a special case"
    );
    let classified = classify_llm_extract_error("doc-x", "llm_extract_pass1", &typed, 0);
    let message = format!(
        "{}",
        <restate_sdk::errors::HandlerError as AsRef<dyn std::error::Error>>::as_ref(&classified)
    );
    assert!(
        message.starts_with("Terminal error"),
        "an exhausted rejection budget must be TERMINAL at LLM_RETRY_MAX=0: {message}"
    );
}

#[test]
fn the_shipped_defaults_are_zero_and_five() {
    // Pinned rather than assumed. The zero is load-bearing (two incidents in
    // the week of 2026-08-24 were paid for by automatic re-invocation); the
    // five is only defensible while the exemption stays limited to failures
    // that bill nothing.
    let d = LlmRetryPolicy::default();
    assert_eq!(d.max_retries, 0);
    assert_eq!(d.rate_limit_max_retries, 5);
}

#[tokio::test]
async fn params_wrapper_with_system_routes_through_the_system_seam() {
    let p = RecordingProvider::default();
    call_with_rate_limit_retry_params(
        &p,
        Some("SYS"),
        "user",
        &params(),
        0,
        1,
        LlmRetryPolicy::default(),
    )
    .await
    .expect("stub never errors");
    let (system, max_tokens) = p.last.lock().unwrap().clone().expect("a call recorded");
    assert_eq!(system.as_deref(), Some("SYS"), "system prompt must survive");
    assert_eq!(max_tokens, 512, "params.max_tokens must be threaded");
}

#[tokio::test]
async fn params_wrapper_without_system_routes_through_the_plain_seam() {
    let p = RecordingProvider::default();
    call_with_rate_limit_retry_params(&p, None, "user", &params(), 0, 1, LlmRetryPolicy::default())
        .await
        .expect("stub never errors");
    let (system, max_tokens) = p.last.lock().unwrap().clone().expect("a call recorded");
    assert_eq!(system, None, "no system prompt on the plain path");
    assert_eq!(max_tokens, 512);
}
