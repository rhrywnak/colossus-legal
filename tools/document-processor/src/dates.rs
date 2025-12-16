//! Second-pass date enrichment for extracted claims.
//!
//! This module takes already-extracted claims and calls the LLM again
//! with a very small prompt per-claim to extract:
//! - asserted_date
//! - event_date
//! - date_confidence

use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use crate::claims::Claim;
use crate::config::Config;

/// Internal helper struct matching the expected JSON from the LLM.
#[derive(Debug, Deserialize)]
struct DateExtraction {
    pub asserted_date: Option<String>,
    pub event_date: Option<String>,
    pub date_confidence: Option<String>,
}

/// Enrich claims with date information by calling the LLM per-claim.
///
/// - `claims`: the vector of claims to mutate
/// - `config`: full config (used for Ollama URL and timeouts)
/// - `model_name`: which model to use (usually the same as extraction model)
pub async fn enrich_claim_dates(
    claims: &mut [Claim],
    config: &Config,
    model_name: &str,
) -> Result<()> {
    if claims.is_empty() {
        return Ok(());
    }

    let client = Client::new();

    for claim in claims.iter_mut() {
        // Build a small, focused prompt per claim
        let quote_escaped = claim.quote.replace('"', "\\\"");

        let prompt = format!(
"Extract date information from the following legal statement.

You must identify:
- asserted_date: the date when this statement was made, if visible
- event_date: the date of the event described in the statement, if visible
- date_confidence: \"high\", \"medium\", \"low\", or \"null\"

If a date is not visible, use null for that field.

Return ONLY valid JSON, no comments, no extra keys, exactly in this shape:

{{
  \"asserted_date\": \"YYYY-MM-DD or null\",
  \"event_date\": \"YYYY-MM-DD or null\",
  \"date_confidence\": \"high\" | \"medium\" | \"low\" | \"null\"
}}

Statement:
\"{quote}\"
",
            quote = quote_escaped
        );

        let response = client
            .post(format!("{}/api/generate", config.ollama.url))
            .json(&json!({
                "model": model_name,
                "prompt": prompt,
                "stream": false,
                "format": "json",
                "options": {
                    "temperature": 0.1,
                    "num_predict": 256
                }
            }))
            .timeout(Duration::from_secs(config.ollama.timeout_seconds))
            .send()
            .await
            .with_context(|| format!("Date extraction request failed for claim {}", claim.id))?;

        if !response.status().is_success() {
            // If date enrichment fails for this claim, leave its dates as-is and continue.
            continue;
        }

        let result: serde_json::Value = response
            .json()
            .await
            .with_context(|| format!("Failed to parse date extraction HTTP response for claim {}", claim.id))?;

        let Some(resp_text) = result.get("response").and_then(|v| v.as_str()) else {
            // Unexpected shape; skip this claim
            continue;
        };

        let cleaned = resp_text
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        // Try to parse into DateExtraction; if it fails, just skip this claim.
        if let Ok(info) = serde_json::from_str::<DateExtraction>(cleaned) {
            // Only overwrite if LLM actually provided something.
            if info.asserted_date.is_some() {
                claim.asserted_date = info.asserted_date;
            }
            if info.event_date.is_some() {
                claim.event_date = info.event_date;
            }
            if info.date_confidence.is_some() {
                claim.date_confidence = info.date_confidence;
            }
        }
    }

    Ok(())
}
