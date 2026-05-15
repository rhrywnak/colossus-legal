//! Rig 0.36 adapter implementing [`ExtractionEngine`] over Anthropic.
//!
//! ## Design requirement R4 — thin adapter
//!
//! This module is the single place in the colossus-legal backend that
//! imports anything from `rig-core`. Domain code talks to
//! `Arc<dyn ExtractionEngine>`; the implementation lives here. If Rig's
//! API changes, only this file changes.
//!
//! ## HTTP/1.1 enforcement
//!
//! Calling `api.anthropic.com` from inside a Podman container hangs
//! indefinitely when negotiating HTTP/2 over TLS — a known issue
//! documented in `colossus-extract/src/providers/anthropic.rs:30–45`.
//! The legacy `AnthropicProvider` (direct reqwest) works around this by
//! forcing HTTP/1.1 via `.http1_only()` on a custom-built
//! `reqwest::Client`. We do the same here, with one extra hoop:
//!
//! Rig 0.36 implements `rig::http_client::HttpClientExt` ONLY for the
//! reqwest **0.13** Client type. The backend's primary HTTP client
//! stays on reqwest 0.12 (it has many other consumers), so we pull
//! reqwest 0.13 in under a *renamed* alias — `reqwest_13` — via
//! `backend/Cargo.toml`. That alias is used ONLY here: we build a
//! `reqwest_13::Client` configured with `.http1_only()`, hand it to
//! Rig's `ClientBuilder::http_client(...)`, and let the rest of the
//! backend ignore it.
//!
//! **Do not remove `.http1_only()` without an end-to-end container test
//! confirming the replacement does not hang.**
//!
//! ## Rate-limit handling
//!
//! Rig 0.36 does not preserve the Anthropic `retry-after` header on
//! HTTP 429 — `rig-core` 0.36's Anthropic adapter wraps the response
//! body as `CompletionError::ProviderError(text)` and discards the
//! response headers. We detect the rate-limit condition by
//! pattern-matching the substring `"rate_limit"` (case-insensitive) in
//! that body text — Anthropic's 429 body is shaped
//! `{"type":"error","error":{"type":"rate_limit_error",…}}`. On match
//! we surface [`ExtractionEngineError::RateLimited`] with
//! `retry_after_secs: None`; the orchestrator picks a default backoff
//! in the `None` case.
//!
//! This is lossier than the legacy `AnthropicProvider`, which preserves
//! the exact `retry-after` value. We accept the loss for Phase 1;
//! revisit when Rig surfaces response headers (or when we move
//! tool-use workloads to a different adapter in a later phase).
//!
//! ## Environment variables
//!
//! - `ANTHROPIC_API_KEY` (required) — Anthropic API key.
//! - `EXTRACTION_ENGINE_TIMEOUT_SECS` (optional, default `600`) — full
//!   request timeout. Mirrors the Anthropic SDK defaults; large-context
//!   extraction (1M tokens) can legitimately take several minutes.
//! - `EXTRACTION_ENGINE_TCP_KEEPALIVE_SECS` (optional, default `60`) —
//!   TCP keep-alive interval. Some networks drop idle connections
//!   during long responses; keep-alive prevents silent disconnects.

use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use rig::client::CompletionClient;
use rig::completion::{AssistantContent, CompletionError, CompletionModel as _, Message};
use rig::providers::anthropic;
use rig::OneOrMany;

use crate::pipeline::extraction_engine::{
    BatchExtractionItem, ExtractionEngine, ExtractionEngineError, LlmCallResult,
};

// ── Config keys ─────────────────────────────────────────────────
//
// These are NOT "business values" prohibited by Rule 2 — they are the
// canonical names of the environment-variable configuration keys this
// module reads. Centralising them as named constants lets the operator
// `grep` for the literal env var and find exactly one definition.

