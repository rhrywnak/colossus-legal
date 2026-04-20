//! Helper functions extracted from `llm_extract.rs`.
//!
//! This sibling module holds the rate-limit retry wrapper, response-parsing
//! utilities, and best-effort failure recorders. Keeping them here lets
//! `llm_extract.rs` focus on orchestration (config resolution, run insert,
//! chunk-vs-full dispatch) while the general-purpose helpers remain
//! reusable and independently testable.

use sqlx::PgPool;
use tokio::time::Duration;

use colossus_extract::{LlmProvider, LlmResponse, PipelineError};
use colossus_pipeline::cancel::CancellationToken;
use colossus_pipeline::progress::ProgressReporter;

use crate::repositories::pipeline_repository::{extraction, steps};

/// Maximum retry attempts per LLM call on rate-limit (429) errors.
pub(crate) const MAX_RETRIES_PER_CHUNK: u32 = 3;

/// Call the LLM provider with rate-limit-aware retry.
///
/// On `PipelineError::RateLimited`, sleeps exactly `retry_after_secs` and
/// retries. Max [`MAX_RETRIES_PER_CHUNK`] attempts. Any other error returns
/// immediately. Emits progress events during waits so the UI shows status.
///
/// The `chunk_idx` / `chunk_total` pair is used only for logging and
/// progress payloads. For full-document calls, pass `(0, 1)`.
pub(crate) async fn call_with_rate_limit_retry(
    provider: &dyn LlmProvider,
    prompt: &str,
    max_tokens: u32,
    cancel: &CancellationToken,
    progress: &ProgressReporter,
    chunk_idx: usize,
    chunk_total: usize,
) -> Result<LlmResponse, PipelineError> {
    let mut attempt = 0u32;
    loop {
        match provider.invoke(prompt, max_tokens).await {
            Ok(response) => return Ok(response),
            Err(PipelineError::RateLimited { retry_after_secs }) => {
                attempt += 1;
                if attempt > MAX_RETRIES_PER_CHUNK {
                    return Err(PipelineError::LlmProvider(format!(
                        "chunk {}/{}: exhausted {} rate-limit retries",
                        chunk_idx + 1,
                        chunk_total,
                        MAX_RETRIES_PER_CHUNK
                    )));
                }

                progress
                    .report(serde_json::json!({
                        "status": "rate_limited",
                        "chunk": chunk_idx + 1,
                        "total": chunk_total,
                        "retry_after_secs": retry_after_secs,
                        "attempt": attempt,
                    }))
                    .await
                    .ok();

                tracing::warn!(
                    chunk = chunk_idx,
                    retry_after_secs,
                    attempt,
                    "Rate limited, sleeping before retry"
                );

                // Sleep with cancel awareness. Poll is_cancelled every second.
                let mut remaining = retry_after_secs;
                while remaining > 0 {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    remaining -= 1;
                    if cancel.is_cancelled().await {
                        return Err(PipelineError::LlmProvider(
                            "Cancelled during rate-limit wait".into(),
                        ));
                    }
                }
                // Loop continues — retry the call
            }
            Err(other) => return Err(other),
        }
    }
}

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
        "FAILED",
    )
    .await
    {
        tracing::warn!(run_id, error = %e, reason, "mark_run_failed: DB write failed (non-fatal)");
    }
}

/// Log-and-ignore writer that records a step failure via the repository.
///
/// Same rationale as [`mark_run_failed`]: don't let a secondary write
/// error shadow the primary extraction failure.
pub(crate) async fn mark_step_failed(
    db: &PgPool,
    step_id: i32,
    step_start: std::time::Instant,
    error_message: &str,
) {
    if let Err(e) = steps::record_step_failure(
        db,
        step_id,
        step_start.elapsed().as_secs_f64(),
        error_message,
    )
    .await
    {
        tracing::warn!(step_id, error = %e, "mark_step_failed: DB write failed (non-fatal)");
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_fences_json_block() {
        let input = "```json\n{\"entities\":[]}\n```";
        assert_eq!(strip_markdown_fences(input), "{\"entities\":[]}");
    }

    #[test]
    fn test_strip_fences_plain_block() {
        let input = "```\n{\"entities\":[]}\n```";
        assert_eq!(strip_markdown_fences(input), "{\"entities\":[]}");
    }

    #[test]
    fn test_strip_fences_no_fences() {
        let input = "{\"entities\":[]}";
        assert_eq!(strip_markdown_fences(input), "{\"entities\":[]}");
    }

    #[test]
    fn test_strip_fences_with_whitespace() {
        let input = "  \n```json\n{\"a\":1}\n```\n  ";
        assert_eq!(strip_markdown_fences(input), "{\"a\":1}");
    }

    #[test]
    fn test_parse_valid_json() {
        let input =
            r#"{"entities": [{"id": "0", "entity_type": "Party"}], "relationships": []}"#;
        let result = parse_chunk_response(input);
        assert!(result.is_ok());
        let val = result.unwrap();
        assert!(val["entities"].is_array());
        assert_eq!(val["entities"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_parse_fenced_json() {
        let input = "```json\n{\"entities\": [], \"relationships\": []}\n```";
        let result = parse_chunk_response(input);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_repairable_json() {
        // Trailing comma is invalid JSON but llm_json can repair it.
        let input =
            r#"{"entities": [{"id": "0", "entity_type": "Party"},], "relationships": []}"#;
        let result = parse_chunk_response(input);
        assert!(result.is_ok(), "Expected repair to succeed, got: {result:?}");
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

    #[test]
    fn ensure_object_accepts_object() {
        let v = serde_json::json!({"entities": []});
        assert!(ensure_object(v).is_ok());
    }
}
