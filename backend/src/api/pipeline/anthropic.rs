//! Anthropic Messages API client for extraction calls.
//!
//! ## Rust Learning: Direct HTTP API call vs SDK
//!
//! Rather than pulling in the full Anthropic SDK, we call the Messages API
//! directly via reqwest. This keeps our dependency tree small and gives us
//! full control over timeouts and error handling.

use serde::{Deserialize, Serialize};

use crate::error::AppError;

// ── Anthropic API types ──────────────────────────────────────────

#[derive(Serialize)]
pub(super) struct AnthropicRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<AnthropicMessage>,
}

#[derive(Serialize)]
pub(super) struct AnthropicMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
pub(super) struct AnthropicResponse {
    pub content: Vec<AnthropicContent>,
    pub usage: AnthropicUsage,
}

#[derive(Deserialize)]
pub(super) struct AnthropicContent {
    #[serde(rename = "type")]
    pub _content_type: String,
    pub text: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct AnthropicUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

// ── API call ─────────────────────────────────────────────────────

/// Call the Anthropic Messages API. Returns (response_text, usage).
///
/// Builds a dedicated reqwest client per call with pool_max_idle_per_host(0)
/// to prevent connection reuse — each extraction gets a fresh connection.
pub(super) async fn call_anthropic(
    api_key: &str,
    model: &str,
    max_tokens: u32,
    prompt: &str,
) -> Result<(String, AnthropicUsage), AppError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180))
        .connect_timeout(std::time::Duration::from_secs(10))
        .pool_max_idle_per_host(0)
        .build()
        .map_err(|e| AppError::Internal {
            message: format!("Failed to build extraction HTTP client: {e}"),
        })?;

    let request_body = AnthropicRequest {
        model: model.to_string(),
        max_tokens,
        messages: vec![AnthropicMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
    };

    tracing::info!(
        body_len = serde_json::to_string(&request_body).unwrap_or_default().len(),
        "Sending Anthropic request"
    );

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| AppError::Internal {
            message: format!("Anthropic API request failed: {e}"),
        })?;

    let status = response.status();
    if !status.is_success() {
        let error_body = response.text().await.unwrap_or_default();
        return Err(AppError::Internal {
            message: format!("Anthropic API returned {status}: {error_body}"),
        });
    }

    let api_response: AnthropicResponse =
        response.json().await.map_err(|e| AppError::Internal {
            message: format!("Failed to parse Anthropic response: {e}"),
        })?;

    let text = api_response
        .content
        .into_iter()
        .find_map(|c| c.text)
        .ok_or_else(|| AppError::Internal {
            message: "Anthropic response contained no text content".to_string(),
        })?;

    Ok((text, api_response.usage))
}
