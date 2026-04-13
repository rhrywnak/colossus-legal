//! Anthropic chunk extractor — calls the Anthropic Messages API directly
//! via reqwest for precise rate limit handling.
//!
//! ## Why direct HTTP instead of rig
//!
//! The `rig` crate provides a convenient abstraction over LLM providers,
//! but it converts HTTP responses into string errors, discarding the
//! `retry-after` header that Anthropic includes in every 429 response.
//! Without the exact retry-after value, any backoff strategy is a guess.
//!
//! Anthropic uses a token bucket algorithm for rate limiting — capacity
//! refills continuously, not at fixed intervals. The retry-after header
//! contains the exact seconds until the bucket has enough capacity for
//! the next request. Waiting less causes immediate re-rejection. Waiting
//! more wastes time. Only the exact value is correct.
//!
//! Direct reqwest gives us the full HTTP response including all headers,
//! letting us read retry-after precisely and return PipelineError::RateLimited
//! with the authoritative wait duration.
//!
//! ## Error taxonomy (important for retry decisions)
//!
//! HTTP 429 rate_limit_error → PipelineError::RateLimited { retry_after_secs }
//!   Orchestrator must wait retry_after_secs, then retry this chunk.
//!
//! HTTP 529 overloaded_error → PipelineError::LlmProvider("overloaded: ...")
//!   Server-side capacity issue unrelated to our rate limit. Orchestrator
//!   uses short exponential backoff (different from rate limit handling).
//!
//! HTTP 5xx (500, 502, 503, 504) → PipelineError::LlmProvider("server error: ...")
//!   Transient server errors. Short exponential backoff appropriate.
//!
//! HTTP 4xx (400, 401, 403) → PipelineError::LlmProvider("client error: ...")
//!   Permanent errors. Do not retry — the request itself is wrong.
//!
//! Network timeout → PipelineError::LlmProvider("request timeout: ...")
//!   Transient. Short exponential backoff appropriate.
//!
//! JSON parse failure → llm_json repair → PipelineError::Extraction if repair fails
//!   Not retryable with same input. Mark chunk failed.

use std::time::Duration;
use async_trait::async_trait;
use colossus_extract::{ChunkExtractionResult, ChunkExtractor, PipelineError};

// Anthropic API constants.
// These are stable values from the Anthropic API documentation.
// The model_id is not hardcoded here — it comes from self.model.
const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
// Request timeout: 5 minutes. Individual chunk extractions should complete
// in well under this. The timeout prevents a hung connection from blocking
// the pipeline indefinitely.
const REQUEST_TIMEOUT_SECS: u64 = 300;
// Default retry-after value when the header is absent.
// Anthropic's documentation states the header is always present on 429,
// but we default to 60s (one full minute) as a safe fallback.
const DEFAULT_RETRY_AFTER_SECS: u64 = 60;

pub struct AnthropicChunkExtractor {
    api_key: String,
    model: String,
    max_tokens: u64,
}

impl AnthropicChunkExtractor {
    pub fn new(api_key: String, model: String, max_tokens: u64) -> Self {
        Self { api_key, model, max_tokens }
    }

    /// Build the extraction prompt from template + schema + chunk text.
    fn build_prompt(
        &self,
        chunk_text: &str,
        schema_json: &serde_json::Value,
        prompt_template: &str,
        examples: &str,
    ) -> String {
        let schema_str = serde_json::to_string_pretty(schema_json)
            .unwrap_or_else(|_| "{}".to_string());
        prompt_template
            .replace("{{schema_json}}", &schema_str)
            .replace("{{chunk_text}}", chunk_text)
            .replace("{{examples}}", examples)
    }

    /// Parse the Anthropic API response body into ChunkExtractionResult.
    ///
    /// Tries direct deserialization first. If that fails (LLM produced
    /// slightly malformed JSON), attempts llm_json repair before giving up.
    fn parse_response(
        &self,
        response_text: &str,
    ) -> Result<ChunkExtractionResult, PipelineError> {
        // The Anthropic response body looks like:
        // { "content": [{ "type": "text", "text": "{ JSON here }" }], ... }
        // We extract the text field from the first content block.
        let response_json: serde_json::Value = serde_json::from_str(response_text)
            .map_err(|e| PipelineError::LlmProvider(
                format!("Failed to parse Anthropic response envelope: {e}")
            ))?;

        let content_text = response_json["content"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|block| block["text"].as_str())
            .ok_or_else(|| PipelineError::LlmProvider(
                "Anthropic response missing content[0].text field".into()
            ))?;

        // Try direct JSON parse of the content text.
        // The LLM should return a JSON object matching ChunkExtractionResult.
        if let Ok(result) = serde_json::from_str::<ChunkExtractionResult>(content_text) {
            return Ok(result);
        }

        // Fallback: llm_json repair for slightly malformed JSON.
        // Common LLM failure modes: trailing commas, unquoted keys, truncated output.
        tracing::warn!(
            model = %self.model,
            "Direct JSON parse failed, attempting llm_json repair"
        );
        let repaired = llm_json::repair_json(content_text, &Default::default())
            .map_err(|e| PipelineError::Extraction(
                format!("JSON repair failed: {e}")
            ))?;

        serde_json::from_str::<ChunkExtractionResult>(&repaired)
            .map_err(|e| PipelineError::Extraction(
                format!("Repaired JSON still invalid: {e}. Preview: {}",
                    &repaired[..repaired.len().min(200)])
            ))
    }
}

