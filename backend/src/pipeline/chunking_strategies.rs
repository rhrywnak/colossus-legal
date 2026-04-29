//! Named chunking strategy expansion for legal document types.
//!
//! A "strategy" is a convenience name that expands into a set of
//! `chunking_config` key-value pairs. These pairs configure the
//! `StructureAwareSplitter` in colossus-extract.
//!
//! ## Why this lives in colossus-legal, not colossus-rs
//!
//! Strategy names encode legal document knowledge:
//! - `qa_pair` means "numbered questions with answer markers" — a
//!   pattern specific to discovery responses in civil litigation.
//! - `numbered_paragraph` means "paragraphs beginning with digits" —
//!   the structure of complaints and affidavits.
//! - `section_heading` means "ALL-CAPS headings or specific legal
//!   keywords" — court rulings and orders.
//!
//! colossus-rs stays domain-agnostic. It only sees `boundary_pattern`
//! and `response_marker` as generic regex strings. Adding a new legal
//! document type means editing this module and a sibling profile YAML —
//! no changes to the reusable splitter crate.
//!
//! ## Rust Learning: Strategy as data expansion, not polymorphism
//!
//! We could model strategies as trait objects (each strategy implements
//! a `Strategy` trait with a `configure()` method). But our strategies
//! are just named sets of key-value pairs — no behavior, no state, no
//! lifecycle. A simple `match` arm that returns a `HashMap` is more
//! honest about what's happening: data lookup, not polymorphic dispatch.
//! If strategies ever need runtime behavior (e.g., auto-detecting
//! boundary patterns from document content), that's when a trait earns
//! its keep.

use std::collections::HashMap;

use serde_json::json;

/// Expand a named strategy into `chunking_config` defaults.
///
/// Returns a `HashMap<String, serde_json::Value>` with the strategy's
/// default configuration. The caller merges this UNDERNEATH the
/// profile's explicit `chunking_config` values — explicit values always
/// win over strategy defaults.
///
/// Unknown strategy names return an empty map, which means "use whatever
/// explicit values are in the profile." This is intentional: an
/// unrecognized strategy is not an error — it just means the profile
/// must supply all values explicitly (the "custom" case).
///
/// ## Merge order (lowest to highest priority)
///
/// 1. Strategy defaults (this function's output)
/// 2. Profile YAML `chunking_config` values
/// 3. Per-document overrides from `pipeline_config`
///
/// ## Rust Learning: Why return `HashMap`, not `Result`
///
/// An unknown strategy is not an error condition — it's the "custom"
/// case where the profile provides all config values directly. Returning
/// `Result<HashMap, Error>` would force every caller to handle an error
/// that isn't really an error. An empty `HashMap` naturally falls
/// through to the profile's explicit values via `HashMap::extend()`.
pub fn expand_strategy(strategy: &str) -> HashMap<String, serde_json::Value> {
    match strategy {
        // Discovery responses: numbered questions (1. 2. 3. ...) with
        // answer markers. Each Q&A pair is an atomic unit.
        "qa_pair" => HashMap::from([
            ("boundary_pattern".to_string(), json!(r"^\d+\.\s")),
            ("response_marker".to_string(), json!(r"^Answer:\s")),
            ("units_per_chunk".to_string(), json!(25)),
            ("unit_overlap".to_string(), json!(0)),
        ]),

        // Complaints and affidavits: numbered paragraphs (1. 2. 3. ...).
        // Each paragraph is an atomic unit — typically one allegation or
        // one sworn statement.
        "numbered_paragraph" => HashMap::from([
            ("boundary_pattern".to_string(), json!(r"^\d+\.\s")),
            ("units_per_chunk".to_string(), json!(30)),
            ("unit_overlap".to_string(), json!(0)),
        ]),

        // Court rulings and orders: section headings in ALL CAPS or
        // specific legal keywords (ORDER, OPINION, RULING, FINDINGS).
        // Fewer units per chunk because sections tend to be longer than
        // individual paragraphs.
        "section_heading" => HashMap::from([
            (
                "boundary_pattern".to_string(),
                json!(r"^(?:[A-Z][A-Z\s]{3,}|(?:ORDER|OPINION|RULING|FINDINGS))"),
            ),
            ("units_per_chunk".to_string(), json!(5)),
            ("unit_overlap".to_string(), json!(0)),
        ]),

        // Unknown or "custom" — return empty map. The profile must
        // supply boundary_pattern and other values explicitly.
        _ => HashMap::new(),
    }
}

