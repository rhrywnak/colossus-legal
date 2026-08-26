//! The observability and configuration half of the pre-ingest edge bar.
//!
//! [`super::edge_bar`] decides; this module resolves how hard it bites and
//! reports what it did. They are separate files because the decision half is
//! pure — no environment, no logging, no clock — and keeping it that way is what
//! lets every rule be asserted directly.

use super::edge_bar::{
    filter_pass2_payload, EdgeBarOutcome, FilteredPayload, PatternMode, PatternTriple,
    RejectReason, SupersedeRule,
};

/// Env var selecting the allowlist mode. Absent → [`PatternMode::ReportOnly`].
pub const EDGE_BAR_MODE_ENV: &str = "EDGE_BAR_PATTERN_MODE";

/// EMERGENCY OVERRIDE for the supersession rule, as `weaker:stronger`.
///
/// Absent → [`DEFAULT_SUPERSEDE`], which is ruling Bar B and the normal state.
/// Set only to disable or re-point the rule without a rebuild if it misfires;
/// an unparseable value leaves rule 2 inert and warns loudly.
pub const EDGE_BAR_SUPERSEDE_ENV: &str = "EDGE_BAR_SUPERSEDE";

/// Ruling Bar B, encoded: ABOUT yields to CHARACTERIZES on the same pair.
///
/// See the `// STRUCTURAL:` note below for why this is a compiled constant and
/// not a configurable default.
// STRUCTURAL: this is not configuration — it encodes standing ruling Bar B
// (Roman, 2026-08-25). A deployment where ABOUT does not yield to CHARACTERIZES
// is a deployment violating the ruling, not a deployment with a different
// setting, so there is no environment in which a different value would be
// correct. `EDGE_BAR_SUPERSEDE` exists as an EMERGENCY OVERRIDE — a way to
// disable or re-point the rule without a rebuild if it misfires in production —
// and an override is not the same thing as a configurable default.
//
// It lives in this module rather than in `edge_bar.rs` because the vocabulary is
// Awad v. CFS's, not the filter's: `edge_bar.rs` stays free of case ontology so
// another Colossus project can use it unchanged.
pub const DEFAULT_SUPERSEDE: &str = "ABOUT:CHARACTERIZES";

/// Parse the allowlist mode from an env value.
///
/// ## Why unrecognised input is `ReportOnly` and not an error
///
/// The two failure directions are not symmetrical. Defaulting a typo to
/// `Enforce` would silently start discarding edges nobody chose to discard;
/// defaulting it to `ReportOnly` stores everything and counts. A wrong value is
/// still surfaced — [`resolve_pattern_mode`] warns — so this is a loud fallback,
/// not a silent one.
pub fn parse_pattern_mode(raw: &str) -> Option<PatternMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "enforce" => Some(PatternMode::Enforce),
        "report_only" | "report-only" | "reportonly" => Some(PatternMode::ReportOnly),
        _ => None,
    }
}

/// Parse `weaker:stronger` into a supersession rule.
///
/// Both halves must be non-empty and different — `"ABOUT:ABOUT"` would make
/// every edge of that type supersede itself, which is a configuration mistake
/// rather than a rule.
pub fn parse_supersede(raw: &str) -> Option<SupersedeRule> {
    let (weaker, stronger) = raw.split_once(':')?;
    let (weaker, stronger) = (weaker.trim(), stronger.trim());
    if weaker.is_empty() || stronger.is_empty() || weaker == stronger {
        return None;
    }
    Some((weaker.to_string(), stronger.to_string()))
}

/// Resolve the allowlist mode from the environment, warning on bad input.
///
/// ## Why `ReportOnly` is the shipped default (measured 2026-08-25)
///
/// Seven of the eleven schema files declare TWO `valid_patterns` while declaring
/// SIX `relationship_types` — those lists were authored for
/// `ExtractionSchema::validate`'s boot-time self-check, not as a statement of
/// which edges are legal. Enforcing against them as they stand would have
/// discarded 155 of the Penzien brief's 457 edges, including all 111 that reach
/// an Allegation. Setting `EDGE_BAR_PATTERN_MODE=enforce` is what the
/// schema-completion job turns on — a config change, not a rebuild.
pub fn resolve_pattern_mode() -> PatternMode {
    match std::env::var(EDGE_BAR_MODE_ENV) {
        Ok(raw) => parse_pattern_mode(&raw).unwrap_or_else(|| {
            tracing::warn!(
                var = EDGE_BAR_MODE_ENV,
                value = %raw,
                "Edge bar: unrecognised pattern mode; falling back to report_only \
                 (expected 'enforce' or 'report_only')"
            );
            PatternMode::ReportOnly
        }),
        Err(_) => PatternMode::ReportOnly,
    }
}

/// Resolve the supersession rule from the environment, warning on bad input.
pub fn resolve_supersede() -> Option<SupersedeRule> {
    let raw =
        std::env::var(EDGE_BAR_SUPERSEDE_ENV).unwrap_or_else(|_| DEFAULT_SUPERSEDE.to_string());
    let parsed = parse_supersede(&raw);
    if parsed.is_none() {
        tracing::warn!(
            var = EDGE_BAR_SUPERSEDE_ENV,
            value = %raw,
            "Edge bar: unparseable supersession rule; rule 2 is DISABLED for this run \
             (expected 'WEAKER:STRONGER', e.g. 'ABOUT:CHARACTERIZES')"
        );
    }
    parsed
}

