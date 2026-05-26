//! Bridge from the legacy `LlmProvider` trait to the new
//! `ExtractionEngine` trait.
//!
//! ## Why a bridge
//!
//! The legacy `LlmProvider` trait (in the `colossus-extract` git dep)
//! has dozens of call sites: pipeline steps, the RAG synthesizer
//! inside `colossus-rag`, every reader of `cost_per_input_token()` /
//! `model_name()` / `supports_structured_output()`. Migrating every
//! call site to `ExtractionEngine` in one commit would be a sprawling
//! diff with high regression risk.
//!
//! Instead, this module implements `LlmProvider` on a new struct that
//! internally delegates every operation to an `Arc<dyn ExtractionEngine>`.
//! Every existing call site keeps calling `LlmProvider::invoke()`
//! exactly as before, but the actual HTTP traffic now goes through
//! Rig 0.36 — with HTTP/1.1 enforcement (via `RigExtractionEngine`'s
//! reqwest 0.13 client) instead of `colossus-extract`'s legacy
//! reqwest 0.12 client.
//!
//! Phase 3 removes the legacy `LlmProvider` trait outright and migrates
//! the call sites to `ExtractionEngine` directly. Until then this
//! bridge is the seam.
//!
//! ## Behaviour preservation
//!
//! The bridge holds the same five fields the legacy `AnthropicProvider`
//! held: model id, optional input/output costs, optional temperature,
//! plus the shared engine. `provider_for_model` constructs it with
//! `temperature: Some(0.0)` to preserve deterministic extraction;
//! `AppContext` constructs it with whatever `LLM_TEMPERATURE` was set
//! to (or `None` if unset/unparseable — required for Opus 4.7 which
//! rejects any sampling key).
//!
//! ## Observability seam
//!
//! `provider_name()` returns `"rig-anthropic"` (NOT `"anthropic"`).
//! This is intentional: log/trace events emitted by the bridge carry a
//! different `provider_name` than the legacy
//! `colossus_extract::AnthropicProvider` did, so log aggregators and
//! cost dashboards can tell pre-migration from post-migration data
//! apart for the duration of the rollout window.

use std::sync::Arc;

use async_trait::async_trait;
use colossus_extract::{LlmProvider, LlmResponse, PipelineError};

use crate::pipeline::extraction_engine::{ExtractionEngine, ExtractionEngineError};

/// Default `retry-after` value when the engine reports a rate limit
/// with no header value.
///
/// CONST: matches the fallback stated in `PipelineError::RateLimited`'s
/// own doc (colossus-extract/src/error.rs:71 — "If the header was
/// absent (rare), defaults to 60"). Rig 0.36 discards the
/// `retry-after` header, so this value will be returned for *every*
/// 429 the bridge sees until Rig surfaces the header. The legacy
/// `AnthropicProvider` preserved the precise header value — the
/// migration to Rig sacrifices precision for that header in
/// exchange for the broader Rig integration. The orchestrator's
/// retry policy must therefore tolerate `60s` as a worst case for
/// every rate-limit response.
const DEFAULT_RATE_LIMIT_RETRY_SECS: u64 = 60;

/// Provider name returned by [`LlmProvider::provider_name`].
///
/// CONST: operator-visible identifier emitted on log/trace events.
/// Deliberately `"rig-anthropic"` (NOT `"anthropic"`) so log
/// aggregators and cost reports can tell pre-migration (legacy
/// `colossus_extract::AnthropicProvider`, recorded as `"anthropic"`)
/// from post-migration (this bridge → `RigExtractionEngine`, recorded
/// as `"rig-anthropic"`) traffic. Not configurable: the value is
/// part of the observability contract.
const PROVIDER_NAME: &str = "rig-anthropic";

