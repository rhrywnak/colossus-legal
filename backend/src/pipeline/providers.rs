//! Per-document LLM provider construction.
//!
//! The global `AppContext.llm_provider` is used by the RAG pipeline (chat).
//! For extraction, we need per-document model selection — the user can
//! choose a different model for each document via the processing profile
//! or a per-document override.
//!
//! This module constructs an `LlmProvider` trait object from an
//! `llm_models` DB row. The concrete provider type (`AnthropicProvider`,
//! `VllmProvider`) is chosen based on the row's `provider` column.
//!
//! Design: DOC_PROCESSING_CONFIG_DESIGN_v2.md Section 3.8.

use colossus_extract::{AnthropicProvider, LlmProvider, VllmProvider};

use crate::repositories::pipeline_repository::LlmModelRecord;

/// Env var holding the Anthropic API key (read here, not from `AppContext`,
/// to keep the signature of this function clean for future per-model keys).
const ANTHROPIC_API_KEY_ENV: &str = "ANTHROPIC_API_KEY";

/// Env var holding an optional vLLM API key (authenticated deployments).
const VLLM_API_KEY_ENV: &str = "VLLM_API_KEY";

/// Default `max_tokens_default` for the provider when the `llm_models`
/// row has `max_output_tokens = NULL`. The per-call `invoke()` uses its
/// own `max_tokens` parameter, so this is only a fallback accessor value.
const FALLBACK_MAX_TOKENS: u32 = 8000;

/// Construct an `LlmProvider` trait object from a registered model row.
///
/// Dispatches on `model.provider`:
/// - `"anthropic"` → `AnthropicProvider::new` using `ANTHROPIC_API_KEY`
///   from the environment and the model id as the Anthropic model name.
/// - `"vllm"` → `VllmProvider::new` using `model.api_endpoint` as the
///   base URL (required for vLLM) and `VLLM_API_KEY` (optional).
///
/// Returns `Err` with a descriptive message if the provider string is
/// unknown, a required env var is missing, or a required `api_endpoint`
/// is missing for a vLLM row.
pub fn provider_for_model(model: &LlmModelRecord) -> Result<Box<dyn LlmProvider>, String> {
    let max_tokens_default = model
        .max_output_tokens
        .and_then(|n| u32::try_from(n).ok())
        .unwrap_or(FALLBACK_MAX_TOKENS);

    match model.provider.as_str() {
        "anthropic" => {
            let api_key = std::env::var(ANTHROPIC_API_KEY_ENV).map_err(|_| {
                format!("{ANTHROPIC_API_KEY_ENV} is not set — required for anthropic provider")
            })?;
            let provider =
                AnthropicProvider::new(api_key, model.id.clone(), max_tokens_default)
                    .map_err(|e| format!("AnthropicProvider::new failed: {e}"))?;
            Ok(Box::new(provider))
        }
        "vllm" => {
            let endpoint = model.api_endpoint.clone().ok_or_else(|| {
                format!(
                    "vLLM model '{}' has no api_endpoint — required for vllm provider",
                    model.id
                )
            })?;
            let api_key = std::env::var(VLLM_API_KEY_ENV).ok();
            let provider =
                VllmProvider::new(endpoint, model.id.clone(), api_key, max_tokens_default)
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
    use chrono::Utc;

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
        }
    }

    #[test]
    fn unknown_provider_returns_error() {
        let model = make_model("foo", "openai", None);
        let result = provider_for_model(&model);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("Unknown provider 'openai'"));
        assert!(err.contains("expected 'anthropic' or 'vllm'"));
    }

    #[test]
    fn vllm_without_endpoint_returns_error() {
        let model = make_model("llama-3-8b", "vllm", None);
        let result = provider_for_model(&model);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("has no api_endpoint"));
    }
}