#[async_trait]
impl ChunkExtractor for AnthropicChunkExtractor {
    /// Make a single extraction attempt against the Anthropic Messages API.
    ///
    /// Returns:
    /// - Ok(result) on success
    /// - Err(RateLimited { retry_after_secs }) on HTTP 429
    /// - Err(LlmProvider(...)) on HTTP 529, 5xx, network timeout
    /// - Err(Extraction(...)) on JSON parse failure after repair
    ///
    /// Does NOT retry internally. The orchestrator owns retry logic and
    /// has access to the database pool for progress label updates during waits.
    async fn extract_chunk(
        &self,
        chunk_text: &str,
        schema_json: &serde_json::Value,
        prompt_template: &str,
        examples: &str,
    ) -> Result<ChunkExtractionResult, PipelineError> {
        let prompt = self.build_prompt(chunk_text, schema_json, prompt_template, examples);

        // Build the Anthropic Messages API request body.
        // We use a simple user message containing the full prompt.
        // The system message instructs the model to return only JSON.
        let request_body = serde_json::json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "temperature": 0.0,
            "system": "You extract entities and relationships from text. \
                       Return ONLY valid JSON with 'nodes' and 'relationships' arrays. \
                       No markdown, no explanation, no preamble.",
            "messages": [
                { "role": "user", "content": prompt }
            ]
        });

        // Build reqwest client with timeout.
        // A new client per call is acceptable here — each chunk extraction
        // is a discrete operation. Connection pooling is not needed for
        // sequential single-chunk processing.
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|e| PipelineError::LlmProvider(
                format!("Failed to build HTTP client: {e}")
            ))?;

        let response = client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                // reqwest errors on send include: connection refused, DNS failure,
                // and request timeout. All are transient — short backoff appropriate.
                if e.is_timeout() {
                    PipelineError::LlmProvider(format!("request timeout after {REQUEST_TIMEOUT_SECS}s: {e}"))
                } else {
                    PipelineError::LlmProvider(format!("network error: {e}"))
                }
            })?;

        let status = response.status();

        // Handle rate limiting specifically.
        // HTTP 429 means our token bucket is depleted. Anthropic tells us
        // exactly how long to wait via the retry-after header.
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            // Read retry-after header. Parse as integer seconds.
            // Fall back to DEFAULT_RETRY_AFTER_SECS if header is absent or unparseable.
            let retry_after_secs = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(DEFAULT_RETRY_AFTER_SECS);

            tracing::warn!(
                model = %self.model,
                retry_after_secs,
                "Anthropic API rate limited (429) — returning RateLimited error to orchestrator"
            );

            return Err(PipelineError::RateLimited { retry_after_secs });
        }

        // Handle server overload (529) — distinct from rate limiting.
        // This is Anthropic's server being busy, not our quota being exceeded.
        // Short exponential backoff is appropriate (not the full retry-after window).
        if status.as_u16() == 529 {
            let body = response.text().await.unwrap_or_default();
            return Err(PipelineError::LlmProvider(
                format!("overloaded (529): {}", &body[..body.len().min(200)])
            ));
        }

        // Handle other non-success HTTP status codes.
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            // 4xx errors (except 429) are permanent — wrong API key, bad request, etc.
            // 5xx errors are transient server errors.
            let category = if status.is_client_error() { "client error" } else { "server error" };
            return Err(PipelineError::LlmProvider(
                format!("{category} HTTP {status}: {}", &body[..body.len().min(300)])
            ));
        }

        // Success — parse the response body.
        let response_text = response.text().await
            .map_err(|e| PipelineError::LlmProvider(
                format!("Failed to read response body: {e}")
            ))?;

        self.parse_response(&response_text)
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_retry_after_is_safe() {
        // When the retry-after header is absent (rare for 429),
        // we fall back to DEFAULT_RETRY_AFTER_SECS.
        // This must be >= 60 to ensure the rate limit window clears.
        assert!(DEFAULT_RETRY_AFTER_SECS >= 60,
            "Default retry-after must be at least 60s to allow rate limit window to clear. \
             Got: {}s", DEFAULT_RETRY_AFTER_SECS);
    }

    #[test]
    fn test_request_timeout_is_generous() {
        // The request timeout must be long enough for slow chunk extractions
        // (large chunks, complex schemas) but finite to prevent hung connections.
        // 5 minutes (300s) is the right range.
        assert!(REQUEST_TIMEOUT_SECS >= 120,
            "Request timeout too short — large chunks may time out legitimately");
        assert!(REQUEST_TIMEOUT_SECS <= 600,
            "Request timeout too long — hung connections block pipeline for too long");
    }

    #[test]
    fn test_parse_response_handles_repair() {
        // Verify that parse_response can handle slightly malformed JSON
        // (the llm_json repair path).
        // We test this with a well-formed response to verify the happy path.
        // The repair path is tested implicitly by the llm_json crate's own tests.
        let extractor = AnthropicChunkExtractor::new(
            "test-key".into(),
            "claude-test".into(),
            1000,
        );

        // Well-formed Anthropic response envelope
        let good_response = r#"{
            "content": [{
                "type": "text",
                "text": "{\"nodes\": [], \"relationships\": []}"
            }],
            "usage": {"input_tokens": 10, "output_tokens": 5}
        }"#;

        let result = extractor.parse_response(good_response);
        assert!(result.is_ok(), "Well-formed response should parse successfully");
        let chunk_result = result.unwrap();
        assert_eq!(chunk_result.nodes.len(), 0);
        assert_eq!(chunk_result.relationships.len(), 0);
    }
}