/// Adapter that exposes the legacy `LlmProvider` interface while
/// delegating to the new `ExtractionEngine`.
///
/// ## Rust Learning: `Arc<dyn Trait>` for shared dependency injection
///
/// The bridge holds an `Arc<dyn ExtractionEngine>`, not an owned
/// `RigExtractionEngine`. This means a single engine — built once at
/// startup in `AppContext` — is shared by every bridge instance:
/// one per `provider_for_model(...)` call (per document, for
/// extraction), one in `AppContext.llm_provider` (for RAG), and the
/// future Restate workflow handlers (P2). They all keep refcount on
/// the same underlying Rig client.
pub struct RigLlmProviderBridge {
    /// Shared engine — one HTTP client, refcount-shared across every
    /// bridge instance and the workflow handlers.
    engine: Arc<dyn ExtractionEngine>,
    /// Model identifier passed to `engine.extract(...)` on every
    /// `invoke` call. Sourced from `LlmModelRecord.id` (per-document
    /// extraction path) or from `LLM_MODEL` env var (AppContext path).
    model: String,
    /// Optional cost per input token in USD. Returned verbatim from
    /// the LlmProvider accessor; not consumed by the bridge itself.
    cost_input: Option<f64>,
    /// Optional cost per output token in USD.
    cost_output: Option<f64>,
    /// Sampling temperature passed to `engine.extract(...)`.
    ///
    /// Provider_for_model passes `Some(0.0)` to preserve deterministic
    /// extraction (matching the legacy AnthropicProvider behaviour).
    /// AppContext passes whatever `LLM_TEMPERATURE` parses to (or
    /// `None` if unset/unparseable — required for Claude Opus 4.7
    /// which rejects any sampling key). The bridge stores it owned
    /// per-instance so different consumers can hold different
    /// temperatures against the same engine.
    temperature: Option<f64>,
}

impl RigLlmProviderBridge {
    /// Construct a bridge.
    ///
    /// Always succeeds — no I/O, no env reads. The shared engine is
    /// expected to have been built by the caller (typically
    /// `AppContext::from_deps_and_env`) before this is called.
    pub fn new(
        engine: Arc<dyn ExtractionEngine>,
        model: String,
        cost_input: Option<f64>,
        cost_output: Option<f64>,
        temperature: Option<f64>,
    ) -> Self {
        Self {
            engine,
            model,
            cost_input,
            cost_output,
            temperature,
        }
    }
}

/// Convert a `u64` token count (returned by `LlmCallResult`) into an
/// `Option<u32>` (expected by `LlmResponse`).
///
/// `None` means the provider did not report — distinguishable from
/// `Some(0)`.
///
/// ## Rust Learning: `u32::try_from(u64).ok()`
///
/// `try_from` succeeds for values `<= u32::MAX` (~4.3 B) and fails
/// otherwise. Calling `.ok()` collapses the failure into `None`. We
/// chose this over the cheaper `n as u32` cast because the cast
/// silently wraps on overflow — Rule 1 says distinct states need
/// distinct observables, and "provider returned an unreasonable
/// count" is a distinct state from "provider didn't tell us anything".
/// Token counts never reach the overflow threshold in practice;
/// preferring the explicit failure mode costs us nothing and makes
/// the failure visible if it ever does.
fn convert_tokens(value: Option<u64>) -> Option<u32> {
    // best-effort: u64 → u32 narrowing fails for counts above u32::MAX
    // (~4.3 billion) — we collapse the failure to None rather than
    // wrap. None is distinguishable from "provider reported zero" via
    // the Option layer (Rule 1: distinct states stay distinct).
    value.and_then(|n| u32::try_from(n).ok())
}

