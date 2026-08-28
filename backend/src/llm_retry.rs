//! Shared rate-limit-aware LLM retry wrapper.
//!
//! A single, provider-agnostic helper that both the extraction pipeline
//! (`pipeline::steps::llm_extract` / `llm_extract_pass2`) and the Theme Scan
//! service (`services::theme_scan`) call. It lives at the crate root rather than
//! inside `pipeline::steps` so a *service* can reuse it without importing a
//! pipeline step's internals — the retry logic knows nothing about either layer,
//! so neither should own it.
//!
//! ## Why this is a shared util, not duplicated per caller
//!
//! Retry-on-rate-limit is subtle (bounded attempts, honour the server's
//! `retry_after`, propagate every other error immediately). Duplicating it per
//! caller would let the copies drift — one could grow a bug the other lacks.
//! One definition, two callers (no tech debt / no duplication).
//!
//! ## The cap is ZERO by default, and that is the point (ruled 2026-08-28)
//!
//! Automatic retries of LLM calls cost real money and, twice in the week of
//! 2026-08-24, bought nothing: a deterministic failure re-run is a deterministic
//! failure. The truncation gate settled that argument for one failure class
//! (census R-3); the 600s whole-request timeout that killed a healthy
//! 64000-token generation settled it for another — Restate reran the step and
//! spent the same money reaching the same wall.
//!
//! So the cap now defaults to `0`: an LLM-call failure marks the step failed
//! immediately and waits for a human to click Re-process. The number is
//! [`DEFAULT_MAX_RETRIES`], read from `LLM_RETRY_MAX` at startup by
//! [`crate::config::llm_retry_max_from_env`], and it is a PARAMETER of every
//! call here rather than a constant — raising it is a config change, and it must
//! reach both this loop and the Restate terminal-vs-retryable classification in
//! `pipeline::workflow_steps::llm_extract` from the same read.
//!
//! Note what the cap does NOT govern: truncation stays TERMINAL at any value
//! (see [`crate::pipeline::truncation`]), because no number of retries against
//! the same `max_tokens` ceiling changes the outcome.

use tokio::time::Duration;

use colossus_extract::{LlmProvider, LlmResponse, PipelineError};

use crate::domain::llm_params::ResolvedLlmParams;
use crate::domain::llm_provider_ext::LlmProviderExt;

/// Default maximum automatic retry attempts per LLM call on rate-limit (429)
/// errors, when `LLM_RETRY_MAX` is unset.
///
/// ZERO by ruling (2026-08-28) — see the module doc. This is deliberately the
/// safe direction: an operator who has not thought about the key gets no
/// automatic spend, and the failure waits for a human.
///
// CONST: the DEFAULT for an env-var-configured policy, which is exactly what
// Standing Rule 2 asks a default to be — the live value is
// `LLM_RETRY_MAX`, read once at startup. Raising the cap is a config change and
// a restart, with no code change and no rebuild.
pub(crate) const DEFAULT_MAX_RETRIES: u32 = 0;

/// Call the LLM provider with rate-limit-aware retry.
///
/// On `PipelineError::RateLimited`, sleeps exactly `retry_after_secs` and
/// retries, up to `max_retries` times. Any other error returns immediately.
///
/// `max_retries` comes from `LLM_RETRY_MAX` via the caller's `AppContext` /
/// `AppConfig` — it is threaded rather than read here so the pipeline path and
/// the service paths cannot disagree about the policy, and so no call site can
/// quietly opt out of it.
///
/// The `chunk_idx` / `chunk_total` pair is used only for logging.
///
/// When `system` is `Some`, the call routes through
/// [`LlmProvider::invoke_with_system`] so providers with a native
/// system-prompt field (Anthropic Messages API) populate it instead of
/// concatenating system+user into a single prompt.
///
/// ## Rust Learning: `&dyn LlmProvider` is safe to call from many tasks
///
/// The parameter is a shared, immutable trait-object borrow. `LlmProvider` is
/// `Send + Sync + 'static` and the concrete providers hold no per-call interior
/// mutability, and this function keeps all its state (`attempt`) on its own
/// stack — so N concurrent callers each get an independent retry loop over the
/// same shared provider. That is what lets the Theme Scan fan these calls out
/// with `buffer_unordered` while extraction calls them sequentially.
pub(crate) async fn call_with_rate_limit_retry(
    provider: &dyn LlmProvider,
    system: Option<&str>,
    prompt: &str,
    max_tokens: u32,
    chunk_idx: usize,
    chunk_total: usize,
    max_retries: u32,
) -> Result<LlmResponse, PipelineError> {
    retry_rate_limited(chunk_idx, chunk_total, max_retries, || async {
        match system {
            Some(s) => provider.invoke_with_system(s, prompt, max_tokens).await,
            None => provider.invoke(prompt, max_tokens).await,
        }
    })
    .await
}