/// Anthropic API key env var.
const ANTHROPIC_API_KEY_ENV: &str = "ANTHROPIC_API_KEY";
/// Optional override for the full reqwest request timeout, in seconds.
const TIMEOUT_SECS_ENV: &str = "EXTRACTION_ENGINE_TIMEOUT_SECS";
/// Optional override for the TCP keep-alive interval, in seconds.
const TCP_KEEPALIVE_SECS_ENV: &str = "EXTRACTION_ENGINE_TCP_KEEPALIVE_SECS";

/// Default request timeout when `EXTRACTION_ENGINE_TIMEOUT_SECS` is unset.
///
/// Matches `colossus_extract::AnthropicProvider`'s built-in default,
/// which in turn matches Anthropic's own Python and TypeScript SDKs.
const DEFAULT_TIMEOUT_SECS: u64 = 600;

/// Default TCP keep-alive interval when the env var is unset.
///
/// Matches the legacy provider's `TCP_KEEPALIVE_SECS` constant.
const DEFAULT_TCP_KEEPALIVE_SECS: u64 = 60;

/// Substring searched (case-insensitive) in `CompletionError::ProviderError`
/// payloads to detect Anthropic 429 rate-limit responses.
///
/// Anthropic's 429 body is shaped
/// `{"type":"error","error":{"type":"rate_limit_error",…}}`, so this
/// catches both the discriminator field and the error type identifier.
///
/// CONST: Anthropic's error-taxonomy key — not an env-configurable
/// knob. This string is part of the provider's public error protocol;
/// the only way it would need to change is if Anthropic re-shaped
/// their 429 body, in which case the entire detection path needs a
/// code change regardless. An env-var override here would let a
/// misconfigured operator silently break 429 detection — exactly the
/// opposite of what configurability would buy us.
const RATE_LIMIT_MARKER: &str = "rate_limit";

/// Rig 0.36 implementation of [`ExtractionEngine`].
///
/// Wraps a single Rig `anthropic::Client`. The injected HTTP client is
/// `reqwest_13::Client` configured with `.http1_only()` — see module
/// docs for why HTTP/1.1 enforcement is mission-critical.
///
/// ## Rust Learning: why no `Clone` derive here
///
/// `RigExtractionEngine` is intended to live behind `Arc<dyn ExtractionEngine>`
/// — the trait object is what gets cloned (cheaply), not the struct.
/// Implementing `Clone` on the struct would invite call sites to clone
/// the engine itself instead of cloning the `Arc`, which is more
/// expensive and obscures the intended sharing model.
pub struct RigExtractionEngine {
    /// Configured Rig client carrying our HTTP/1.1-only reqwest 0.13
    /// instance internally. Built once in `from_env`; reused for every
    /// `extract` call.
    client: anthropic::Client<reqwest_13::Client>,
}