/// Map an [`ExtractionEngineError`] to a legacy [`PipelineError`].
///
/// Mapping is one-way only (legacy → modern is not needed because
/// no call site walks the chain in the modern direction). Each
/// variant has documented behavior:
///
/// - `RateLimited { retry_after_secs, .. }` → preserves the typed
///   shape. The bridge supplies `60` as the default when the engine
///   reports `None` (Rig 0.36 always reports None — see
///   [`DEFAULT_RATE_LIMIT_RETRY_SECS`]). The legacy variant's `u64`
///   type forces a value; we cannot pass through the
///   None/Some distinction the modern variant carries.
/// - `LlmCallFailed { model, source }` → `LlmProvider(String)` —
///   prefixes the model id so the legacy variant's flat string
///   still names which model failed.
/// - `Configuration(msg)` → `LlmProvider(String)` — tagged as
///   `"configuration: …"` so a log scraper can distinguish from
///   call-time failures.
fn map_engine_error(err: ExtractionEngineError) -> PipelineError {
    match err {
        ExtractionEngineError::RateLimited {
            model: _,
            retry_after_secs,
        } => PipelineError::RateLimited {
            retry_after_secs: retry_after_secs.unwrap_or(DEFAULT_RATE_LIMIT_RETRY_SECS),
        },
        ExtractionEngineError::LlmCallFailed { model, source } => {
            PipelineError::LlmProvider(format!("model {model}: {source}"))
        }
        ExtractionEngineError::Configuration(msg) => {
            PipelineError::LlmProvider(format!("configuration: {msg}"))
        }
    }
}

#[async_trait]
impl LlmProvider for RigLlmProviderBridge {
    async fn invoke(&self, prompt: &str, max_tokens: u32) -> Result<LlmResponse, PipelineError> {
        let result = self
            .engine
            .extract(None, prompt, &self.model, max_tokens, self.temperature)
            .await
            .map_err(map_engine_error)?;
        Ok(LlmResponse {
            text: result.response_text,
            input_tokens: convert_tokens(result.input_tokens),
            output_tokens: convert_tokens(result.output_tokens),
        })
    }

    async fn invoke_with_system(
        &self,
        system: &str,
        prompt: &str,
        max_tokens: u32,
    ) -> Result<LlmResponse, PipelineError> {
        let result = self
            .engine
            .extract(
                Some(system),
                prompt,
                &self.model,
                max_tokens,
                self.temperature,
            )
            .await
            .map_err(map_engine_error)?;
        Ok(LlmResponse {
            text: result.response_text,
            input_tokens: convert_tokens(result.input_tokens),
            output_tokens: convert_tokens(result.output_tokens),
        })
    }

    fn provider_name(&self) -> &str {
        PROVIDER_NAME
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn cost_per_input_token(&self) -> Option<f64> {
        self.cost_input
    }

    fn cost_per_output_token(&self) -> Option<f64> {
        self.cost_output
    }

    fn supports_structured_output(&self) -> bool {
        // CONST: always true for the Anthropic-backed
        // RigExtractionEngine — the only backend wired in P1-8. Will
        // become a per-engine capability flag if/when Rig grows
        // vLLM or other completion models (Phase 3+). Anthropic
        // Messages API supports tool-use JSON output natively, so
        // there is no useful operator override of this value today;
        // returning `false` would falsely disable structured-output
        // codepaths the model fully supports.
        true
    }
}

#[cfg(test)]
mod tests {
    //! Tests for the pure mapping helpers — `map_engine_error` and
    //! `convert_tokens`. Bridge-method bodies that call
    //! `engine.extract(...)` are not unit-tested because the engine
    //! does live I/O; P1-9 covers them via integration test.
    //!
    //! Accessor tests use a stub `ExtractionEngine` that panics on
    //! `extract` (we never invoke it) — sufficient to construct an
    //! `Arc<dyn ExtractionEngine>` so the bridge can be built.
    use super::*;

    use async_trait::async_trait;

    use crate::pipeline::extraction_engine::{BatchExtractionItem, LlmCallResult};

    /// Stub engine that panics if `extract` is ever called. Used only
    /// to construct an `Arc<dyn ExtractionEngine>` for accessor tests
    /// that never reach a network call.
    struct UnreachableEngine;

