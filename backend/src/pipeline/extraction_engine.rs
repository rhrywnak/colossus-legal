//! ExtractionEngine — the colossus-legal-facing facade over LLM operations.
//!
//! ## Design requirement R4 — thin adapter over Rig
//!
//! Domain code in this repo must never touch `rig-core` directly. Every LLM
//! operation goes through this trait. If Rig's API changes (and `rig-core`
//! had several breaking releases in the year before this Phase 1 work
//! landed), only the adapter implementation changes — the domain code
//! stays put.
//!
//! When `rig-core 0.33` was wired straight into the `colossus-rag` crate,
//! the upgrade to 0.36 forced a coordinated bump across every consumer.
//! With this trait the colossus-legal backend depends on `ExtractionEngine`,
//! not on Rig types, and the adapter is the only file that has to track
//! Rig's API surface.
//!
//! ## Relationship to `LlmProvider` in `colossus-extract`
//!
//! `LlmProvider` (from the `colossus-extract` git dep) is the *legacy*
//! interface used by the pre-Restate pipeline. It speaks `reqwest` directly
//! and predates the Rig integration. During Phase 1 both interfaces coexist:
//!
//! - Legacy `LlmExtract` / `LlmExtractPass2` steps → `LlmProvider`
//! - Restate-driven workflow steps → `ExtractionEngine` (this trait)
//!
//! Phase 4 removes `LlmProvider` and `colossus-rag` together; only
//! `ExtractionEngine` remains.

use std::time::Duration;

use async_trait::async_trait;

/// Single LLM call result returned by [`ExtractionEngine::extract`].
///
/// Wider than `colossus_extract::LlmResponse` — adds `request_id`
/// (Rig surfaces the provider's request id for trace correlation) and
/// `duration` (measured at the adapter boundary so callers record latency
/// in `pipeline_events` without re-measuring).
///
/// Tokens widen from `u32` to `u64`. Large extraction runs that aggregate
/// many chunk-level token counts can plausibly exceed `u32::MAX` (~4.3B);
/// `u64` future-proofs the aggregation paths without rippling type changes.
///
/// ## Rust Learning: Eager common-trait derives (C-COMMON-TRAITS)
///
/// - `Debug` is required by the Rust API Guidelines for every public type
///   and enables `{:?}` formatting in error messages and `tracing` events.
/// - `Clone` is added because callers commonly need to both record token
///   counts (move them into a `pipeline_events` row) and parse the response
///   text (consume the `String`); cloning once at the boundary is simpler
///   than restructuring the call sites to thread ownership.
#[derive(Debug, Clone)]
pub struct LlmCallResult {
    /// Raw text returned by the model. The caller parses it (e.g.
    /// JSON-decoding into an extraction result struct).
    pub response_text: String,

    /// Input tokens consumed, if the provider reported them.
    ///
    /// `None` means the provider did not return usage metadata —
    /// distinguishable from `Some(0)`, which would mean "the provider
    /// said zero" (Rule 1: every distinct state has a distinct value).
    pub input_tokens: Option<u64>,

    /// Output tokens produced, if the provider reported them.
    pub output_tokens: Option<u64>,

    /// Provider-supplied request id (Anthropic's `x-request-id` header,
    /// OpenAI's `id` field). Used to correlate a `pipeline_events` row
    /// with the provider's own logs when a request goes wrong.
    pub request_id: Option<String>,

    /// Wall-clock duration of the call, measured at the adapter boundary
    /// (between sending the request and parsing the response). Recorded
    /// in `pipeline_events` for latency observability.
    pub duration: Duration,
}

/// One item in a batch passed to [`ExtractionEngine::extract_batch`].
///
/// Carries everything `extract` needs as owned data so the batch helper
/// can move items into spawned futures or `buffer_unordered` streams
/// without lifetime gymnastics. The slight allocation cost (one `String`
/// per field per item) is negligible next to the LLM round-trip.
#[derive(Debug, Clone)]
pub struct BatchExtractionItem {
    /// Optional system prompt. `None` sends only the user prompt; `Some`
    /// routes through the provider's native system-prompt field (e.g.
    /// Anthropic Messages API's `system` parameter).
    pub system_prompt: Option<String>,

    /// Required user prompt body.
    pub user_prompt: String,

    /// Model identifier (e.g. `"claude-sonnet-4-6"`). Per-item so a
    /// single engine instance can fan out to multiple models in one
    /// batch — useful when extraction and synthesis pick different
    /// Anthropic models in the same workflow.
    pub model: String,

    /// Hard cap on output tokens for this item.
    pub max_tokens: u32,

    /// Sampling temperature.
    ///
    /// `None` omits the field from the API request entirely (required
    /// for Claude Opus 4.7, which returns HTTP 400 if `temperature`,
    /// `top_p`, or `top_k` are set to any non-default value).
    /// `Some(0.0)` gives deterministic output for extraction.
    /// `Some(t > 0)` for synthesis or natural-variation chat workloads.
    pub temperature: Option<f64>,
}