impl RigExtractionEngine {
    /// Construct a `RigExtractionEngine` from environment variables.
    ///
    /// Reads `ANTHROPIC_API_KEY` (required), `EXTRACTION_ENGINE_TIMEOUT_SECS`
    /// (optional), and `EXTRACTION_ENGINE_TCP_KEEPALIVE_SECS` (optional).
    /// Builds a `reqwest_13::Client` with `.http1_only()` and injects it
    /// into a Rig `anthropic::Client` via the builder's
    /// `http_client(...)` hook — the only way to keep Rig from using
    /// its default HTTP/2-capable client.
    ///
    /// # Errors
    ///
    /// Returns [`ExtractionEngineError::Configuration`] when:
    /// - `ANTHROPIC_API_KEY` is unset.
    /// - The underlying `reqwest_13::Client` fails to build (typically
    ///   a TLS backend issue — rare, but possible if the system's TLS
    ///   provider is missing).
    /// - Rig rejects the configured client (e.g., a header value
    ///   constructed from the API key turned out to be non-ASCII —
    ///   should not happen for a well-formed Anthropic key, but the
    ///   error is propagated rather than panicked on).
    pub fn from_env() -> Result<Self, ExtractionEngineError> {
        let api_key = std::env::var(ANTHROPIC_API_KEY_ENV).map_err(|_| {
            ExtractionEngineError::Configuration(format!(
                "{ANTHROPIC_API_KEY_ENV} is unset — required to construct RigExtractionEngine"
            ))
        })?;

        let timeout_secs = read_secs_env(TIMEOUT_SECS_ENV, DEFAULT_TIMEOUT_SECS);
        let keepalive_secs = read_secs_env(TCP_KEEPALIVE_SECS_ENV, DEFAULT_TCP_KEEPALIVE_SECS);

        // Build the reqwest client ourselves so we can pin HTTP/1.1.
        // See module doc for why this is non-negotiable for
        // Anthropic + Podman.
        let reqwest_client = reqwest_13::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .tcp_keepalive(Duration::from_secs(keepalive_secs))
            .http1_only()
            .build()
            .map_err(|e| {
                ExtractionEngineError::Configuration(format!(
                    "Failed to build reqwest 0.13 client for Rig adapter: {e}"
                ))
            })?;

        let client = anthropic::Client::<reqwest_13::Client>::builder()
            .api_key(api_key)
            .http_client(reqwest_client)
            .build()
            .map_err(|e| {
                ExtractionEngineError::Configuration(format!(
                    "Failed to build Rig Anthropic client: {e}"
                ))
            })?;

        Ok(Self { client })
    }
}

/// Read an env var as `u64` seconds, falling back to `default` on
/// absence or parse failure.
///
/// Mirrors the silent-fallback convention already used by
/// `AppContext::from_deps_and_env` for `PIPELINE_LLM_CONCURRENCY` —
/// established house style. Unlike that call site, this helper emits a
/// `tracing::warn!` when the value is present but unparseable, so the
/// failure remains observable per Rule 1 even though the engine still
/// starts.
fn read_secs_env(name: &str, default: u64) -> u64 {
    let Ok(raw) = std::env::var(name) else {
        return default;
    };
    match raw.parse::<u64>() {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                env_var = name,
                raw_value = %raw,
                error = %e,
                default = default,
                "Invalid duration env var — falling back to default"
            );
            default
        }
    }
}

/// Map a Rig [`CompletionError`] into an [`ExtractionEngineError`].
///
/// `CompletionError::ProviderError(text)` containing the rate-limit
/// marker → [`ExtractionEngineError::RateLimited`]. Everything else →
/// [`ExtractionEngineError::LlmCallFailed`] with the Rig error wrapped
/// as the `source` so callers walking the error chain via
/// `std::error::Error::source()` still see the original.
///
/// We deliberately do not split `HttpError`, `JsonError`, `UrlError`,
/// `RequestError`, `ResponseError` into distinct
/// `ExtractionEngineError` variants: from the orchestrator's
/// perspective they all mean "the call failed, retry policy is up to
/// you", and adding variants would tighten coupling between this
/// module and Rig's error taxonomy in ways that `R4` is designed to
/// prevent.
fn map_rig_error(err: CompletionError, model: &str) -> ExtractionEngineError {
    if let CompletionError::ProviderError(ref body) = err {
        if body.to_ascii_lowercase().contains(RATE_LIMIT_MARKER) {
            return ExtractionEngineError::RateLimited {
                model: model.to_string(),
                retry_after_secs: None,
            };
        }
    }
    ExtractionEngineError::LlmCallFailed {
        model: model.to_string(),
        source: Box::new(err),
    }
}

