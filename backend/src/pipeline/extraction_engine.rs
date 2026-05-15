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

// ─────────────────────────────────────────────────────────────────
//
// `EmbeddingEngine` and its error type live in the same module as
// `ExtractionEngine` because they are companion abstractions over
// the same domain (one wraps LLM completion, the other wraps text
// embedding). Splitting them into separate modules would invite
// drift — they share doc-comment cross-references, share the same
// `Send + Sync + 'static` rationale, and share the same `Box<dyn
// Error + Send + Sync>` source-erasure pattern. Keeping them
// adjacent makes the shared design loud rather than implicit.
//
// ─────────────────────────────────────────────────────────────────

/// Typed error from any [`EmbeddingEngine`] method.
///
/// Companion to [`ExtractionEngineError`] — the same
/// `Box<dyn Error + Send + Sync>` source-erasure keeps the
/// underlying embedding-backend types (FastEmbed, ONNX, Rig's
/// `EmbeddingError`, etc.) out of the trait surface, preserving R4.
///
/// Unlike [`ExtractionEngineError`] there is no `RateLimited`
/// variant. The only embedding workload colossus-legal currently
/// runs is the local FastEmbed-on-CPU path, which cannot be
/// rate-limited. If a future implementation adds a remote embedding
/// provider (OpenAI, Cohere) with quota enforcement, add a
/// `RateLimited` variant then — rather than carry one on speculation
/// now.
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingEngineError {
    /// The underlying embedding call failed — ONNX runtime error,
    /// tokeniser failure, transient HTTP error against a remote
    /// embedding backend, etc.
    ///
    /// `model` is the model identifier that was being called —
    /// included in the message so the operator can see which
    /// embedding model misbehaved without consulting the call site
    /// (Rule 1: failures are observable and distinguishable).
    #[error("Embedding call failed for model {model}: {source}")]
    EmbedFailed {
        /// Model identifier that was being called.
        model: String,
        /// Underlying error from the adapter implementation.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// The engine was constructed with invalid configuration —
    /// missing `FASTEMBED_MODEL`, an unrecognised model name, a
    /// missing `VLLM_BASE_URL` for the vLLM backend, etc.
    ///
    /// Distinct from `EmbedFailed`: configuration errors are caught
    /// at engine-construction time and never imply a retry is
    /// appropriate.
    #[error("Provider configuration error: {0}")]
    Configuration(String),
}

/// The colossus-legal-facing facade over text-embedding operations.
///
/// Companion to [`ExtractionEngine`] — same posture, same bounds,
/// same R4 rationale (domain code never touches the underlying
/// embedding library). When the implementation adopts Rig's
/// `rig-fastembed` companion crate in a later phase, this trait is
/// the single abstraction the rest of the backend depends on; Rig
/// types stay confined to the adapter module.
///
/// ## Relationship to `EmbeddingProvider` in `colossus-extract`
///
/// `EmbeddingProvider` (from the `colossus-extract` git dep) is the
/// *legacy* embedding interface used by the current Index step,
/// QdrantRetriever, and EmbeddingReranker. During Phase 1 both
/// interfaces coexist:
///
/// - Legacy `Index` step + RAG retriever + reranker → `EmbeddingProvider`
/// - Restate-driven indexing workflow (future) → `EmbeddingEngine` (this trait)
///
/// Phase 4 removes `EmbeddingProvider` together with `colossus-rag`;
/// only `EmbeddingEngine` remains.
///
/// The bounds (`Send + Sync + 'static`, `#[async_trait]`) carry the
/// same rationale as on [`ExtractionEngine`] — see the "Rust
/// Learning" sections on that trait for the full explanation rather
/// than restate it here.
#[async_trait]
pub trait EmbeddingEngine: Send + Sync + 'static {
    /// Embed a single text into a vector.
    ///
    /// # Errors
    ///
    /// - [`EmbeddingEngineError::Configuration`] for permanent setup
    ///   failures (model not loaded, missing endpoint URL). Do NOT
    ///   retry.
    /// - [`EmbeddingEngineError::EmbedFailed`] for everything else
    ///   (transient I/O, ONNX runtime error, remote backend error).
    ///   Caller decides retry policy.
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingEngineError>;

    /// Embed multiple texts in a single call.
    ///
    /// Default implementation calls [`embed`](Self::embed) serially
    /// over the slice. Implementations backed by native batch APIs
    /// (FastEmbed's rayon-parallel batch path, OpenAI's batch
    /// embeddings endpoint, vLLM's batched inference) should
    /// override this for throughput. The legacy
    /// `colossus_extract::FastembedProvider` overrides its
    /// equivalent for exactly this reason: a single `spawn_blocking`
    /// over a rayon parallel iterator is dramatically cheaper than
    /// `N` serial blocking calls.
    ///
    /// `texts` is `&[String]` rather than `&[&str]` so implementations
    /// can move the owned strings into spawned blocking tasks or
    /// remote-API request bodies without re-allocating. Callers
    /// usually already own the strings (e.g. extraction-step output);
    /// the slight ergonomic cost of building a `Vec<String>` is
    /// dwarfed by avoiding the clone an implementer would otherwise
    /// have to do.
    ///
    /// # Errors
    ///
    /// The trait contract is fail-fast: the returned `Result` is
    /// either `Ok(Vec)` with the same length as `texts`, or `Err`.
    /// (Compare with [`ExtractionEngine::extract_batch`], which
    /// returns `Vec<Result<_, _>>` per-item — that asymmetry is
    /// deliberate: LLM rate limits make partial-success the common
    /// case for extraction, whereas embedding failures are usually
    /// fail-the-whole-document affairs.)
    async fn embed_batch(
        &self,
        texts: &[String],
    ) -> Result<Vec<Vec<f32>>, EmbeddingEngineError> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    /// Dimension of the embedding vectors this engine produces.
    ///
    /// MUST equal the dimensions configured in the Qdrant collection
    /// — mismatches cause runtime query failures with vector-shape
    /// errors. The startup path in `main.rs` reads this value to
    /// call `qdrant_service::ensure_collection`, and the indexing
    /// step re-reads it as a defensive check before writing vectors.
    /// Implementations expose the static, configured dimension — not
    /// the actual size of any specific `embed` call's output.
    fn dimensions(&self) -> u32;

    /// Human-readable model name for logging and cost tracking.
    ///
    /// Returned value is stable across the engine's lifetime and is
    /// suitable for grouping observations in `pipeline_events`.
    /// Typical values: `"nomic-embed-text-v1.5"`,
    /// `"bge-small-en-v1.5"`, `"text-embedding-3-small"`.
    fn model_name(&self) -> &str;
}

