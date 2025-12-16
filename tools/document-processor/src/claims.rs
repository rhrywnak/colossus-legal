//! Claim parsing and JSON salvage logic.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents a single legal claim extracted from a document.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claim {
    pub id: String,
    pub quote: String,

    /// Grounding anchors (required by prompt; defaulted here so parsing never hard-fails)
    #[serde(default)]
    pub anchor_before: String,
    #[serde(default)]
    pub anchor_after: String,

    pub made_by: String,
    pub page: Option<i32>,
    pub topic: String,
    pub severity: i32,

    #[serde(default)]
    pub asserted_date: Option<String>,
    #[serde(default)]
    pub event_date: Option<String>,
    #[serde(default)]
    pub date_confidence: Option<String>,

    /// Injected by the pipeline, not the LLM.
    #[serde(default)]
    pub source_document: String,
}

/// Top-level LLM response shape.
#[derive(Debug, Serialize, Deserialize)]
struct ClaimResponse {
    pub claims: Vec<Claim>,
}

/// Parse claims from an LLM response string.
///
/// - strips markdown fences
/// - isolates JSON
/// - attempts strict parse
/// - attempts salvage as fallback
pub fn parse_claims(response_text: &str, document_name: &str) -> Result<Vec<Claim>> {
    let cleaned = response_text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let json_start = cleaned.find('{').unwrap_or(0);
    let json_end = cleaned
        .rfind('}')
        .map(|i| i + 1)
        .unwrap_or(cleaned.len());

    let json_str = &cleaned[json_start..json_end];

    match serde_json::from_str::<ClaimResponse>(json_str) {
        Ok(mut response) => {
            for claim in &mut response.claims {
                claim.source_document = document_name.to_string();
            }
            Ok(response.claims)
        }
        Err(first_err) => {
            let snippet: String = json_str.chars().take(500).collect();

            match salvage_claims(json_str, document_name) {
                Ok(claims) => {
                    eprintln!(
                        "JSON salvage: initial parse failed ({}), salvaged {} claims",
                        first_err,
                        claims.len()
                    );
                    Ok(claims)
                }
                Err(salvage_err) => Err(anyhow::anyhow!(
                    "Failed to parse JSON. First 500 chars:\n{}\n\nInitial error: {}\nSalvage error: {}",
                    snippet,
                    first_err,
                    salvage_err,
                )),
            }
        }
    }
}

/// Salvage claims from malformed or truncated JSON.
///
/// Conservative: accepts only individually valid Claim objects.
/// Returns Ok(vec![]) if nothing salvageable.
fn salvage_claims(json_str: &str, document_name: &str) -> Result<Vec<Claim>> {
    let value: Value = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(_) => {
            let wrapped = salvage_claims_json(json_str)?;
            serde_json::from_str(&wrapped)?
        }
    };

    let claims_array = value
        .get("claims")
        .and_then(|v| v.as_array())
        .or_else(|| value.as_array())
        .ok_or_else(|| anyhow::anyhow!("No claims array found during salvage"))?;

    let mut claims = Vec::new();
    let mut rejected = 0usize;

    for (idx, item) in claims_array.iter().enumerate() {
        match serde_json::from_value::<Claim>(item.clone()) {
            Ok(mut claim) => {
                claim.source_document = document_name.to_string();
                claims.push(claim);
            }
            Err(e) => {
                rejected += 1;
                eprintln!("Salvage rejected claim {}: {}", idx, e);
            }
        }
    }

    if rejected > 0 {
        eprintln!(
            "Salvage accepted {} claims, rejected {} malformed entries",
            claims.len(),
            rejected
        );
    }

    Ok(claims)
}

/// Attempt to extract and wrap a partial `"claims"` array into valid JSON.
fn salvage_claims_json(json_str: &str) -> Result<String> {
    let claims_key_pos = json_str
        .find("\"claims\"")
        .or_else(|| json_str.find("'claims'"))
        .ok_or_else(|| anyhow::anyhow!("'claims' key not found"))?;

    let after_claims = &json_str[claims_key_pos..];

    let array_start_rel = after_claims
        .find('[')
        .ok_or_else(|| anyhow::anyhow!("'[' for claims array not found"))?;
    let array_start = claims_key_pos + array_start_rel;

    let last_brace = json_str
        .rfind('}')
        .filter(|idx| *idx > array_start)
        .ok_or_else(|| anyhow::anyhow!("No closing '}}' after claims array start"))?;

    if last_brace <= array_start {
        return Err(anyhow::anyhow!(
            "Closing '}}' occurs before claims array start"
        ));
    }

    let body = &json_str[array_start + 1..=last_brace];

    let mut trimmed = body.trim().to_string();
    while trimmed.ends_with(',') || trimmed.ends_with('\n') || trimmed.ends_with('\r') || trimmed.ends_with(' ') {
        trimmed.pop();
    }

    Ok(format!(r#"{{"claims":[{}]}}"#, trimmed))
}
