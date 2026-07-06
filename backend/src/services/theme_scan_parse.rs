//! Theme Scan verdict parsing (D2b).
//!
//! Turns one raw LLM reply into a validated [`Verdict`], or a per-item error
//! string the scan COUNTS as a failure (never a panic, never a silent drop).
//!
//! ## Why this ports `llm_extract_helpers`' three-tier discipline
//!
//! The extraction pipeline learned the hard way that LLM JSON needs a specific
//! parse discipline (`pipeline/steps/llm_extract_helpers.rs`): strip markdown
//! fences → try `serde_json` → fall back to `llm_json::repair_json` → and,
//! crucially, GATE the result on being a top-level object. The last step exists
//! because `repair_json` is aggressively permissive: handed prose, it coerces it
//! into a valid JSON *string primitive* rather than failing, so without the
//! object-gate a garbage reply parses "successfully" into a useless value and is
//! miscounted as a success.
//!
//! We deliberately PORT that discipline here rather than reach into the pipeline
//! step's `pub(crate)` helpers: a service depending on a pipeline step's
//! internals is the wrong layering direction, and a self-contained parser is
//! independently testable (mirroring that module's own test style). The verdict
//! adds two validations on top of the object-gate: the role must be one of the
//! four `FactRole` tokens, and `confidence` must be in `[0.0, 1.0]`.

use serde::Deserialize;

use crate::domain::fact_role::FactRole;

/// A validated judge verdict for one candidate quote.
///
/// ## Rust Learning: `proposed_role: FactRole` deserializes the validation
///
/// Because [`FactRole`] derives `Deserialize` with `snake_case` tokens, an
/// out-of-set role (`"makes"`, a typo, `null`) fails `serde_json::from_value`
/// right here — the role validation IS the deserialization. No separate "is this
/// a known role?" check can drift from the enum, and the failure is a per-item
/// error the caller counts, not a silent default (Standing Rule 1).
///
// serde: `deny_unknown_fields` is deliberately OMITTED. A judge running over a
// large real corpus should not discard a substantively-correct verdict just
// because the model volunteered an extra key (a stray `"explanation"`,
// `"notes"`, etc.) alongside the four required ones. The four required fields
// are still mandatory — a MISSING or mistyped field is a hard parse failure via
// serde — so leniency here only tolerates *surplus*, never absence, and the raw
// reply is retained in the failure log when parsing does fail (see judge_one).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Verdict {
    /// Whether the quote bears on the accusation at all.
    pub relevant: bool,
    /// The role the quote plays if relevant. Required even when `relevant` is
    /// `false` (the prompt still asks for it; it is simply ignored downstream) —
    /// so a missing/invalid role is always a hard parse failure.
    pub proposed_role: FactRole,
    /// One-to-two-sentence justification, stored as the fact ref's `note`.
    pub reason: String,
    /// Self-reported confidence, validated to `[0.0, 1.0]` by [`parse_verdict`].
    pub confidence: f32,
}

/// Parse and validate one raw LLM reply into a [`Verdict`].
///
/// Returns `Err(reason)` — a human-readable cause — for every failure mode:
/// unparseable/unrepairable JSON, a non-object reply, an out-of-set role, or an
/// out-of-range confidence. The caller treats any `Err` as a counted per-item
/// failure; it never aborts the batch and never panics.
pub fn parse_verdict(text: &str) -> Result<Verdict, String> {
    let value = parse_json_object(text)?;
    let verdict: Verdict = serde_json::from_value(value)
        .map_err(|e| format!("verdict JSON did not match the required shape: {e}"))?;

    // Range-check confidence: serde accepts any JSON number into an f32, so the
    // `[0.0, 1.0]` contract is enforced here, not by deserialization. NaN fails
    // this check too (no comparison with NaN is true), which is correct — a NaN
    // confidence is not a usable judgment.
    if !(0.0..=1.0).contains(&verdict.confidence) {
        return Err(format!(
            "confidence {} is outside the required range [0.0, 1.0]",
            verdict.confidence
        ));
    }
    Ok(verdict)
}