/// Resolve the effective chunking configuration for extraction.
///
/// Merges three layers (lowest to highest priority):
/// 1. Strategy defaults (from [`expand_strategy`])
/// 2. Profile's `chunking_config` (from the YAML file)
/// 3. Per-document overrides (already merged into `resolved_chunking_config`
///    by `resolve_config()` in `pipeline/config.rs`)
///
/// The input map already contains the merged result of profile +
/// per-document overrides — this function adds the strategy layer
/// underneath.
///
/// ## Rust Learning: `HashMap::extend()` ordering
///
/// `base.extend(overlay)` overwrites base keys with overlay values.
/// So we start with strategy defaults (lowest priority), then extend
/// with the resolved config (highest priority). The result is: strategy
/// provides defaults, explicit config overrides them.
pub fn resolve_chunking_config(
    resolved_chunking_config: &HashMap<String, serde_json::Value>,
) -> HashMap<String, serde_json::Value> {
    // Read the strategy name from the resolved config.
    // Default "full_document" means no strategy expansion needed.
    let strategy = resolved_chunking_config
        .get("strategy")
        .and_then(|v| v.as_str())
        .unwrap_or("full_document");

    // Start with strategy defaults (lowest priority).
    let mut effective = expand_strategy(strategy);

    // Overlay the resolved config (profile + per-doc overrides win).
    effective.extend(
        resolved_chunking_config
            .iter()
            .map(|(k, v)| (k.clone(), v.clone())),
    );

    effective
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── expand_strategy tests ───────────────────────────────────

    #[test]
    fn qa_pair_strategy_has_boundary_and_marker() {
        let config = expand_strategy("qa_pair");
        assert_eq!(
            config.get("boundary_pattern").and_then(|v| v.as_str()),
            Some(r"^\d+\.\s"),
            "qa_pair must have a numbered-item boundary pattern"
        );
        assert_eq!(
            config.get("response_marker").and_then(|v| v.as_str()),
            Some(r"^Answer:\s"),
            "qa_pair must have an answer response marker"
        );
        assert_eq!(
            config.get("units_per_chunk").and_then(|v| v.as_i64()),
            Some(25)
        );
        assert_eq!(
            config.get("unit_overlap").and_then(|v| v.as_i64()),
            Some(0)
        );
    }

    #[test]
    fn numbered_paragraph_strategy_has_boundary_no_marker() {
        let config = expand_strategy("numbered_paragraph");
        assert_eq!(
            config.get("boundary_pattern").and_then(|v| v.as_str()),
            Some(r"^\d+\.\s"),
        );
        // No response_marker for paragraphs — they don't have Q&A structure.
        assert!(
            config.get("response_marker").is_none(),
            "numbered_paragraph should not have a response_marker"
        );
        assert_eq!(
            config.get("units_per_chunk").and_then(|v| v.as_i64()),
            Some(30)
        );
    }

    #[test]
    fn section_heading_strategy_has_caps_pattern() {
        let config = expand_strategy("section_heading");
        let pattern = config
            .get("boundary_pattern")
            .and_then(|v| v.as_str())
            .expect("section_heading must have a boundary_pattern");
        // Verify the pattern matches typical court ruling headings.
        let re = regex::Regex::new(pattern).expect("pattern must be valid regex");
        assert!(re.is_match("ORDER"), "should match ORDER");
        assert!(
            re.is_match("FINDINGS OF FACT"),
            "should match FINDINGS OF FACT"
        );
        assert!(re.is_match("OPINION"), "should match OPINION");
        assert!(
            !re.is_match("The court finds"),
            "should not match lowercase prose"
        );
        assert_eq!(
            config.get("units_per_chunk").and_then(|v| v.as_i64()),
            Some(5),
            "section_heading uses fewer units because sections are longer"
        );
    }

    #[test]
    fn unknown_strategy_returns_empty_map() {
        let config = expand_strategy("nonexistent_strategy");
        assert!(
            config.is_empty(),
            "unknown strategy should return empty map, not error"
        );
    }

    #[test]
    fn custom_strategy_returns_empty_map() {
        let config = expand_strategy("custom");
        assert!(
            config.is_empty(),
            "'custom' strategy returns empty — profile provides all values"
        );
    }

    // ── resolve_chunking_config tests ───────────────────────────

    #[test]
    fn resolve_applies_strategy_defaults() {
        // Profile says strategy=qa_pair but provides no boundary_pattern.
        let profile_config = HashMap::from([
            ("mode".to_string(), json!("structured")),
            ("strategy".to_string(), json!("qa_pair")),
        ]);

        let effective = resolve_chunking_config(&profile_config);

        // Strategy default fills in boundary_pattern.
        assert_eq!(
            effective.get("boundary_pattern").and_then(|v| v.as_str()),
            Some(r"^\d+\.\s"),
            "strategy defaults should fill in missing keys"
        );
        // Profile values preserved.
        assert_eq!(
            effective.get("mode").and_then(|v| v.as_str()),
            Some("structured")
        );
    }

    #[test]
    fn resolve_profile_overrides_strategy_defaults() {
        // Profile says strategy=qa_pair but overrides units_per_chunk.
        let profile_config = HashMap::from([
            ("strategy".to_string(), json!("qa_pair")),
            ("units_per_chunk".to_string(), json!(15)),
        ]);

        let effective = resolve_chunking_config(&profile_config);

        // Profile's explicit 15 wins over strategy default 25.
        assert_eq!(
            effective.get("units_per_chunk").and_then(|v| v.as_i64()),
            Some(15),
            "explicit profile value must override strategy default"
        );
        // Strategy default still provides boundary_pattern.
        assert_eq!(
            effective.get("boundary_pattern").and_then(|v| v.as_str()),
            Some(r"^\d+\.\s"),
        );
    }

    #[test]
    fn resolve_no_strategy_key_uses_full_document_default() {
        // No strategy key at all — full_document is the default,
        // which expands to empty map.
        let profile_config = HashMap::from([("mode".to_string(), json!("full"))]);

        let effective = resolve_chunking_config(&profile_config);

        // Just the profile values, no strategy injection.
        assert_eq!(
            effective.get("mode").and_then(|v| v.as_str()),
            Some("full")
        );
        assert!(effective.get("boundary_pattern").is_none());
    }

    #[test]
    fn resolve_custom_strategy_uses_explicit_values_only() {
        // strategy=custom with explicit boundary_pattern.
        let profile_config = HashMap::from([
            ("strategy".to_string(), json!("custom")),
            ("boundary_pattern".to_string(), json!(r"^SECTION\s+\d+")),
            ("units_per_chunk".to_string(), json!(10)),
        ]);

        let effective = resolve_chunking_config(&profile_config);

        assert_eq!(
            effective.get("boundary_pattern").and_then(|v| v.as_str()),
            Some(r"^SECTION\s+\d+"),
            "custom strategy uses the profile's explicit pattern"
        );
        assert_eq!(
            effective.get("units_per_chunk").and_then(|v| v.as_i64()),
            Some(10)
        );
    }
}