/// Join every `AssistantContent::Text` block in the response into a
/// single string, in order, separated by newlines.
///
/// Anthropic responses can interleave text blocks with tool_use,
/// reasoning, or image blocks. For an extraction call (our only use
/// case) we want the text content concatenated; the other block types
/// are ignored. Matches the convention in Rig's own
/// `ProviderResponseExt::get_text_response` on the Anthropic-specific
/// response (`rig-core` 0.36 `completion.rs:82–97`).
fn collect_text(choice: &OneOrMany<AssistantContent>) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for content in choice.iter() {
        if let AssistantContent::Text(t) = content {
            parts.push(&t.text);
        }
    }
    parts.join("\n")
}

#[async_trait]
impl ExtractionEngine for RigExtractionEngine {
    async fn extract(
        &self,
        system_prompt: Option<&str>,
        user_prompt: &str,
        model: &str,
        max_tokens: u32,
        temperature: Option<f64>,
    ) -> Result<LlmCallResult, ExtractionEngineError> {
        let start = Instant::now();

        // Construct a fresh CompletionModel per call. The inner Rig
        // `Client<Ext, H>` is `Arc`-shared (base_url, headers, and the
        // reqwest client all live behind `Arc` or are themselves cheap
        // to clone), so this is a few refcount bumps — not a real cost.
        let model_handle = self.client.completion_model(model);

        // Rig builder pattern: start with the user prompt, then layer
        // optional system message + per-call params via the fluent API.
        //
        // We prefer `Message::System` in `chat_history` over the legacy
        // `.preamble()` accessor, per the explicit guidance on
        // `rig::completion::CompletionRequest::preamble` (request.rs:508).
        let mut builder = model_handle.completion_request(Message::user(user_prompt));
        if let Some(sys) = system_prompt {
            builder = builder.messages([Message::system(sys)]);
        }
        builder = builder
            .max_tokens(u64::from(max_tokens))
            .temperature_opt(temperature);

        let response = builder.send().await.map_err(|e| map_rig_error(e, model))?;

        let response_text = collect_text(&response.choice);
        if response_text.is_empty() {
            // A response with no text blocks is a failed extraction —
            // Rule 1 says distinct states need distinct observables.
            // Returning `Ok(LlmCallResult { response_text: "", … })`
            // would let downstream code silently produce zero entities
            // and mark the document "complete with no extractions",
            // which is indistinguishable from a successful empty page.
            let source_msg: String = format!(
                "model {model} returned no text content \
                 (response contained only tool_use / reasoning / image blocks)"
            );
            return Err(ExtractionEngineError::LlmCallFailed {
                model: model.to_string(),
                source: source_msg.into(),
            });
        }

        Ok(LlmCallResult {
            response_text,
            input_tokens: Some(response.usage.input_tokens),
            output_tokens: Some(response.usage.output_tokens),
            request_id: response.message_id.clone(),
            duration: start.elapsed(),
        })
    }

    async fn extract_batch(
        &self,
        items: &[BatchExtractionItem],
        concurrency: usize,
    ) -> Vec<Result<LlmCallResult, ExtractionEngineError>> {
        // `concurrency = 0` would make `buffer_unordered(0)` yield
        // nothing (no slots open, no progress); clamp to at least 1 so
        // a misconfigured caller still gets serial behaviour rather
        // than a silent hang. Rule 1: a misconfigured value must not
        // produce zero work.
        let concurrency = concurrency.max(1);

        // ## Rust Learning: tagging futures to preserve input order
        //
        // `buffer_unordered(n)` runs up to `n` futures concurrently
        // and yields each result as soon as it completes — which means
        // completion order, not input order. The trait contract on
        // `extract_batch` promises input-ordered output, so we attach
        // the input index to each future's return value, collect the
        // tagged tuples, then sort by index before stripping the tag.
        //
        // ## Rust Learning: capturing by index, not by `&T`
        //
        // The natural-looking pattern
        // `items.iter().enumerate().map(|(idx, item)| async move { … })`
        // trips the `FnOnce` HRTB inference: `slice::Iter::Item` is
        // `&'a BatchExtractionItem` for a *specific* `'a`, but the
        // async-block closure is asked to satisfy `FnOnce` for *any*
        // pair of lifetimes — and the compiler cannot reconcile the
        // two, because the `#[async_trait]` desugaring re-pins the
        // body inside a fresh future scope.
        //
        // Workaround: iterate `0..items.len()`, capture `idx: usize`
        // (owned, no lifetime) in the closure, and do the
        // `&items[idx]` borrow *inside* the async block. The borrow's
        // lifetime is then tied to the async future itself, which is
        // exactly what the outer `extract_batch` future already owns.
        let futures = (0..items.len()).map(|idx| async move {
            let item = &items[idx];
            let result = self
                .extract(
                    item.system_prompt.as_deref(),
                    &item.user_prompt,
                    &item.model,
                    item.max_tokens,
                    item.temperature,
                )
                .await;
            (idx, result)
        });

        let mut indexed: Vec<(usize, Result<LlmCallResult, ExtractionEngineError>)> =
            stream::iter(futures)
                .buffer_unordered(concurrency)
                .collect()
                .await;
        indexed.sort_by_key(|(idx, _)| *idx);
        indexed.into_iter().map(|(_, r)| r).collect()
    }
}