#[cfg(test)]
mod tests {
    //! Tests for the `ExtractionEngineError` and `EmbeddingEngineError`
    //! Display surfaces.
    //!
    //! The `#[error(...)]` format strings on the variants are public
    //! contract — log scrapers and operator-facing messages depend on
    //! them. Even though this module ships no implementation of either
    //! trait, the error types are fully usable and their rendered
    //! output should be locked down here. Future tasks must not
    //! silently change these strings without a corresponding test
    //! update.
    use super::*;

    #[test]
    fn llm_call_failed_display_includes_model_and_source() {
        let source: Box<dyn std::error::Error + Send + Sync> = "boom".into();
        let err = ExtractionEngineError::LlmCallFailed {
            model: "claude-sonnet-4-6".to_string(),
            source,
        };
        let rendered = format!("{err}");
        assert_eq!(
            rendered,
            "LLM call failed for model claude-sonnet-4-6: boom"
        );
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
        let err = ExtractionEngineError::Configuration("ANTHROPIC_API_KEY is unset".to_string());
        let rendered = format!("{err}");
        assert_eq!(
            rendered,
            "Provider configuration error: ANTHROPIC_API_KEY is unset"
        );
    }

    // ── EmbeddingEngineError ─────────────────────────────────────

    #[test]
    fn embed_failed_display_includes_model_and_source() {
        let source: Box<dyn std::error::Error + Send + Sync> = "ONNX runtime failure".into();
        let err = EmbeddingEngineError::EmbedFailed {
            model: "nomic-embed-text-v1.5".to_string(),
            source,
        };
        let rendered = format!("{err}");
        assert_eq!(
            rendered,
            "Embedding call failed for model nomic-embed-text-v1.5: ONNX runtime failure"
        );
    }

    #[test]
    fn embedding_configuration_display_includes_payload() {
        // Distinct test name from `configuration_display_includes_payload`
        // (which covers `ExtractionEngineError::Configuration`) — both
        // live in the same test module so the names must not collide.
        let err = EmbeddingEngineError::Configuration("FASTEMBED_MODEL is unset".to_string());
        let rendered = format!("{err}");
        assert_eq!(
            rendered,
            "Provider configuration error: FASTEMBED_MODEL is unset"
        );
    }
}
