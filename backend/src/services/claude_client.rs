//! HTTP client for the Anthropic Messages API.
//!
//! ## Pattern: Raw HTTP API integration
//! Instead of adding a dedicated Anthropic SDK crate, we use reqwest
//! (already a dependency) to call the REST API directly. This keeps the
//! dependency tree small and gives us full control over request/response
//! handling.
//!
//! The request/response types are private — callers only see `synthesize()`
//! and `SynthesisResult`.

use reqwest::Client;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Request types (private — only used to build the POST body)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ClaudeMessage>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

// ---------------------------------------------------------------------------
// Response types (private — parsed from JSON, then mapped to SynthesisResult)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// The result of a successful Claude synthesis call.
#[derive(Debug)]
pub struct SynthesisResult {
    pub answer: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Errors from the Claude API client.
#[derive(Debug, thiserror::Error)]
pub enum ClaudeError {
    #[error("API key not configured")]
    NoApiKey,

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("API error {status}: {body}")]
    ApiError { status: u16, body: String },

    #[error("Empty response from Claude")]
    EmptyResponse,
}

// ---------------------------------------------------------------------------
// Main function
// ---------------------------------------------------------------------------

/// Send a question to Claude with a system prompt and return the synthesis.
///
/// ## Pattern: Custom headers for API authentication
/// The Anthropic API requires two non-standard headers:
/// - `x-api-key`: the secret key for authentication
/// - `anthropic-version`: API version string (date-based)
///
/// reqwest's `.header()` builder method adds these before sending.
pub async fn synthesize(
    client: &Client,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_question: &str,
) -> Result<SynthesisResult, ClaudeError> {
    let request = ClaudeRequest {
        model: model.to_string(),
        max_tokens: 2048,
        system: system_prompt.to_string(),
        messages: vec![ClaudeMessage {
            role: "user".to_string(),
            content: user_question.to_string(),
        }],
    };

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await?;

    let status = response.status().as_u16();
    if status != 200 {
        let body = response.text().await.unwrap_or_default();
        return Err(ClaudeError::ApiError { status, body });
    }

    let claude_response: ClaudeResponse = response.json().await?;

    // Extract text blocks from the response.
    // Claude can return multiple content blocks; we join all text blocks.
    let answer = claude_response
        .content
        .iter()
        .filter(|block| block.content_type == "text")
        .filter_map(|block| block.text.as_deref())
        .collect::<Vec<_>>()
        .join("\n");

    if answer.is_empty() {
        return Err(ClaudeError::EmptyResponse);
    }

    Ok(SynthesisResult {
        answer,
        input_tokens: claude_response.usage.input_tokens,
        output_tokens: claude_response.usage.output_tokens,
    })
}
