//! Anthropic chunk extractor — implements ChunkExtractor using rig's prompt_typed.
//!
//! This module handles per-chunk LLM extraction with three layers of defense:
//! 1. Structured output via rig's prompt_typed (guaranteed valid JSON)
//! 2. JSON repair via llm_json (fallback for providers without structured output)
//! 3. Retry with exponential backoff via backon (handles rate limits)
//!
//! ## Rust Learning: async_trait for trait implementation
//!
//! ChunkExtractor is defined in colossus-extract with #[async_trait].
//! Our implementation must also use #[async_trait] so the signatures match.
//! The trait is provider-agnostic — this file is the Anthropic-specific impl.

use std::time::Duration;

use async_trait::async_trait;
use backon::{ExponentialBuilder, Retryable};
use colossus_extract::{ChunkExtractionResult, ChunkExtractor, PipelineError};

/// Anthropic-powered chunk extractor using rig's structured output.
///
/// Each instance holds an API key and model name. The rig Client is
/// created per-call (it's cheap — just stores config, no connection pool).
pub struct AnthropicChunkExtractor {
    api_key: String,
    model: String,
    max_tokens: u64,
}

impl AnthropicChunkExtractor {
    pub fn new(api_key: String, model: String, max_tokens: u64) -> Self {
        Self {
            api_key,
            model,
            max_tokens,
        }
    }

    /// Build the extraction prompt from template + schema + chunk text.
    ///
    /// This is a simple string substitution — the prompt template has
    /// placeholders for {{schema_json}}, {{chunk_text}}, and {{examples}}.
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

    /// Single extraction attempt — called by the retry wrapper.
    ///
    /// Tries structured output first (prompt_typed). If that fails,
    /// falls back to raw completion + JSON repair.
    async fn try_extract(
        &self,
        prompt: &str,
    ) -> Result<ChunkExtractionResult, PipelineError> {
        use rig::client::CompletionClient;

        let client = rig::providers::anthropic::Client::new(&self.api_key)
            .map_err(|e| PipelineError::LlmProvider(format!(
                "Failed to create Anthropic client: {e}"
            )))?;

        let agent = client
            .agent(&self.model)
            .preamble("You extract entities and relationships from text. Follow the schema exactly.")
            .max_tokens(self.max_tokens)
            .temperature(0.0)
            .build();

        // Try structured output first (prompt_typed)
        use rig::completion::TypedPrompt;
        let typed_result: Result<ChunkExtractionResult, _> = agent
            .prompt_typed(prompt)
            .await;

        match typed_result {
            Ok(result) => {
                tracing::debug!(
                    nodes = result.nodes.len(),
                    rels = result.relationships.len(),
                    "Structured output extraction succeeded"
                );
                Ok(result)
            }
            Err(typed_err) => {
                tracing::warn!(
                    error = %typed_err,
                    "Structured output failed, falling back to raw completion + JSON repair"
                );

                // Fallback: raw completion + JSON repair
                self.extract_with_repair(&client, prompt).await
            }
        }
    }

    /// Fallback extraction: raw completion + llm_json repair.
    ///
    /// Used when structured output fails (e.g., schema too complex,
    /// or provider doesn't support it).
    async fn extract_with_repair(
        &self,
        client: &rig::providers::anthropic::Client,
        prompt: &str,
    ) -> Result<ChunkExtractionResult, PipelineError> {
        use rig::client::CompletionClient;
        use rig::completion::Prompt;

        let agent = client
            .agent(&self.model)
            .preamble("You extract entities and relationships from text. Return ONLY valid JSON with 'nodes' and 'relationships' arrays. No markdown, no explanation.")
            .max_tokens(self.max_tokens)
            .temperature(0.0)
            .build();

        let raw_text: String = agent
            .prompt(prompt)
            .await
            .map_err(|e| PipelineError::LlmProvider(format!(
                "Raw completion failed: {e}"
            )))?;

        // Try direct parse first
        if let Ok(result) = serde_json::from_str::<ChunkExtractionResult>(&raw_text) {
            tracing::debug!("Raw JSON parsed directly (no repair needed)");
            return Ok(result);
        }

        // JSON repair
        let repaired = llm_json::repair_json(&raw_text, &Default::default())
            .map_err(|e| PipelineError::Extraction(format!(
                "JSON repair failed: {e}"
            )))?;

        serde_json::from_str::<ChunkExtractionResult>(&repaired)
            .map_err(|e| PipelineError::Extraction(format!(
                "Repaired JSON still invalid: {e}. Preview: {}",
                &repaired[..repaired.len().min(200)]
            )))
    }
}

#[async_trait]
impl ChunkExtractor for AnthropicChunkExtractor {
    async fn extract_chunk(
        &self,
        chunk_text: &str,
        schema_json: &serde_json::Value,
        prompt_template: &str,
        examples: &str,
    ) -> Result<ChunkExtractionResult, PipelineError> {
        let prompt = self.build_prompt(chunk_text, schema_json, prompt_template, examples);

        // Retry with exponential backoff (handles 429 rate limits
        // and transient API errors)
        (|| async { self.try_extract(&prompt).await })
            .retry(
                ExponentialBuilder::default()
                    .with_min_delay(Duration::from_secs(1))
                    .with_max_delay(Duration::from_secs(60))
                    .with_max_times(3),
            )
            .when(|e| {
                let msg = format!("{e:?}");
                msg.contains("429")
                    || msg.contains("rate limit")
                    || msg.contains("too many requests")
                    || msg.contains("overloaded")
                    || msg.contains("timeout")
            })
            .notify(|err, dur: Duration| {
                tracing::warn!(
                    error = %err,
                    retry_after_secs = dur.as_secs(),
                    "Retrying chunk extraction after error"
                );
            })
            .await
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}