/// One edge's own words, for a log line that stands alone.
///
/// `("?", "?", "?")` when the index is not in the payload. That cannot happen
/// today — the indices come from the same array — but a wrong-looking log line
/// is better than a panic inside a logging call.
pub type EdgeWords<'a> = (&'a str, &'a str, &'a str);

/// Report what the bar did, at a level matched to the consequence.
///
/// Standing Rule 1: a removed edge is never silent, and an edge STORED while
/// failing the allowlist is not silent either — under the shipped `ReportOnly`
/// that is the list an operator acts on, so it gets the same per-edge detail a
/// rejection gets.
pub fn log_edge_bar(
    document_id: &str,
    run_id: i32,
    outcome: &EdgeBarOutcome,
    rejections: &[(usize, RejectReason)],
    pattern_warnings: &[(usize, String, String)],
    parsed: &serde_json::Value,
    words_at: impl Fn(&serde_json::Value, usize) -> Option<EdgeWords<'_>>,
) {
    for (index, reason) in rejections {
        let (from, to, rel_type) = words_at(parsed, *index).unwrap_or(("?", "?", "?"));
        // The level comes from the reason's own classification, so debug-vs-warn
        // is a tested decision rather than a `match` arm only a subscriber sees.
        if !reason.is_operator_visible() {
            tracing::debug!(
                document_id,
                run_id,
                from,
                to,
                rel_type,
                "Edge bar: dropped an exact duplicate edge (no-op, first occurrence kept)"
            );
            continue;
        }
        match reason {
            RejectReason::ExactDuplicate => unreachable!("handled by is_operator_visible above"),
            RejectReason::SupersededBy { stronger } => tracing::warn!(
                document_id, run_id, from, to, rel_type, stronger = %stronger,
                "Edge bar: dropped this edge — a stronger edge already holds the same pair"
            ),
            RejectReason::PatternNotAllowed { from_type, to_type } => tracing::warn!(
                document_id, run_id, from, to, rel_type,
                from_type = %from_type, to_type = %to_type,
                "Edge bar: REJECTED — to permit this edge, add ({from_type}, {rel_type}, \
                 {to_type}) to valid_patterns in this document type's schema file"
            ),
        }
    }

    for (index, from_type, to_type) in pattern_warnings {
        let (from, to, rel_type) = words_at(parsed, *index).unwrap_or(("?", "?", "?"));
        tracing::warn!(
            document_id, run_id, from, to, rel_type,
            from_type = %from_type, to_type = %to_type,
            "Edge bar: STORED but outside valid_patterns (report_only) — add ({from_type}, \
             {rel_type}, {to_type}) to this document type's schema, or set \
             EDGE_BAR_PATTERN_MODE=enforce to reject it"
        );
    }

    let c = outcome.counts;
    if outcome.is_clean() {
        tracing::info!(
            document_id,
            run_id,
            accepted = c.accepted,
            "Edge bar: clean — no duplicates, no supersessions, no pattern misses"
        );
    } else {
        tracing::info!(
            document_id,
            run_id,
            accepted = c.accepted,
            exact_duplicates = c.exact_duplicates,
            deduped = c.deduped,
            rejected_by_pattern = c.rejected_by_pattern,
            pattern_warnings = c.pattern_warnings,
            "Edge bar: filtered pass-2 relationship output"
        );
    }
}

/// Build the bar's inputs, run it, and report — the whole wiring, in one call.
///
/// ## Why the caller passes iterators rather than its own types
///
/// The pass-2 step holds `Pass1Entity` and `CrossDocEntity`; naming those here
/// would tie this module to the repository layer for no gain. Taking
/// `(key, entity_type)` pairs and pattern triples keeps the coupling at the
/// shape of the data, and keeps this function usable from the workflow engine
/// or a future re-run tool without either of them importing the other's structs.
pub fn apply_and_report(
    document_id: &str,
    run_id: i32,
    parsed: &serde_json::Value,
    resolve: impl Fn(&serde_json::Value) -> (String, String, String),
    words_at: impl Fn(&serde_json::Value, usize) -> Option<EdgeWords<'_>>,
    entity_types: impl Iterator<Item = (String, String)>,
    patterns: impl Iterator<Item = PatternTriple>,
) -> FilteredPayload {
    let entity_type_of: std::collections::HashMap<String, String> = entity_types.collect();
    let valid_patterns: Vec<PatternTriple> = patterns.collect();
    let barred = filter_pass2_payload(
        parsed,
        resolve,
        &entity_type_of,
        &valid_patterns,
        resolve_supersede().as_ref(),
        resolve_pattern_mode(),
    );
    log_edge_bar(
        document_id,
        run_id,
        &barred.outcome,
        &barred.rejections,
        &barred.pattern_warnings,
        parsed,
        words_at,
    );
    barred
}

#[cfg(test)]
#[path = "edge_bar_report_tests.rs"]
mod tests;