/// Same rate-limit-aware retry, but dispatching through the params-aware seam
/// ([`LlmProviderExt`]) so a RESOLVED parameter set drives the call.
///
/// The Theme Scan uses this: `system` is `Some(theme_scan_prompt)`, so it routes
/// through `invoke_with_system_and_params` and the judging system prompt SURVIVES
/// (the whole reason the scan judges through a system/user split). Only
/// `params.max_tokens` reaches the wire today — see [`LlmProviderExt`] for why
/// the other resolved fields do not yet (Chunk B seam ceiling).
pub(crate) async fn call_with_rate_limit_retry_params(
    provider: &dyn LlmProvider,
    system: Option<&str>,
    prompt: &str,
    params: &ResolvedLlmParams,
    chunk_idx: usize,
    chunk_total: usize,
    max_retries: u32,
) -> Result<LlmResponse, PipelineError> {
    retry_rate_limited(chunk_idx, chunk_total, max_retries, || async {
        match system {
            Some(s) => {
                provider
                    .invoke_with_system_and_params(s, prompt, params)
                    .await
            }
            None => provider.invoke_with_params(prompt, params).await,
        }
    })
    .await
}

/// The shared retry/backoff loop. Both public wrappers differ ONLY in how they
/// dispatch one call; this owns the bounded-retry-on-429 policy so the two cannot
/// drift — one definition, no duplication.
///
/// ## Rust Learning: a retried async operation as an `FnMut() -> Future`
///
/// The `call` closure is invoked ONCE PER ATTEMPT and must produce a FRESH future
/// each time — a future is consumed by `.await`, so a single future cannot be
/// re-awaited to retry. `FnMut() -> Fut` captures that: each call re-borrows the
/// captured `provider`/`prompt`/`params` and returns a new future. This is the
/// idiomatic way to make a piece of async work retryable without boxing it.
async fn retry_rate_limited<F, Fut>(
    chunk_idx: usize,
    chunk_total: usize,
    max_retries: u32,
    mut call: F,
) -> Result<LlmResponse, PipelineError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<LlmResponse, PipelineError>>,
{
    let mut attempt = 0u32;
    loop {
        match call().await {
            Ok(response) => return Ok(response),
            Err(PipelineError::RateLimited { retry_after_secs }) => {
                attempt += 1;
                if attempt > max_retries {
                    // Two distinct states, two distinct messages (Standing Rule
                    // 1). "The policy forbade retrying" and "the policy allowed
                    // N retries and all N failed" cost very different amounts of
                    // money, and an operator reading `pipeline_jobs.error` must
                    // be able to tell which one they are looking at without
                    // going to find out what LLM_RETRY_MAX was set to.
                    let reason = if max_retries == 0 {
                        "rate limited, and LLM_RETRY_MAX is 0 — NO automatic retry was \
                         attempted. The step fails now and waits for a human to click \
                         Re-process. Raise LLM_RETRY_MAX if automatic retries are wanted"
                            .to_string()
                    } else {
                        format!("exhausted {max_retries} rate-limit retries (LLM_RETRY_MAX)")
                    };
                    tracing::error!(
                        chunk = chunk_idx,
                        chunk_total,
                        max_retries,
                        retry_after_secs,
                        "LLM call abandoned: {reason}"
                    );
                    return Err(PipelineError::LlmProvider(format!(
                        "chunk {}/{}: {reason}",
                        chunk_idx + 1,
                        chunk_total,
                    )));
                }

                tracing::warn!(
                    chunk = chunk_idx,
                    retry_after_secs,
                    attempt,
                    "Rate limited, sleeping before retry"
                );

                // Single sleep, no per-second cancel polling. The legacy Worker's
                // `cancel_watcher` race in `colossus-pipeline/src/worker/executor.rs`
                // still cancels the whole step future at the `tokio::select!`, so
                // mid-sleep cancellation still works at the step level — granularity
                // drops from ~1s to ~retry_after_secs. The Restate path kills the
                // awaiting future directly via SDK abort.
                tokio::time::sleep(Duration::from_secs(retry_after_secs)).await;
                // Loop continues — retry the call.
            }
            Err(other) => return Err(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::llm_params::ResolvedLlmParams;
    use async_trait::async_trait;
    use std::sync::Mutex;

    /// Records the (system, max_tokens) of the last call so a test can assert
    /// which dispatch branch `call_with_rate_limit_retry_params` took. Never a
    /// network client — every method returns canned text.
    #[derive(Default)]
    struct RecordingProvider {
        last: Mutex<Option<(Option<String>, u32)>>,
    }

    #[async_trait]
    impl LlmProvider for RecordingProvider {
        async fn invoke(
            &self,
            _prompt: &str,
            max_tokens: u32,
        ) -> Result<LlmResponse, PipelineError> {
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

    /// Always rate-limited, and counts how many times it was asked.
    ///
    /// `retry_after_secs: 0` so a test that DOES permit retries does not sleep.
    #[derive(Default)]
    struct AlwaysRateLimited {
        calls: Mutex<u32>,
    }

    #[async_trait]
    impl LlmProvider for AlwaysRateLimited {
        async fn invoke(&self, _prompt: &str, _max: u32) -> Result<LlmResponse, PipelineError> {
            *self.calls.lock().expect("test mutex") += 1;
            Err(PipelineError::RateLimited {
                retry_after_secs: 0,
            })
        }
        async fn invoke_with_system(
            &self,
            _system: &str,
            _prompt: &str,
            _max: u32,
        ) -> Result<LlmResponse, PipelineError> {
            self.invoke(_prompt, _max).await
        }
        fn provider_name(&self) -> &str {
            "always-rate-limited"
        }
        fn model_name(&self) -> &str {
            "always-rate-limited-model"
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

    #[tokio::test]
    async fn a_cap_of_zero_calls_the_model_exactly_once() {
        // The shipped default, and the whole point of the 2026-08-28 ruling: the
        // provider is asked ONCE, and the second paid call never happens.
        let p = AlwaysRateLimited::default();
        let err = call_with_rate_limit_retry(&p, None, "user", 512, 0, 1, 0)
            .await
            .expect_err("a rate-limited call with no retries must fail");
        assert_eq!(
            *p.calls.lock().expect("test mutex"),
            1,
            "a cap of zero must not produce a second paid call"
        );
        let text = err.to_string();
        assert!(
            text.contains("LLM_RETRY_MAX is 0"),
            "the failure must say the POLICY stopped it, not that retries ran out: {text}"
        );
        assert!(
            text.contains("Re-process"),
            "the failure must name the human action that resumes the run: {text}"
        );
    }

    #[tokio::test]
    async fn a_raised_cap_takes_effect_without_a_code_change() {
        // The other half of "configurable": the cap is a parameter, so a
        // deployment that sets LLM_RETRY_MAX=2 gets the initial call plus two
        // retries — three paid calls — and a message that says so.
        let p = AlwaysRateLimited::default();
        let err = call_with_rate_limit_retry(&p, None, "user", 512, 0, 1, 2)
            .await
            .expect_err("still rate limited after the retries");
        assert_eq!(
            *p.calls.lock().expect("test mutex"),
            3,
            "one initial call plus two retries"
        );
        let text = err.to_string();
        assert!(
            text.contains("exhausted 2"),
            "an exhausted cap reads differently from a zero cap: {text}"
        );
    }

    #[test]
    fn the_shipped_default_is_zero() {
        // Pinned rather than assumed. If this constant is ever raised, the
        // change should have to argue with a failing test first: the two
        // incidents in the week of 2026-08-24 were both paid for by automatic
        // re-invocation of a deterministic failure.
        assert_eq!(DEFAULT_MAX_RETRIES, 0);
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
            DEFAULT_MAX_RETRIES,
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
        call_with_rate_limit_retry_params(&p, None, "user", &params(), 0, 1, DEFAULT_MAX_RETRIES)
            .await
            .expect("stub never errors");
        let (system, max_tokens) = p.last.lock().unwrap().clone().expect("a call recorded");
        assert_eq!(system, None, "no system prompt on the plain path");
        assert_eq!(max_tokens, 512);
    }
}