/// Typed error from any [`ExtractionEngine`] method.
///
/// Distinct from `colossus_extract::PipelineError` — that type belongs
/// to the legacy `LlmProvider` path and carries baggage (entity-resolution
/// variants, splitter errors) the new pipeline does not need.
///
/// ## Rust Learning: `Box<dyn Error + Send + Sync>` as a wrapped source
///
/// The variant `LlmCallFailed::source` is a trait object, not a concrete
/// type. That is what lets a Rig-backed adapter wrap any Rig error
/// without leaking Rig into the trait surface. If we said
/// `source: rig_core::Error`, every consumer of `ExtractionEngineError`
/// would compile-depend on `rig-core` — and R4 (thin adapter) would be
/// broken at the type level.
///
/// `Send + Sync` are required so the error can cross `await` points and
/// be reported from a different task; `'static` is implicit on
/// `Box<dyn Trait>` and on `dyn Error` without an explicit lifetime.
#[derive(Debug, thiserror::Error)]
pub enum ExtractionEngineError {
    /// The underlying LLM call failed (network, parse, transient 5xx,
    /// or a permanent 4xx other than 429).
    ///
    /// `model` is the model identifier that was being called — included
    /// in the message so the operator can see which model misbehaved
    /// without consulting the call site (Rule 1: failures are observable).
    #[error("LLM call failed for model {model}: {source}")]
    LlmCallFailed {
        /// Model identifier that was being called.
        model: String,
        /// Underlying error from the adapter implementation.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Provider returned HTTP 429 (or equivalent). `retry_after_secs`
    /// carries the provider's own `retry-after` value when present;
    /// the orchestrator must wait at least that long before retrying.
    ///
    /// `None` means the provider did not include a retry-after header —
    /// distinguishable from `Some(0)` (which would mean "retry
    /// immediately"). The orchestrator picks a default in the `None`
    /// case.
    ///
    /// `model` is included for the same reason it appears on
    /// [`Self::LlmCallFailed`]: a single engine can fan out to multiple
    /// models in one `extract_batch` call, and the operator needs to
    /// know whose quota was exhausted without re-deriving it from
    /// surrounding log context.
    #[error("Rate limited by model {model}, retry_after_secs={retry_after_secs:?}")]
    RateLimited {
        /// Model identifier whose quota was hit.
        model: String,
        /// Seconds the provider asked us to wait, if it told us.
        retry_after_secs: Option<u64>,
    },

    /// The provider was constructed with invalid configuration
    /// (missing API key, unknown model, malformed endpoint URL).
    ///
    /// Distinct from `LlmCallFailed`: configuration errors are caught
    /// at engine-construction or call-dispatch time and never imply a
    /// retry is appropriate.
    #[error("Provider configuration error: {0}")]
    Configuration(String),
}

/// The colossus-legal-facing facade over LLM operations.
///
/// Implementors wrap a concrete LLM client (Rig 0.36 in the Phase 1
/// implementation; later phases may add provider-specific adapters).
/// Domain code holds `Arc<dyn ExtractionEngine>` and never imports
/// anything from `rig-core`.
///
/// ## Rust Learning: `Send + Sync + 'static` bounds on a trait
///
/// `Arc<dyn ExtractionEngine>` is the canonical way to share one engine
/// across many concurrent extraction tasks. For that to compile, the
/// trait must promise every implementing type satisfies all three bounds:
///
/// - `Send` — the trait object can be moved between threads, which
///   `tokio::spawn` relies on.
/// - `Sync` — multiple tasks can hold shared references to the same
///   engine simultaneously.
/// - `'static` — the engine owns all of its data; nothing borrowed
///   from a shorter scope.
///
/// The same bounds appear on `colossus_extract::LlmProvider` for the
/// same reason.
///
/// ## Rust Learning: `#[async_trait]`
///
/// Native async-in-trait stabilized in Rust 1.75 but does not yet
/// support `dyn Trait` (object safety). `#[async_trait]` desugars each
/// `async fn` to `fn(…) -> Pin<Box<dyn Future + Send + '_>>`, which
/// IS object-safe — at the cost of one heap allocation per call. The
/// trade-off is identical to `LlmProvider`'s and is the established
/// pattern in this repo.
#[async_trait]
pub trait ExtractionEngine: Send + Sync + 'static {
    /// Perform a single LLM call.
    ///
    /// `system_prompt` is `Option<&str>`: providers with native
    /// system-prompt fields (Anthropic Messages API, OpenAI Chat
    /// Completions) populate them when `Some`; `None` sends only the
    /// user prompt. Adapter implementations are responsible for the
    /// system-vs-user distinction at the API layer.
    ///
    /// `temperature` follows the omit-when-`None` convention documented
    /// on [`BatchExtractionItem::temperature`].
    ///
    /// # Errors
    ///
    /// - [`ExtractionEngineError::RateLimited`] on HTTP 429 — the caller
    ///   must back off at least `retry_after_secs` seconds.
    /// - [`ExtractionEngineError::Configuration`] for permanent setup
    ///   failures (invalid API key, unknown model, malformed endpoint).
    ///   Do NOT retry.
    /// - [`ExtractionEngineError::LlmCallFailed`] for everything else
    ///   (network, parse, transient 5xx). Caller decides retry policy.
    async fn extract(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
        model: &str,
        max_tokens: u32,
        temperature: Option<f64>,
    ) -> Result<LlmCallResult, ExtractionEngineError>;