    #[async_trait]
    impl ExtractionEngine for UnreachableEngine {
        async fn extract(
            &self,
            _system_prompt: Option<&str>,
            _user_prompt: &str,
            _model: &str,
            _max_tokens: u32,
            _temperature: Option<f64>,
        ) -> Result<LlmCallResult, ExtractionEngineError> {
            unreachable!(
                "UnreachableEngine.extract called — test should not exercise the call path"
            );
        }

        async fn extract_batch(
            &self,
            _items: &[BatchExtractionItem],
            _concurrency: usize,
        ) -> Vec<Result<LlmCallResult, ExtractionEngineError>> {
            unreachable!("UnreachableEngine.extract_batch called");
        }
    }

    fn build_bridge(temperature: Option<f64>) -> RigLlmProviderBridge {
        RigLlmProviderBridge::new(
            Arc::new(UnreachableEngine),
            "claude-sonnet-4-6".to_string(),
            Some(0.000_003),
            Some(0.000_015),
            temperature,
        )
    }

    // ── map_engine_error ─────────────────────────────────────────

    #[test]
    fn error_map_rate_limited_with_some_secs_preserves_value() {
        let err = ExtractionEngineError::RateLimited {
            model: "claude-sonnet-4-6".to_string(),
            retry_after_secs: Some(120),
        };
        match map_engine_error(err) {
            PipelineError::RateLimited { retry_after_secs } => {
                assert_eq!(retry_after_secs, 120);
            }
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }

    #[test]
    fn error_map_rate_limited_with_none_secs_falls_back_to_default() {
        let err = ExtractionEngineError::RateLimited {
            model: "claude-sonnet-4-6".to_string(),
            retry_after_secs: None,
        };
        match map_engine_error(err) {
            PipelineError::RateLimited { retry_after_secs } => {
                assert_eq!(retry_after_secs, DEFAULT_RATE_LIMIT_RETRY_SECS);
                assert_eq!(retry_after_secs, 60); // Spec lock-down.
            }
            other => panic!("expected RateLimited, got {other:?}"),
        }
    }

    #[test]
    fn error_map_llm_call_failed_to_llm_provider_with_model_in_message() {
        let source: Box<dyn std::error::Error + Send + Sync> = "network read failed".into();
        let err = ExtractionEngineError::LlmCallFailed {
            model: "claude-sonnet-4-6".to_string(),
            source,
        };
        match map_engine_error(err) {
            PipelineError::LlmProvider(msg) => {
                assert!(
                    msg.contains("claude-sonnet-4-6"),
                    "message should name the model, got: {msg}"
                );
                assert!(
                    msg.contains("network read failed"),
                    "message should include the underlying source, got: {msg}"
                );
            }
            other => panic!("expected LlmProvider, got {other:?}"),
        }
    }

    #[test]
    fn error_map_configuration_to_llm_provider_tagged_as_configuration() {
        let err = ExtractionEngineError::Configuration("ANTHROPIC_API_KEY is unset".to_string());
        match map_engine_error(err) {
            PipelineError::LlmProvider(msg) => {
                assert!(
                    msg.contains("configuration"),
                    "message should be tagged as configuration, got: {msg}"
                );
                assert!(
                    msg.contains("ANTHROPIC_API_KEY is unset"),
                    "message should preserve the original payload, got: {msg}"
                );
            }
            other => panic!("expected LlmProvider, got {other:?}"),
        }
    }

    // ── convert_tokens ───────────────────────────────────────────

    #[test]
    fn token_conversion_normal_values_pass_through() {
        assert_eq!(convert_tokens(Some(0)), Some(0));
        assert_eq!(convert_tokens(Some(100)), Some(100));
        assert_eq!(convert_tokens(Some(u32::MAX as u64)), Some(u32::MAX));
    }

    #[test]
    fn token_conversion_none_passes_through_as_none() {
        // "Provider did not report" must remain distinguishable from
        // "provider reported zero" — Rule 1.
        assert_eq!(convert_tokens(None), None);
    }

    #[test]
    fn token_conversion_overflow_collapses_to_none() {
        // A u64 token count above u32::MAX maps to None rather than
        // silently wrapping. Distinguishable from "didn't report" only
        // by absence of an accompanying log; the trait surface is
        // lossy here by necessity (the legacy `LlmResponse` is
        // u32-shaped). Acceptable because real Anthropic responses
        // never approach this boundary.
        let overflow = (u32::MAX as u64) + 1;
        assert_eq!(convert_tokens(Some(overflow)), None);
    }

    // ── accessors ────────────────────────────────────────────────

    #[test]
    fn provider_name_is_rig_anthropic() {
        let bridge = build_bridge(Some(0.0));
        assert_eq!(bridge.provider_name(), "rig-anthropic");
        assert_eq!(bridge.provider_name(), PROVIDER_NAME); // spec lock-down
    }

    #[test]
    fn model_name_returns_constructor_value() {
        let bridge = build_bridge(None);
        assert_eq!(bridge.model_name(), "claude-sonnet-4-6");
    }

    #[test]
    fn cost_accessors_return_constructor_values() {
        let bridge = build_bridge(Some(0.0));
        assert_eq!(bridge.cost_per_input_token(), Some(0.000_003));
        assert_eq!(bridge.cost_per_output_token(), Some(0.000_015));
    }

    #[test]
    fn supports_structured_output_is_true() {
        let bridge = build_bridge(None);
        assert!(bridge.supports_structured_output());
    }

    // ── Live integration test ────────────────────────────────────
    //
    // Companion to `pipeline::rig_provider::tests::test_rig_engine_live`
    // but exercises the full bridge path: a real `RigExtractionEngine`
    // built from env, wrapped in `RigLlmProviderBridge`, called via
    // the legacy `LlmProvider::invoke_with_system` surface. Validates
    // the end-to-end production call sequence:
    //
    //     LlmProvider::invoke_with_system(bridge, system, prompt, max)
    //       → bridge.engine.extract(...)
    //         → RigExtractionEngine → reqwest 0.13 HTTP/1.1
    //           → api.anthropic.com
    //         ← LlmCallResult
    //       ← map_engine_error / convert_tokens
    //     ← LlmResponse
    //
    // Gated with `#[ignore]` + self-skip; run with:
    //
    //     cargo test --lib test_rig_bridge_live -- --ignored --nocapture

    #[tokio::test]
    #[ignore]
    async fn test_rig_bridge_live() {
        let Ok(_) = std::env::var("ANTHROPIC_API_KEY") else {
            return;
        };

        let engine = Arc::new(
            crate::pipeline::rig_provider::RigExtractionEngine::from_env()
                .expect("engine construction"),
        );
        let bridge = RigLlmProviderBridge::new(
            engine,
            "claude-sonnet-4-6".to_string(),
            None,
            None,
            Some(0.0),
        );

        let result = bridge
            .invoke_with_system(
                "You are a legal document extraction assistant. Extract the named entity from the text.",
                "Extract the person name from: 'George Phillips was named as defendant.'",
                200,
            )
            .await
            .expect("invoke_with_system call");

        println!("=== RIG BRIDGE LIVE TEST ===");
        println!("Response text: {}", result.text);
        println!("Input tokens: {:?}", result.input_tokens);
        println!("Output tokens: {:?}", result.output_tokens);
        println!("Provider name: {}", bridge.provider_name());
        println!("Model name: {}", bridge.model_name());
        println!("============================");

        assert!(!result.text.is_empty(), "Response text was empty");
        assert!(
            result.text.contains("George") || result.text.contains("Phillips"),
            "Expected entity extraction to find George Phillips, got: {}",
            result.text
        );
        assert!(result.input_tokens.unwrap_or(0) > 0, "Input tokens missing");
        assert!(
            result.output_tokens.unwrap_or(0) > 0,
            "Output tokens missing"
        );
    }
}
