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

use tokio::time::Duration;

use colossus_extract::{LlmProvider, LlmResponse, PipelineError};

use crate::domain::llm_params::ResolvedLlmParams;
use crate::domain::llm_provider_ext::LlmProviderExt;

/// Maximum retry attempts per LLM call on rate-limit (429) errors.
pub(crate) const MAX_RETRIES_PER_CHUNK: u32 = 3;

/// Call the LLM provider with rate-limit-aware retry.
///
/// On `PipelineError::RateLimited`, sleeps exactly `retry_after_secs` and
/// retries. Max [`MAX_RETRIES_PER_CHUNK`] attempts. Any other error returns
/// immediately.
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
) -> Result<LlmResponse, PipelineError> {
    retry_rate_limited(chunk_idx, chunk_total, || async {
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
) -> Result<LlmResponse, PipelineError> {
    retry_rate_limited(chunk_idx, chunk_total, || async {
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
                if attempt > MAX_RETRIES_PER_CHUNK {
                    return Err(PipelineError::LlmProvider(format!(
                        "chunk {}/{}: exhausted {} rate-limit retries",
                        chunk_idx + 1,
                        chunk_total,
                        MAX_RETRIES_PER_CHUNK
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

    #[tokio::test]
    async fn params_wrapper_with_system_routes_through_the_system_seam() {
        let p = RecordingProvider::default();
        call_with_rate_limit_retry_params(&p, Some("SYS"), "user", &params(), 0, 1)
            .await
            .expect("stub never errors");
        let (system, max_tokens) = p.last.lock().unwrap().clone().expect("a call recorded");
        assert_eq!(system.as_deref(), Some("SYS"), "system prompt must survive");
        assert_eq!(max_tokens, 512, "params.max_tokens must be threaded");
    }

    #[tokio::test]
    async fn params_wrapper_without_system_routes_through_the_plain_seam() {
        let p = RecordingProvider::default();
        call_with_rate_limit_retry_params(&p, None, "user", &params(), 0, 1)
            .await
            .expect("stub never errors");
        let (system, max_tokens) = p.last.lock().unwrap().clone().expect("a call recorded");
        assert_eq!(system, None, "no system prompt on the plain path");
        assert_eq!(max_tokens, 512);
    }
}
