//! LLM client logic (Ollama, etc.).
//!
//! This module is responsible for:
//! - Building the prompt (via crate::prompt)
//! - Calling the Ollama HTTP API
//! - Parsing the JSON response into claims (via crate::claims)

use std::collections::HashSet;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde_json::json;

use crate::claims::{Claim, parse_claims};
use crate::prompt::build_prompt;

/// Normalize text for deduplication.
fn normalize_for_dedupe(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Split text into overlapping chunks (balanced defaults).
fn chunk_text(text: &str, max_chars: usize, overlap: usize) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }

    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    let mut chunks = Vec::new();
    let mut start = 0;

    while start < len {
        let end = usize::min(start + max_chars, len);
        let chunk: String = chars[start..end].iter().collect();
        chunks.push(chunk);

        if end == len {
            break;
        }

        start = end.saturating_sub(overlap);
    }

    chunks
}

/// Call the LLM to extract claims from a document.
///
/// This performs chunked extraction to reduce hallucinations:
/// - Document is split into overlapping chunks
/// - Each chunk is processed independently
/// - Results are merged and deduplicated
pub async fn extract_claims(
    text: &str,
    prompt_template: &str,
    document_name: &str,
    ollama_url: &str,
    model: &str,
    temperature: f32,
    num_predict: u32,
    timeout_seconds: u64,
) -> Result<Vec<Claim>> {
    let client = Client::new();

    // ---- Pass A: chunk the document ----
    let chunks = chunk_text(text, 1500, 200);

    let mut all_claims: Vec<Claim> = Vec::new();

    for (i, chunk_text) in chunks.iter().enumerate() {
        let scoped_text = format!(
            "THIS IS CHUNK {} OF {}.\n\
             ONLY extract claims explicitly stated in this text.\n\
             Do NOT infer or summarize.\n\n{}",
            i + 1,
            chunks.len(),
            chunk_text
        );

        let prompt = build_prompt(prompt_template, document_name, &scoped_text);

        let response = client
            .post(format!("{}/api/generate", ollama_url))
            .json(&json!({
                "model": model,
                "prompt": prompt,
                "stream": false,
                "format": "json",
                "options": {
                    "temperature": temperature,
                    "num_predict": num_predict,
                }
            }))
            .timeout(Duration::from_secs(timeout_seconds))
            .send()
            .await
            .context("Failed to call Ollama API")?;

        if !response.status().is_success() {
            bail!("Ollama returned error: {}", response.status());
        }

        let result: serde_json::Value =
            response.json().await.context("Failed to parse Ollama response")?;

        let response_text = result["response"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No response from LLM"))?;

        // Parse claims from this chunk
        let mut claims = parse_claims(response_text, document_name)?;
        all_claims.append(&mut claims);
    }

    // ---- Pass B: deduplicate by normalized quote ----
    let mut seen: HashSet<String> = HashSet::new();
    let mut deduped: Vec<Claim> = Vec::new();

    for claim in all_claims {
        let key = normalize_for_dedupe(&claim.quote);
        if key.is_empty() {
            continue;
        }
        if seen.insert(key) {
            deduped.push(claim);
        }
    }

    Ok(deduped)
}

