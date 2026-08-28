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
//! So `LLM_RETRY_MAX` defaults to `0`: an LLM-call failure marks the step failed
//! immediately and waits for a human to click Re-process.
//!
//! ## What this loop actually retries, after the amendment
//!
//! One thing, and only one: a **pre-generation rejection** — HTTP 429
//! (`rate_limit_error`) or HTTP 529 (`overloaded_error`). Those are refusals at
//! the front door. The model never started, nothing was billed, and retrying
//! costs wall-clock and nothing else, which is the entire reason they are
//! exempt from the zero. They are bounded by
//! [`LlmRetryPolicy::rate_limit_max_retries`] (`LLM_RATE_LIMIT_RETRY_MAX`,
//! default 5) and wait for whatever the provider's `retry-after` asked for,
//! falling back to an exponential schedule when it asked for nothing.
//!
//! **Everything else returns on the first failure.** That includes an
//! `overloaded_error` arriving as an event inside an already-open stream:
//! generation may have started, so it may already have billed, and it is not
//! this loop's call to spend that again. Those failures travel up to the Restate
//! classifier in `pipeline::workflow_steps::llm_extract_classify`, where
//! `LLM_RETRY_MAX` decides whether the ENGINE may re-invoke the step.
//!
//! That division is why [`LlmRetryPolicy::max_retries`] is deliberately not
//! consulted below. Retrying a paid failure in-process would hide the second
//! call inside a step that reported one attempt; the re-invocation decision is
//! the engine's, and it is visible in the Restate journal when it happens.
//!
//! Note what neither cap governs: truncation stays TERMINAL at any value (see
//! [`crate::pipeline::truncation`]), because no number of retries against the
//! same `max_tokens` ceiling changes the outcome.

use colossus_extract::{LlmProvider, LlmResponse, PipelineError};

use crate::domain::llm_params::ResolvedLlmParams;
use crate::domain::llm_provider_ext::LlmProviderExt;
use crate::llm_retry_policy::{self, LlmRetryPolicy};

/// Call the LLM provider with rate-limit-aware retry.
///
/// On `PipelineError::RateLimited` — which the provider bridge raises ONLY for a
/// pre-generation rejection — waits and retries up to
/// `policy.rate_limit_max_retries` times. Any other error returns immediately.
///
/// `policy` comes from the caller's `AppContext` / `AppConfig`, both filled by
/// the one startup reader. It is threaded rather than read here so the pipeline
/// path and the service paths cannot disagree, and so no call site can quietly
/// opt out of it.
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
    policy: LlmRetryPolicy,
) -> Result<LlmResponse, PipelineError> {
    retry_rate_limited(chunk_idx, chunk_total, policy, || async {
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
    policy: LlmRetryPolicy,
) -> Result<LlmResponse, PipelineError> {
    retry_rate_limited(chunk_idx, chunk_total, policy, || async {
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
/// dispatch one call; this owns the bounded-retry-on-rejection policy so the two
/// cannot drift — one definition, no duplication.
///
/// ## What it retries, and what it refuses to
///
/// `PipelineError::RateLimited` is raised by the provider bridge for exactly one
/// thing: an HTTP 429 or 529 recognised from the response STATUS, before any of
/// the body was read (see
/// [`crate::pipeline::anthropic_transport::classify_status`]). Nothing that
/// happens after the stream opens can produce it. So this arm retries free
/// failures only, and `policy.max_retries` is deliberately never consulted here
/// — see the module doc.
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
    policy: LlmRetryPolicy,
    mut call: F,
) -> Result<LlmResponse, PipelineError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<LlmResponse, PipelineError>>,
{
    let budget = policy.rate_limit_max_retries;
    let mut attempt = 0u32;
    loop {
        match call().await {
            Ok(response) => return Ok(response),
            Err(PipelineError::RateLimited { retry_after_secs }) => {
                attempt += 1;
                if attempt > budget {
                    return Err(rejection_exhausted(chunk_idx, chunk_total, budget));
                }

                // The ONE reader of the no-advice sentinel: everything below
                // sees an honest `Option` again.
                let advised = llm_retry_policy::advice_to_header(retry_after_secs);
                let wait = llm_retry_policy::wait_before_retry(retry_after_secs, attempt);
                tracing::warn!(
                    chunk = chunk_idx,
                    chunk_total,
                    attempt,
                    budget,
                    advised_secs = ?advised,
                    wait_secs = wait.as_secs(),
                    "Rejected before generation (429/529) — nothing billed; waiting, then \
                     retrying on the LLM_RATE_LIMIT_RETRY_MAX budget"
                );

                // Single sleep, no per-second cancel polling. The legacy Worker's
                // `cancel_watcher` race in `colossus-pipeline/src/worker/executor.rs`
                // still cancels the whole step future at the `tokio::select!`, so
                // mid-sleep cancellation still works at the step level — granularity
                // drops from ~1s to ~the wait. The Restate path kills the awaiting
                // future directly via SDK abort.
                tokio::time::sleep(wait).await;
                // Loop continues — retry the call.
            }
            // Everything else, including a mid-stream `overloaded_error`: the
            // call may already have billed, so this loop does not spend again.
            // `LLM_RETRY_MAX` decides what happens next, at the Restate
            // classifier.
            Err(other) => return Err(other),
        }
    }
}

/// The failure produced when the rejection budget runs out.
///
/// Deliberately a plain `LlmProvider` error, so it reaches the step classifier
/// as an ordinary `LlmCallFailed` and is TERMINAL at the shipped
/// `LLM_RETRY_MAX=0` — a request that was refused five times running is no
/// longer a free transient, and the run should stop and wait for a human rather
/// than have the engine start the whole step again.
///
/// The message names the budget and the key, because an operator reading
/// `pipeline_jobs.error` should not have to go and find out what the cap was.
fn rejection_exhausted(chunk_idx: usize, chunk_total: usize, budget: u32) -> PipelineError {
    // Two distinct states, two distinct messages (Standing Rule 1). "The policy
    // forbade retrying at all" and "the policy allowed N and all N were refused"
    // are different operational situations — the first is a deliberate
    // configuration, the second is a provider that would not take the request.
    let reason = if budget == 0 {
        "rejected before generation (HTTP 429/529), and LLM_RATE_LIMIT_RETRY_MAX is 0 — \
         NO retry was attempted. The step fails now and waits for a human to click \
         Re-process"
            .to_string()
    } else {
        format!(
            "still rejected before generation (HTTP 429/529) after {budget} retries — the \
             LLM_RATE_LIMIT_RETRY_MAX budget is exhausted. The step fails now and waits for \
             a human to click Re-process"
        )
    };
    tracing::error!(
        chunk = chunk_idx,
        chunk_total,
        budget,
        "LLM call abandoned: {reason}"
    );
    PipelineError::LlmProvider(format!("chunk {}/{}: {reason}", chunk_idx + 1, chunk_total,))
}

#[cfg(test)]
#[path = "llm_retry_tests.rs"]
mod tests;
