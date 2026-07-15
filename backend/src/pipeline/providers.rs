//! Per-document LLM provider construction.
//!
//! The global `AppContext.llm_provider` is used by the RAG pipeline (chat).
//! For extraction, we need per-document model selection — the user can
//! choose a different model for each document via the processing profile
//! or a per-document override.
//!
//! This module constructs an `LlmProvider` trait object from an
//! `llm_models` DB row. The concrete provider type is chosen based on
//! the row's `provider` column:
//!
//! - `"anthropic"` → [`RigLlmProviderBridge`] wrapping the shared
//!   `Arc<dyn ExtractionEngine>` (Rig 0.36 with HTTP/1.1). Replaces the
//!   legacy `colossus_extract::AnthropicProvider` (P1-8 migration).
//! - `"vllm"` → `colossus_extract::VllmProvider` (unchanged — vLLM
//!   support stays on the legacy path until Rig grows a vLLM-style
//!   embedding/inference provider).
//!
//! Design: DOC_PROCESSING_CONFIG_DESIGN_v2.md Section 3.8.

use std::sync::Arc;

use colossus_extract::{LlmProvider, VllmProvider};

use crate::pipeline::extraction_engine::ExtractionEngine;
use crate::pipeline::rig_llm_bridge::RigLlmProviderBridge;
use crate::repositories::pipeline_repository::LlmModelRecord;

/// Env var holding an optional vLLM API key (authenticated deployments).
const VLLM_API_KEY_ENV: &str = "VLLM_API_KEY";

/// Default `max_tokens_default` for the provider when the `llm_models`
/// row has `max_output_tokens = NULL`. The per-call `invoke()` uses its
/// own `max_tokens` parameter, so this is only a fallback accessor value.
///
/// CONST: per-model column default — not env-configurable. Operators
/// set the per-model value by editing the `llm_models.max_output_tokens`
/// column; this constant only governs the unset-column fallback.
const FALLBACK_MAX_TOKENS: u32 = 8000;

/// Sampling temperature pinned by the extraction path for deterministic
/// output.
///
/// CONST: pipeline determinism contract — not env-configurable for the
/// per-document extraction path. The Chat endpoint builds its own
/// providers with `temperature = None` for natural variation (see
/// `main.rs::build_chat_providers`); the per-document extraction path
/// here pins `Some(0.0)` because chunked / structured extraction
/// requires byte-identical reruns to keep verification stable.
const EXTRACTION_TEMPERATURE: Option<f64> = Some(0.0);