#[cfg(test)]
mod tests {
    //! Constructor tests for `RigExtractionEngine::from_env` and the
    //! `read_secs_env` helper.
    //!
    //! The first test mutates process-wide environment state, so it
    //! does all three cases (missing key, present key, malformed
    //! timeout) in a single test function rather than splitting them
    //! into separate `#[test]` blocks: cargo's test harness runs tests
    //! in parallel by default, and parallel env-var mutation is
    //! data-racy. Doing the cases in one function in sequence avoids
    //! the need to add a `serial_test` dev-dependency.
    //!
    //! The `read_secs_env_*` tests use unique env-var names that no
    //! other code reads, so parallel execution is safe.
    //!
    //! We do NOT test the live Anthropic API here. The body of
    //! `extract` is exercised by a later integration test (P1-9).
    use super::*;

    // ## Rust Learning: `unsafe fn set_var`
    //
    // `std::env::set_var` became `unsafe` in Rust 2024 because
    // mutating the process environment while other threads read it
    // is a data race. Within a single-threaded test function we know
    // no other thread is reading our chosen var, so the `unsafe` is
    // discharged by isolation, not by static analysis. We document
    // that reasoning here and `remove_var` after each case so other
    // tests in the binary see a clean slate.
    #[tokio::test]
    async fn from_env_and_extract_batch_against_env_state() {
        // ## Rust Learning: why all env-mutating cases live in ONE test
        //
        // Tests that mutate the same env var must not run concurrently.
        // cargo's harness runs `#[test]` functions on a thread pool by
        // default, so two tests both touching `ANTHROPIC_API_KEY`
        // race: one observes a clean state, sets a value, runs its
        // assertion; the OTHER observes the same var in the middle of
        // the first test's lifecycle and gets a stale read.
        //
        // The earlier P1-3 commit had this test split into two —
        // `from_env_handles_*` and `extract_batch_empty_items_*` —
        // and both raced on `ANTHROPIC_API_KEY`. P1-7's verification
        // surfaced the flake (intermittent failure ~1 in 3 runs). The
        // fix is structural: collapse all `ANTHROPIC_API_KEY`-
        // mutating cases into one sequential test function. There is
        // no other test in the binary that touches the var, so this
        // function now holds exclusive write access to it within a
        // test run.
        //
        // Async because Case 2b exercises `extract_batch(&[], 0).await`.

        // Snapshot existing values so the test does not corrupt the
        // operator's real env when run on a developer machine.
        let prior_api_key = std::env::var(ANTHROPIC_API_KEY_ENV).ok();
        let prior_timeout = std::env::var(TIMEOUT_SECS_ENV).ok();

        // Case 1: ANTHROPIC_API_KEY unset → Configuration error.
        unsafe {
            std::env::remove_var(ANTHROPIC_API_KEY_ENV);
        }
        match RigExtractionEngine::from_env() {
            Err(ExtractionEngineError::Configuration(msg)) => {
                assert!(
                    msg.contains(ANTHROPIC_API_KEY_ENV),
                    "error message should name the missing env var, got: {msg}"
                );
            }
            Err(other) => panic!("expected Configuration error, got {other:?}"),
            Ok(_) => panic!("expected error when API key is unset"),
        }

        // Case 2a: ANTHROPIC_API_KEY set to a dummy value →
        // constructor succeeds (we are not calling the API).
        unsafe {
            std::env::set_var(ANTHROPIC_API_KEY_ENV, "sk-ant-placeholder-for-test");
        }
        let engine =
            RigExtractionEngine::from_env().expect("from_env should succeed with dummy API key");

        // Case 2b: `extract_batch(&[], 0)` must clamp the zero
        // concurrency value and return an empty Vec instead of
        // hanging on `buffer_unordered(0)`. Folded into this test
        // (rather than living in its own `#[tokio::test]`) so
        // ANTHROPIC_API_KEY mutations stay single-threaded.
        let results = engine.extract_batch(&[], 0).await;
        assert!(
            results.is_empty(),
            "extract_batch with empty items should return empty Vec, got {} elements",
            results.len()
        );

        // Case 3: a malformed timeout env var does NOT break
        // construction — the warn!-then-default fallback fires.
        unsafe {
            std::env::set_var(TIMEOUT_SECS_ENV, "not_a_number");
        }
        let engine = RigExtractionEngine::from_env();
        assert!(
            engine.is_ok(),
            "from_env should fall back to default on unparseable timeout, got: {:?}",
            engine.err()
        );

        // Restore prior env state so other tests in this binary are
        // not affected by our mutations.
        unsafe {
            match prior_api_key {
                Some(v) => std::env::set_var(ANTHROPIC_API_KEY_ENV, v),
                None => std::env::remove_var(ANTHROPIC_API_KEY_ENV),
            }
            match prior_timeout {
                Some(v) => std::env::set_var(TIMEOUT_SECS_ENV, v),
                None => std::env::remove_var(TIMEOUT_SECS_ENV),
            }
        }
    }

