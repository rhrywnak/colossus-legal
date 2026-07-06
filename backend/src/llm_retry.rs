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
    let mut attempt = 0u32;
    loop {
        let result = match system {
            Some(s) => provider.invoke_with_system(s, prompt, max_tokens).await,
            None => provider.invoke(prompt, max_tokens).await,
        };
        match result {
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

                // Single sleep, no per-second cancel polling. The
                // legacy Worker's `cancel_watcher` race in
                // `colossus-pipeline/src/worker/executor.rs` still
                // cancels the whole step future at the
                // `tokio::select!`, so mid-sleep cancellation still
                // works at the step level — granularity drops from
                // ~1s to ~retry_after_secs. The Restate path kills
                // the awaiting future directly via SDK abort.
                tokio::time::sleep(Duration::from_secs(retry_after_secs)).await;
                // Loop continues — retry the call
            }
            Err(other) => return Err(other),
        }
    }
}
