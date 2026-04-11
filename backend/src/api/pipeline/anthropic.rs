#![allow(dead_code)] // FP-5: chunk pipeline replaced the single-call path; kept for reference.
//! Anthropic Messages API client for extraction calls — via Rig.
//!
//! Uses rig-core's Anthropic provider, which is already proven to work
//! from inside our container (the RAG pipeline uses it successfully).
//! This replaces the raw reqwest approach which hung on Anthropic API calls.

use rig::client::CompletionClient;
use rig::completion::CompletionModel;
use rig::message::{AssistantContent, Text};

use crate::error::AppError;

/// Result of a successful Anthropic API call.
pub(super) struct ExtractionApiResult {
    pub text: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// Call the Anthropic Messages API using Rig's provider.
///
/// ## Why Rig instead of raw reqwest
///
/// Raw reqwest hung indefinitely when calling api.anthropic.com from inside
/// the container (likely HTTP/2 or TLS negotiation issue with Cloudflare).
/// Rig wraps reqwest internally but handles the Anthropic protocol correctly
/// and is proven to work — the RAG synthesizer uses the same code path.
///
/// ## Rig Concept: CompletionModel low-level API
///
/// We use `completion_request()` (not the Agent high-level API) because:
/// - We need token usage counts (input_tokens, output_tokens)
/// - Our prompt is the entire user message (no separate system prompt needed
///   for extraction — the instructions are baked into the prompt)
/// - We need to set max_tokens per-request (extraction needs 32K)
pub(super) async fn call_anthropic(
    api_key: &str,
    model: &str,
    max_tokens: u32,
    prompt: &str,
) -> Result<ExtractionApiResult, AppError> {
    // Create a Rig Anthropic client — this sets x-api-key and
    // anthropic-version headers automatically.
    let client = rig::providers::anthropic::Client::new(api_key)
        .map_err(|e| AppError::Internal {
            message: format!("Failed to create Anthropic client: {e}"),
        })?;

    let completion_model = client.completion_model(model);

    tracing::info!(
        prompt_len = prompt.len(),
        model = %model,
        max_tokens = max_tokens,
        "Sending Anthropic extraction request via Rig"
    );

    // Use the low-level completion API for token usage access.
    // max_tokens is REQUIRED for Anthropic (Rig enforces this).
    let response = completion_model
        .completion_request(prompt)
        .max_tokens(max_tokens as u64)
        .send()
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Anthropic API request failed: {e}"),
        })?;

    // Extract text from response.
    // response.choice is OneOrMany<AssistantContent>.
    let text = response
        .choice
        .into_iter()
        .filter_map(|content| match content {
            AssistantContent::Text(Text { text, .. }) => Some(text),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    if text.is_empty() {
        return Err(AppError::Internal {
            message: "Anthropic response contained no text content".to_string(),
        });
    }

    Ok(ExtractionApiResult {
        text,
        input_tokens: response.usage.input_tokens,
        output_tokens: response.usage.output_tokens,
    })
}
