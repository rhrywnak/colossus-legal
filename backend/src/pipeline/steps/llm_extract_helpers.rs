//! Helper functions extracted from `llm_extract.rs`.
//!
//! This sibling module holds the rate-limit retry wrapper, response-parsing
//! utilities, and best-effort failure recorders. Keeping them here lets
//! `llm_extract.rs` focus on orchestration (config resolution, run insert,
//! chunk-vs-full dispatch) while the general-purpose helpers remain
//! reusable and independently testable.

use sqlx::PgPool;

use crate::repositories::pipeline_repository::extraction;

// The rate-limit retry wrapper moved to the crate root (`crate::llm_retry`) so
// the Theme Scan service can share it without importing a pipeline step's
// internals (an inverted layering dependency). Re-exported here under the
// original path so this module's existing callers (`llm_extract`,
// `llm_extract_pass2`) keep their `use super::call_with_rate_limit_retry`
// imports unchanged.
pub(crate) use crate::llm_retry::call_with_rate_limit_retry;

/// Parse an LLM response as a JSON Value containing entities and relationships.
///
/// Tries direct `serde_json` parse first. On failure, strips markdown fences
/// and uses `llm_json::repair_json` for repair, then retries parse. In either
/// path, the parsed result must be a JSON object — see [`ensure_object`].
pub(crate) fn parse_chunk_response(text: &str) -> Result<serde_json::Value, String> {
    let stripped = strip_markdown_fences(text);

    // Direct parse.
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&stripped) {
        return ensure_object(val);
    }

    // Repair + parse.
    match llm_json::repair_json(&stripped, &Default::default()) {
        Ok(repaired) => {
            let val: serde_json::Value = serde_json::from_str(&repaired)
                .map_err(|e| format!("JSON repair succeeded but parse still failed: {e}"))?;
            ensure_object(val)
        }
        Err(repair_err) => {
            let preview = &stripped[..stripped.len().min(200)];
            Err(format!(
                "JSON parse and repair both failed. Repair error: {repair_err}. Preview: {preview}"
            ))
        }
    }
}

/// Gate parsed LLM output on a top-level JSON object.
///
/// `llm_json::repair_json` is aggressively permissive — it coerces arbitrary
/// prose into a valid JSON string primitive rather than failing. Without this
/// check, `parsed["entities"].as_array()` silently returns `None` downstream
/// and the chunk counts as a zero-entity success instead of a parse failure.
pub(crate) fn ensure_object(val: serde_json::Value) -> Result<serde_json::Value, String> {
    if !val.is_object() {
        let kind = match &val {
            serde_json::Value::String(_) => "string",
            serde_json::Value::Array(_) => "array",
            serde_json::Value::Number(_) => "number",
            serde_json::Value::Bool(_) => "bool",
            serde_json::Value::Null => "null",
            _ => "unknown",
        };
        return Err(format!(
            "LLM returned valid JSON but not an object (got {kind})"
        ));
    }
    Ok(val)
}

/// Strip leading/trailing markdown code fences.
pub(crate) fn strip_markdown_fences(text: &str) -> String {
    let t = text.trim();
    let t = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```"))
        .unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t);
    t.trim().to_string()
}

/// Log-and-ignore writer used when the step is about to return `Err`.
///
/// We want the DB record to reflect the failure, but we don't want a
/// secondary write error to mask the primary cause.
pub(crate) async fn mark_run_failed(db: &PgPool, run_id: i32, reason: &str) {
    if let Err(e) = extraction::complete_extraction_run(
        db,
        run_id,
        &serde_json::json!({"error": reason}),
        None,
        None,
        None,
        crate::models::document_status::RUN_STATUS_FAILED,
    )
    .await
    {
        tracing::warn!(run_id, error = %e, reason, "mark_run_failed: DB write failed (non-fatal)");
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_markdown_fences_routing() {
        // Routing table: input → expected stripped output. Four rows
        // covering all observed shapes:
        // - ```json fenced
        // - ``` plain fenced
        // - no fences (passthrough)
        // - whitespace-then-fences-then-whitespace (trim + strip)
        let cases: &[(&str, &str)] = &[
            ("```json\n{\"entities\":[]}\n```", "{\"entities\":[]}"),
            ("```\n{\"entities\":[]}\n```", "{\"entities\":[]}"),
            ("{\"entities\":[]}", "{\"entities\":[]}"),
            ("  \n```json\n{\"a\":1}\n```\n  ", "{\"a\":1}"),
        ];
        for (input, expected) in cases {
            assert_eq!(
                strip_markdown_fences(input),
                *expected,
                "strip_markdown_fences({input:?}) should be {expected:?}"
            );
        }
    }

    #[test]
    fn test_parse_repairable_json() {
        // Trailing comma is invalid JSON but llm_json can repair it.
        let input = r#"{"entities": [{"id": "0", "entity_type": "Party"},], "relationships": []}"#;
        let result = parse_chunk_response(input);
        assert!(
            result.is_ok(),
            "Expected repair to succeed, got: {result:?}"
        );
    }

    #[test]
    fn test_parse_garbage_fails() {
        let input = "This is not JSON at all, just random text from the LLM.";
        let result = parse_chunk_response(input);
        assert!(result.is_err());
    }

    #[test]
    fn ensure_object_rejects_bare_string() {
        let v = serde_json::Value::String("hello".into());
        assert!(ensure_object(v).is_err());
    }
}