/// Construct an `LlmProvider` trait object from a registered model row.
///
/// Dispatches on `model.provider`:
/// - `"anthropic"` → [`RigLlmProviderBridge`] wrapping the supplied
///   shared engine. Uses the model id from the DB row, the row's cost
///   columns, and pins `temperature = Some(0.0)` for deterministic
///   extraction.
/// - `"vllm"` → `VllmProvider::new` using `model.api_endpoint` as the
///   base URL (required for vLLM) and `VLLM_API_KEY` (optional).
///
/// `engine` is the shared `Arc<dyn ExtractionEngine>` constructed once
/// at startup in `AppContext` — see P1-5 for the wiring. The same
/// engine instance is used across every per-document bridge.
///
/// Returns `Err` with a descriptive message if the provider string is
/// unknown or a required `api_endpoint` is missing for a vLLM row.
pub fn provider_for_model(
    engine: &Arc<dyn ExtractionEngine>,
    model: &LlmModelRecord,
) -> Result<Box<dyn LlmProvider>, String> {
    match model.provider.as_str() {
        "anthropic" => {
            // The bridge does NOT consume max_tokens at construction —
            // each `invoke` call passes its own max_tokens. Cost columns
            // and temperature ARE constructor-time: costs are returned
            // verbatim via the LlmProvider accessor; temperature pins
            // extraction determinism (see [`EXTRACTION_TEMPERATURE`]).
            let bridge = RigLlmProviderBridge::new(
                Arc::clone(engine),
                model.id.clone(),
                model.cost_per_input_token,
                model.cost_per_output_token,
                EXTRACTION_TEMPERATURE,
            );
            Ok(Box::new(bridge))
        }
        "vllm" => {
            // VllmProvider stays on the legacy path — Rig 0.36 does not
            // yet provide a vLLM-compatible completion model, and
            // colossus-extract's VllmProvider already speaks the OpenAI-
            // compatible API correctly. Migrate when Rig adds support.
            let endpoint = model.api_endpoint.clone().ok_or_else(|| {
                format!(
                    "vLLM model '{}' has no api_endpoint — required for vllm provider",
                    model.id
                )
            })?;
            // best-effort: `VLLM_API_KEY` is optional for unauthenticated
            // vLLM deployments. `.ok()` collapses `VarError::NotPresent`
            // to `None` and forwards it to the provider, which treats
            // None as "send no auth header".
            let api_key = std::env::var(VLLM_API_KEY_ENV).ok();
            // best-effort: `max_output_tokens` is i32 in the DB row but
            // the provider API expects u32. A negative or out-of-range
            // value collapses via try_from→None and falls back to
            // FALLBACK_MAX_TOKENS — protects against a corrupt
            // llm_models row without aborting the worker.
            let max_tokens_default = model
                .max_output_tokens
                .and_then(|n| u32::try_from(n).ok())
                .unwrap_or(FALLBACK_MAX_TOKENS);
            // `request_timeout_secs = None` → provider default. The
            // per-request timeout for vLLM will be threaded through
            // when the colossus-extract VllmProvider grows the hook.
            let provider = VllmProvider::new(
                endpoint,
                model.id.clone(),
                api_key,
                max_tokens_default,
                None,
            )
            .map_err(|e| format!("VllmProvider::new failed: {e}"))?;
            Ok(Box::new(provider))
        }
        other => Err(format!(
            "Unknown provider '{other}' for model '{}' — expected 'anthropic' or 'vllm'",
            model.id
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;

    use crate::pipeline::extraction_engine::{
        BatchExtractionItem, ExtractionEngineError, LlmCallResult,
    };

    /// Stub engine used to satisfy `provider_for_model`'s `&Arc<dyn
    /// ExtractionEngine>` parameter in tests. None of these tests
    /// reach the `extract` call path — they exercise dispatch on
    /// `model.provider` and the vllm endpoint-missing branch.
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
            unreachable!("provider_for_model tests must not call extract");
        }

        async fn extract_batch(
            &self,
            _items: &[BatchExtractionItem],
            _concurrency: usize,
        ) -> Vec<Result<LlmCallResult, ExtractionEngineError>> {
            unreachable!("provider_for_model tests must not call extract_batch");
        }
    }

    fn engine() -> Arc<dyn ExtractionEngine> {
        Arc::new(UnreachableEngine)
    }

    fn make_model(id: &str, provider: &str, endpoint: Option<&str>) -> LlmModelRecord {
        LlmModelRecord {
            id: id.to_string(),
            display_name: id.to_string(),
            provider: provider.to_string(),
            api_endpoint: endpoint.map(String::from),
            max_context_tokens: None,
            max_output_tokens: Some(8000),
            cost_per_input_token: None,
            cost_per_output_token: None,
            is_active: true,
            created_at: Utc::now(),
            notes: None,
            // Chunk A added these read-only fields to LlmModelRecord. This test
            // helper exercises provider_for_model, which does not read them, so
            // they are left None/unset here (mechanical struct-literal fix only).
            default_temperature: None,
            temperature_mode: None,
            timeout_secs: None,
            structured_output_mode: None,
            max_concurrency: None,
        }
    }

    #[test]
    fn anthropic_returns_bridge_named_rig_anthropic() {
        let engine = engine();
        let model = make_model("claude-sonnet-4-6", "anthropic", None);
        let provider = provider_for_model(&engine, &model)
            .expect("anthropic provider should construct from a shared engine");
        // Sanity-check the bridge's accessors so a future refactor that
        // accidentally routes anthropic through the legacy path
        // (provider_name = "anthropic") fails this test.
        assert_eq!(provider.provider_name(), "rig-anthropic");
        assert_eq!(provider.model_name(), "claude-sonnet-4-6");
    }

    #[test]
    fn unknown_provider_returns_error() {
        let engine = engine();
        let model = make_model("foo", "openai", None);
        let result = provider_for_model(&engine, &model);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("Unknown provider 'openai'"));
        assert!(err.contains("expected 'anthropic' or 'vllm'"));
    }

    #[test]
    fn vllm_without_endpoint_returns_error() {
        let engine = engine();
        let model = make_model("llama-3-8b", "vllm", None);
        let result = provider_for_model(&engine, &model);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("has no api_endpoint"));
    }
}