    /// Fan out multiple extractions with bounded concurrency.
    ///
    /// `concurrency` caps the number of in-flight LLM calls. Each item
    /// is processed independently — a per-item failure is reported in
    /// place (as `Err` in the returned `Vec`) rather than aborting the
    /// batch. This matches the rate-limited reality where some calls
    /// succeed and others retry.
    ///
    /// The returned vector preserves input order and is always the same
    /// length as `items`. The method itself does not error — per-item
    /// errors land in the vector entries.
    ///
    /// ## Rust Learning: per-item `Result` vs fail-fast `Result<Vec>`
    ///
    /// `Result<Vec<T>, E>` would force the orchestrator to retry the
    /// entire batch on a single 429. `Vec<Result<T, E>>` lets the
    /// orchestrator inspect per-item outcomes and retry only the failed
    /// indices. The Restate workflow uses the per-item form to
    /// checkpoint partial progress.
    ///
    /// # Implementation guidance
    ///
    /// The default implementation below iterates serially and ignores
    /// `concurrency`. Real adapters override this with
    /// `futures::stream::iter(items).map(...).buffer_unordered(concurrency)`,
    /// re-sorting to input order before returning. `concurrency` is part
    /// of the trait contract so callers do not change shape when swapping
    /// engines; the default impl honours correctness only, not throughput.
    async fn extract_batch(
        &self,
        items: &[BatchExtractionItem],
        concurrency: usize,
    ) -> Vec<Result<LlmCallResult, ExtractionEngineError>> {
        // `concurrency` is intentionally unused by the default impl —
        // see "Implementation guidance" above. Bind to `_` so the
        // compiler does not warn while keeping the parameter visible
        // in the trait signature for adapters to honour.
        let _ = concurrency;
        let mut results = Vec::with_capacity(items.len());
        for item in items {
            results.push(
                self.extract(
                    item.system_prompt.as_deref(),
                    &item.user_prompt,
                    &item.model,
                    item.max_tokens,
                    item.temperature,
                )
                .await,
            );
        }
        results
    }
}

#[cfg(test)]
mod tests {
    //! Tests for the `ExtractionEngineError` Display surface.
    //!
    //! The `#[error(...)]` format strings on the variants are public
    //! contract — log scrapers and operator-facing messages depend on
    //! them. Even though P1-2 ships no implementation of the trait,
    //! the error type is fully usable and its rendered output should be
    //! locked down here. P1-3 onward must not silently change these
    //! strings without a corresponding test update.
    use super::*;

    #[test]
    fn llm_call_failed_display_includes_model_and_source() {
        let source: Box<dyn std::error::Error + Send + Sync> = "boom".into();
        let err = ExtractionEngineError::LlmCallFailed {
            model: "claude-sonnet-4-6".to_string(),
            source,
        };
        let rendered = format!("{err}");
        assert_eq!(rendered, "LLM call failed for model claude-sonnet-4-6: boom");
    }

    #[test]
    fn rate_limited_display_includes_model_and_some_retry_after() {
        let err = ExtractionEngineError::RateLimited {
            model: "claude-sonnet-4-6".to_string(),
            retry_after_secs: Some(60),
        };
        // `{retry_after_secs:?}` formats Option with Debug, yielding `Some(60)`.
        // Locking down the literal rendering so a future refactor that swaps
        // to `{retry_after_secs}` (which would not compile for Option) or to
        // a manual Display impl is caught immediately.
        let rendered = format!("{err}");
        assert_eq!(
            rendered,
            "Rate limited by model claude-sonnet-4-6, retry_after_secs=Some(60)"
        );
    }

    #[test]
    fn rate_limited_display_with_no_retry_after_renders_none() {
        let err = ExtractionEngineError::RateLimited {
            model: "vllm-llama-3-8b".to_string(),
            retry_after_secs: None,
        };
        let rendered = format!("{err}");
        assert_eq!(
            rendered,
            "Rate limited by model vllm-llama-3-8b, retry_after_secs=None"
        );
    }

    #[test]
    fn configuration_display_includes_payload() {
        let err =
            ExtractionEngineError::Configuration("ANTHROPIC_API_KEY is unset".to_string());
        let rendered = format!("{err}");
        assert_eq!(
            rendered,
            "Provider configuration error: ANTHROPIC_API_KEY is unset"
        );
    }
}