    #[test]
    fn read_secs_env_returns_default_when_unset() {
        // Unique env-var name so parallel test execution is safe.
        const TEST_ENV: &str = "EXTRACTION_ENGINE_TEST_DEFAULT_FALLBACK_PROBE";
        unsafe {
            std::env::remove_var(TEST_ENV);
        }
        assert_eq!(read_secs_env(TEST_ENV, 42), 42);
    }

    #[test]
    fn read_secs_env_parses_valid_value() {
        const TEST_ENV: &str = "EXTRACTION_ENGINE_TEST_VALID_PARSE_PROBE";
        unsafe {
            std::env::set_var(TEST_ENV, "123");
        }
        assert_eq!(read_secs_env(TEST_ENV, 42), 123);
        unsafe {
            std::env::remove_var(TEST_ENV);
        }
    }

    // ── map_rig_error ────────────────────────────────────────────
    //
    // Pure function over (CompletionError, &str). The rate-limit
    // branch and the fallback branch are both worth locking down —
    // they are the only place where Rig's loose `ProviderError(text)`
    // is classified into the structured `ExtractionEngineError`
    // taxonomy, and a regression here would silently route 429s
    // through `LlmCallFailed` (retry decisions go wrong) or vice
    // versa (orchestrator burns retries on non-rate-limit failures).