/// Strip markdown fences, parse, repair-on-failure, and gate on a JSON object.
///
/// Ported from `llm_extract_helpers::parse_chunk_response` +
/// `ensure_object`; see the module doc for why the object-gate is load-bearing.
fn parse_json_object(text: &str) -> Result<serde_json::Value, String> {
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
            // Char-boundary-safe preview (byte slicing could panic mid-codepoint
            // on a multibyte reply — we take whole chars instead).
            let preview: String = stripped.chars().take(200).collect();
            Err(format!(
                "JSON parse and repair both failed. Repair error: {repair_err}. Preview: {preview}"
            ))
        }
    }
}

/// Gate a parsed value on being a top-level JSON object.
///
/// `repair_json` coerces prose into a JSON string primitive rather than failing;
/// without this gate a garbage reply becomes a "successful" non-object value and
/// downstream field access silently yields nothing. Ported verbatim in intent
/// from `llm_extract_helpers::ensure_object`.
fn ensure_object(val: serde_json::Value) -> Result<serde_json::Value, String> {
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
fn strip_markdown_fences(text: &str) -> String {
    let t = text.trim();
    let t = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```"))
        .unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t);
    t.trim().to_string()
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_clean_relevant_verdict() {
        let input = r#"{"relevant": true, "proposed_role": "rebuts", "reason": "Directly denies the alleged statement.", "confidence": 0.9}"#;
        let v = parse_verdict(input).expect("clean verdict should parse");
        assert!(v.relevant);
        assert_eq!(v.proposed_role, FactRole::Rebuts);
        assert_eq!(v.confidence, 0.9);
        assert_eq!(v.reason, "Directly denies the alleged statement.");
    }

    #[test]
    fn parses_a_fenced_verdict() {
        // Models often wrap JSON in a ```json fence despite instructions not to.
        let input = "```json\n{\"relevant\": false, \"proposed_role\": \"supports\", \"reason\": \"Unrelated topic.\", \"confidence\": 0.2}\n```";
        let v = parse_verdict(input).expect("fenced verdict should parse");
        assert!(!v.relevant);
        assert_eq!(v.proposed_role, FactRole::Supports);
    }

    #[test]
    fn irrelevant_verdict_still_requires_a_valid_role() {
        // relevant=false is fine, but a valid role is still mandatory — a missing
        // or bad role is a hard failure even for a rejection.
        let ok = r#"{"relevant": false, "proposed_role": "contradicts", "reason": "n/a", "confidence": 0.1}"#;
        assert!(parse_verdict(ok).is_ok());

        let missing_role = r#"{"relevant": false, "reason": "n/a", "confidence": 0.1}"#;
        assert!(parse_verdict(missing_role).is_err());
    }

    #[test]
    fn repairs_a_trailing_comma() {
        // Invalid JSON (trailing comma) but llm_json repairs it — the verdict
        // must still come through.
        let input = r#"{"relevant": true, "proposed_role": "supports", "reason": "ok", "confidence": 0.5,}"#;
        assert!(parse_verdict(input).is_ok());
    }

    #[test]
    fn rejects_garbage_as_a_failure_not_a_panic() {
        let input = "I could not determine relevance for this quote, sorry.";
        let result = parse_verdict(input);
        assert!(
            result.is_err(),
            "prose must be a counted failure, not a parse success"
        );
    }

    #[test]
    fn rejects_repair_coerced_bare_string_via_object_gate() {
        // The load-bearing case: repair_json coerces this prose into a JSON
        // string primitive. The object-gate must reject it rather than let it
        // through as a non-object "success".
        let input = "just some words with no json at all";
        let err = parse_verdict(input).expect_err("bare string must be rejected");
        assert!(
            err.contains("not an object") || err.contains("repair both failed"),
            "expected an object-gate or parse failure, got: {err}"
        );
    }

    #[test]
    fn rejects_out_of_set_role() {
        // "makes" was floated in design but has no graph edge — it must not parse.
        let input =
            r#"{"relevant": true, "proposed_role": "makes", "reason": "x", "confidence": 0.8}"#;
        assert!(
            parse_verdict(input).is_err(),
            "an out-of-set role is a failure"
        );
    }

    #[test]
    fn rejects_confidence_out_of_range() {
        let high =
            r#"{"relevant": true, "proposed_role": "supports", "reason": "x", "confidence": 1.5}"#;
        assert!(parse_verdict(high).is_err());

        let negative =
            r#"{"relevant": true, "proposed_role": "supports", "reason": "x", "confidence": -0.1}"#;
        assert!(parse_verdict(negative).is_err());
    }
}
