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

use crate::domain::llm_effort::Effort;
use crate::domain::llm_params::construction_temperature;
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

/// Construct an `LlmProvider` trait object from a registered model row.
///
/// Dispatches on `model.provider`:
/// - `"anthropic"` → [`RigLlmProviderBridge`] wrapping the supplied
///   shared engine. Uses the model id from the DB row, the row's cost
///   columns, and derives the construction temperature FROM THE ROW via
///   [`construction_temperature`] — honoring `temperature_mode` (an `omit`
///   model, e.g. `claude-opus-4-7`, sends NO temperature; a `zero-ok`/unmarked
///   model gets its `default_temperature`, else the deterministic `Some(0.0)`
///   extraction has always used). This REPLACES the old hardcoded
///   `Some(0.0)`, which ignored the column and 400-ed temperature-deprecated
///   models.
/// - `"vllm"` → `VllmProvider::new` using `model.api_endpoint` as the
///   base URL (required for vLLM) and `VLLM_API_KEY` (optional).
///
/// `engine` is the shared `Arc<dyn ExtractionEngine>` constructed once
/// at startup in `AppContext` — see P1-5 for the wiring. The same
/// engine instance is used across every per-document bridge.
///
/// `effort` is the CALLER'S call family, not the model's: extraction passes
/// `policy.extraction` (turned down to `low` by default) and the Theme Scan and
/// practice reader pass `policy.scan` (absent unless asked for). It is a
/// parameter rather than something derived here because this function cannot
/// see which family is asking, and guessing from the model id would be exactly
/// the kind of inference that makes a scan quietly inherit an extraction
/// setting. See [`crate::domain::llm_effort`] for the ruling. `None` sends no
/// `output_config.effort` field at all — the vLLM branch ignores it entirely,
/// since the parameter is Anthropic's.
///
/// Returns `Err` with a descriptive message if the provider string is
/// unknown or a required `api_endpoint` is missing for a vLLM row.
pub fn provider_for_model(
    engine: &Arc<dyn ExtractionEngine>,
    model: &LlmModelRecord,
    effort: Option<Effort>,
) -> Result<Box<dyn LlmProvider>, String> {
    match model.provider.as_str() {
        "anthropic" => {
            // The bridge does NOT consume max_tokens at construction —
            // each `invoke` call passes its own max_tokens. Cost columns
            // and temperature ARE constructor-time: costs are returned
            // verbatim via the LlmProvider accessor; temperature is derived
            // from the row's `temperature_mode` / `default_temperature` (the
            // single source of truth in `domain::llm_params`) so a
            // temperature-deprecated model omits the key instead of 400-ing.
            // A malformed `temperature_mode` token is a loud, named error here.
            let temperature = construction_temperature(model).map_err(|e| {
                format!("model '{}' has an invalid temperature_mode: {e}", model.id)
            })?;
            let bridge = RigLlmProviderBridge::new(
                Arc::clone(engine),
                model.id.clone(),
                model.cost_per_input_token,
                model.cost_per_output_token,
                temperature,
                effort,
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
mod wiring_tests {
    //! Which call family passes which effort — asserted against the source.
    //!
    //! The dial is easy to add and easy to leave unwired: a `provider_for_model`
    //! call that passes `None` compiles, runs, and looks exactly like one that
    //! passes the policy, right up until a 727-second thinking pass returns no
    //! text again. These fences are what say it actually reached both families,
    //! and that neither took the other's setting.

    use std::fs;
    use std::path::Path;

    fn read(relative: &str) -> String {
        fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join(relative),
        )
        .unwrap_or_else(|e| panic!("{relative} must be readable: {e}"))
    }

    #[test]
    fn both_extraction_passes_send_the_extraction_effort() {
        for file in [
            "pipeline/steps/llm_extract.rs",
            "pipeline/steps/llm_extract_pass2.rs",
        ] {
            let source = read(file);
            assert!(
                source.contains("llm_effort_policy.extraction"),
                "{file} must pass the EXTRACTION effort — this is the call family the \
                 2026-08-28 incident happened on"
            );
            assert!(
                !source.contains("llm_effort_policy.scan"),
                "{file} must not reach for the scan setting"
            );
        }
    }

    #[test]
    fn the_scan_and_the_practice_reader_send_the_scan_effort() {
        // Absent unless `LLM_SCAN_EFFORT` is set. If either of these ever read
        // `.extraction`, a judgement-shaped call would silently inherit `low`
        // and get shallower — a quality change nobody asked for.
        for file in [
            "services/theme_scan_provider.rs",
            "services/practice_read_setup.rs",
        ] {
            let source = read(file);
            assert!(
                source.contains("llm_effort_policy.scan"),
                "{file} must pass the SCAN effort"
            );
            assert!(
                !source.contains("llm_effort_policy.extraction"),
                "{file} must not inherit extraction's turned-down setting"
            );
        }
    }

    #[test]
    fn the_rag_bridge_is_left_alone() {
        // `AppContext.llm_provider` backs the synthesizer and the decomposer,
        // which are neither family. The ruling covered two call families; a
        // third quietly joining one of them is the drift this catches.
        let context = read("pipeline/context.rs");
        assert!(
            !context.contains("llm_effort_policy.extraction")
                && !context.contains("llm_effort_policy.scan"),
            "the RAG bridge must keep today's behaviour — no effort field"
        );
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
            _effort: Option<crate::domain::llm_effort::Effort>,
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
            // Not read by this path; the fixture asserts nothing about it.
            billing_class: "local".to_string(),
        }
    }

    #[test]
    fn anthropic_returns_bridge_named_rig_anthropic() {
        let engine = engine();
        let model = make_model("claude-sonnet-4-6", "anthropic", None);
        let provider = provider_for_model(&engine, &model, None)
            .expect("anthropic provider should construct from a shared engine");
        // Sanity-check the bridge's accessors so a future refactor that
        // accidentally routes anthropic through the legacy path
        // (provider_name = "anthropic") fails this test.
        assert_eq!(provider.provider_name(), "rig-anthropic");
        assert_eq!(provider.model_name(), "claude-sonnet-4-6");
    }

    #[test]
    fn anthropic_with_malformed_temperature_mode_returns_error() {
        // A corrupt `temperature_mode` token must propagate as a loud, named error
        // out of provider_for_model (not a silent Some(0.0) fallback). The
        // construction_temperature error is wrapped here naming the model.
        let engine = engine();
        let mut model = make_model("claude-opus-4-7", "anthropic", None);
        model.temperature_mode = Some("bad-token".to_string());
        let result = provider_for_model(&engine, &model, None);
        assert!(result.is_err());
        let msg = result.err().unwrap();
        assert!(
            msg.contains("claude-opus-4-7"),
            "error should name the model: {msg}"
        );
        assert!(
            msg.contains("invalid temperature_mode"),
            "error should name the cause: {msg}"
        );
    }

    #[test]
    fn unknown_provider_returns_error() {
        let engine = engine();
        let model = make_model("foo", "openai", None);
        let result = provider_for_model(&engine, &model, None);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("Unknown provider 'openai'"));
        assert!(err.contains("expected 'anthropic' or 'vllm'"));
    }

    #[test]
    fn vllm_without_endpoint_returns_error() {
        let engine = engine();
        let model = make_model("llama-3-8b", "vllm", None);
        let result = provider_for_model(&engine, &model, None);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("has no api_endpoint"));
    }
}