    #[test]
    fn map_rig_error_rate_limit_body_returns_rate_limited() {
        let body = r#"{"type":"error","error":{"type":"rate_limit_error","message":"slow down"}}"#;
        let err = CompletionError::ProviderError(body.to_string());
        match map_rig_error(err, "claude-sonnet-4-6") {
            ExtractionEngineError::RateLimited {
                model,
                retry_after_secs,
            } => {
                assert_eq!(model, "claude-sonnet-4-6");
                // Rig discards the retry-after header — `None` is the
                // expected value in this version. Documented in the
                // module doc; if a future Rig version surfaces the
                // header and we wire it through, this test will break
                // and prompt the documentation to be updated alongside
                // the behavior change.
                assert_eq!(retry_after_secs, None);
            }
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }

    #[test]
    fn map_rig_error_rate_limit_match_is_case_insensitive() {
        // Anthropic uses lowercase "rate_limit_error" today, but the
        // detection is intentionally case-insensitive so that a future
        // upstream capitalisation drift does not silently flip
        // classification.
        let body = "Some Upstream Returned RATE_LIMIT in the body";
        let err = CompletionError::ProviderError(body.to_string());
        assert!(matches!(
            map_rig_error(err, "claude-sonnet-4-6"),
            ExtractionEngineError::RateLimited { .. }
        ));
    }

    #[test]
    fn map_rig_error_non_rate_limit_provider_error_returns_llm_call_failed() {
        let body = r#"{"type":"error","error":{"type":"overloaded_error","message":"…"}}"#;
        let err = CompletionError::ProviderError(body.to_string());
        match map_rig_error(err, "claude-sonnet-4-6") {
            ExtractionEngineError::LlmCallFailed { model, .. } => {
                assert_eq!(model, "claude-sonnet-4-6");
            }
            other => panic!("expected LlmCallFailed, got {other:?}"),
        }
    }

    #[test]
    fn map_rig_error_non_provider_variant_returns_llm_call_failed() {
        // `ResponseError` is the simplest non-`ProviderError` variant
        // to construct (takes a `String`); the test covers every
        // non-`ProviderError` variant by extension because the
        // function's branch is a single `if let CompletionError::ProviderError`.
        let err = CompletionError::ResponseError("parse failed".to_string());
        assert!(matches!(
            map_rig_error(err, "vllm-llama-3-8b"),
            ExtractionEngineError::LlmCallFailed { .. }
        ));
    }

    // ── collect_text ─────────────────────────────────────────────
    //
    // Pure function over `&OneOrMany<AssistantContent>`. The
    // `extract` method delegates the "is the response usable?"
    // decision to whether `collect_text` returns an empty string —
    // so the three meaningful input shapes (multiple text blocks,
    // mixed text + non-text, non-text only) all need coverage.

    #[test]
    fn collect_text_joins_multiple_text_blocks_with_newline() {
        let choice = OneOrMany::many(vec![
            AssistantContent::text("first"),
            AssistantContent::text("second"),
        ])
        .expect("non-empty input");
        assert_eq!(collect_text(&choice), "first\nsecond");
    }

    #[test]
    fn collect_text_ignores_non_text_blocks_between_text_blocks() {
        let choice = OneOrMany::many(vec![
            AssistantContent::text("hello"),
            AssistantContent::reasoning("internal thinking — should be skipped"),
            AssistantContent::text("world"),
        ])
        .expect("non-empty input");
        // Reasoning block dropped; remaining text joined with newline.
        assert_eq!(collect_text(&choice), "hello\nworld");
    }

    #[test]
    fn collect_text_returns_empty_string_when_no_text_blocks() {
        // A response made up entirely of non-text content (reasoning
        // here, but the principle is the same for tool_use and image
        // blocks) yields an empty string — which is the signal
        // `extract` uses to fail-fast with `LlmCallFailed`.
        let choice = OneOrMany::one(AssistantContent::reasoning("model thought, did not write"));
        assert_eq!(collect_text(&choice), "");
    }

    // (extract_batch zero-concurrency clamp test was merged into
    // `from_env_and_extract_batch_against_env_state` above to keep
    // ANTHROPIC_API_KEY mutations single-threaded.)
}
