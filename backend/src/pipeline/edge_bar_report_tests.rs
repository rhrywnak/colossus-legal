//! Tests for the edge bar's configuration and reporting half.
//!
//! The parsers are asserted directly. `log_edge_bar` is exercised through every
//! branch — the three reject reasons, the pattern-warning loop, and both
//! `is_clean` arms — with a `words_at` closure that also proves the
//! index-not-found fallback is reachable. There is no tracing subscriber in this
//! repo, so what a test can assert is that each branch RUNS and that the level
//! decision (`is_operator_visible`) is the one the loop keys off.

use super::*;
use crate::pipeline::edge_bar::{EdgeBarCounts, EdgeBarOutcome, EdgeVerdict};

// ── The mode parser ──────────────────────────────────────────────────────────

#[test]
fn pattern_mode_accepts_both_spellings_and_ignores_case_and_space() {
    assert_eq!(parse_pattern_mode("enforce"), Some(PatternMode::Enforce));
    assert_eq!(parse_pattern_mode("  ENFORCE "), Some(PatternMode::Enforce));
    assert_eq!(
        parse_pattern_mode("report_only"),
        Some(PatternMode::ReportOnly)
    );
    assert_eq!(
        parse_pattern_mode("report-only"),
        Some(PatternMode::ReportOnly)
    );
    assert_eq!(
        parse_pattern_mode("ReportOnly"),
        Some(PatternMode::ReportOnly)
    );
}

#[test]
fn an_unrecognised_mode_is_none_so_the_caller_can_warn() {
    // `None`, not `Some(ReportOnly)`: the fallback has to be distinguishable
    // from a deliberate choice, or the warn cannot be emitted.
    assert_eq!(parse_pattern_mode("yes"), None);
    assert_eq!(parse_pattern_mode(""), None);
    assert_eq!(parse_pattern_mode("enforce!"), None);
}

// ── The supersession-rule parser ─────────────────────────────────────────────

#[test]
fn supersede_parses_the_case_default() {
    assert_eq!(
        parse_supersede(DEFAULT_SUPERSEDE),
        Some(("ABOUT".to_string(), "CHARACTERIZES".to_string()))
    );
}

#[test]
fn supersede_rejects_the_shapes_that_would_be_a_configuration_mistake() {
    assert_eq!(parse_supersede("ABOUT"), None, "no separator");
    assert_eq!(parse_supersede(":CHARACTERIZES"), None, "empty weaker");
    assert_eq!(parse_supersede("ABOUT:"), None, "empty stronger");
    // Self-supersession would make every edge of that type drop itself.
    assert_eq!(parse_supersede("ABOUT:ABOUT"), None, "same both sides");
}

#[test]
fn supersede_tolerates_spacing_around_the_separator() {
    assert_eq!(
        parse_supersede(" ABOUT : CHARACTERIZES "),
        Some(("ABOUT".to_string(), "CHARACTERIZES".to_string()))
    );
}

// ── The level decision ───────────────────────────────────────────────────────

#[test]
fn only_the_declared_no_op_is_invisible_to_the_operator() {
    assert!(
        !RejectReason::ExactDuplicate.is_operator_visible(),
        "two identical edges were never two facts — debug"
    );
    assert!(RejectReason::SupersededBy {
        stronger: "CHARACTERIZES".into()
    }
    .is_operator_visible());
    assert!(RejectReason::PatternNotAllowed {
        from_type: "Evidence".into(),
        to_type: "Allegation".into(),
    }
    .is_operator_visible());
}

// ── log_edge_bar, every branch ───────────────────────────────────────────────

fn payload() -> serde_json::Value {
    serde_json::json!({
        "relationships": [
            {"relationship_type": "ABOUT", "from_entity": "evidence-031", "to_entity": "party-004"},
            {"relationship_type": "ABOUT", "from_entity": "evidence-014", "to_entity": "party-001"},
            {"relationship_type": "REBUTS", "from_entity": "evidence-015", "to_entity": "ctx:allegation-038"},
        ]
    })
}

/// The same projection the production call site supplies.
fn words_at(parsed: &serde_json::Value, i: usize) -> Option<(&str, &str, &str)> {
    let r = parsed.get("relationships")?.as_array()?.get(i)?;
    Some((
        r["from_entity"].as_str().unwrap_or("?"),
        r["to_entity"].as_str().unwrap_or("?"),
        r["relationship_type"].as_str().unwrap_or("?"),
    ))
}

#[test]
fn logging_walks_every_reject_reason_and_the_pattern_warning_loop() {
    let outcome = EdgeBarOutcome {
        verdicts: vec![
            EdgeVerdict::Reject(RejectReason::ExactDuplicate),
            EdgeVerdict::Reject(RejectReason::SupersededBy {
                stronger: "CHARACTERIZES".into(),
            }),
            EdgeVerdict::AcceptWithPatternWarning {
                from_type: "Evidence".into(),
                to_type: "Allegation".into(),
            },
        ],
        counts: EdgeBarCounts {
            accepted: 1,
            exact_duplicates: 1,
            deduped: 1,
            rejected_by_pattern: 0,
            pattern_warnings: 1,
        },
    };
    let rejections = vec![
        (0, RejectReason::ExactDuplicate),
        (
            1,
            RejectReason::SupersededBy {
                stronger: "CHARACTERIZES".into(),
            },
        ),
        (
            2,
            RejectReason::PatternNotAllowed {
                from_type: "Evidence".into(),
                to_type: "Allegation".into(),
            },
        ),
    ];
    let warnings = vec![(2, "Evidence".to_string(), "Allegation".to_string())];
    // The `unreachable!` arm in the reject loop is only sound because the loop
    // keys off `is_operator_visible` first. If someone reorders that, this call
    // panics and names the file.
    log_edge_bar(
        "doc-x",
        171,
        &outcome,
        &rejections,
        &warnings,
        &payload(),
        words_at,
    );
    assert!(!outcome.is_clean());
}

#[test]
fn logging_survives_an_index_the_payload_does_not_have() {
    // The fallback path. Cannot happen from `filter_pass2_payload`'s own
    // indices, but a logging call must not panic on a caller's mistake.
    let outcome = EdgeBarOutcome {
        verdicts: vec![EdgeVerdict::Reject(RejectReason::ExactDuplicate)],
        counts: EdgeBarCounts {
            exact_duplicates: 1,
            ..Default::default()
        },
    };
    log_edge_bar(
        "doc-x",
        171,
        &outcome,
        &[(99, RejectReason::ExactDuplicate)],
        &[(99, "?".to_string(), "?".to_string())],
        &payload(),
        words_at,
    );
}

#[test]
fn a_clean_outcome_takes_the_clean_branch() {
    let outcome = EdgeBarOutcome {
        verdicts: vec![EdgeVerdict::Accept],
        counts: EdgeBarCounts {
            accepted: 1,
            ..Default::default()
        },
    };
    assert!(outcome.is_clean());
    log_edge_bar("doc-x", 171, &outcome, &[], &[], &payload(), words_at);
}

// ── The whole wiring, in one call ────────────────────────────────────────────

fn resolve_json(r: &serde_json::Value) -> (String, String, String) {
    (
        r["from_entity"].as_str().unwrap_or("").to_string(),
        r["to_entity"].as_str().unwrap_or("").to_string(),
        r["relationship_type"].as_str().unwrap_or("").to_string(),
    )
}

#[test]
fn apply_and_report_collects_its_inputs_and_returns_the_bar_s_own_verdicts() {
    // Deliberately does NOT set either env var: this asserts the DEFAULT path,
    // which is what a real run takes, and setting process-wide env in a test
    // would race every other test in this binary.
    //
    // The payload carries one of each interesting shape: a duplicate pair
    // (Bar B, using the default ABOUT:CHARACTERIZES rule), an exact duplicate,
    // and one edge that is simply fine.
    let parsed = serde_json::json!({
        "relationships": [
            {"relationship_type": "ABOUT", "from_entity": "e-1", "to_entity": "p-1"},
            {"relationship_type": "CHARACTERIZES", "from_entity": "e-1", "to_entity": "p-1"},
            {"relationship_type": "ABOUT", "from_entity": "e-2", "to_entity": "p-1"},
            {"relationship_type": "ABOUT", "from_entity": "e-2", "to_entity": "p-1"},
        ]
    });
    let entity_types = [
        ("e-1".to_string(), "Evidence".to_string()),
        ("e-2".to_string(), "Evidence".to_string()),
        ("p-1".to_string(), "Party".to_string()),
    ]
    .into_iter();
    let patterns = [(
        "Evidence".to_string(),
        "ABOUT".to_string(),
        "Party".to_string(),
    )]
    .into_iter();

    let out = apply_and_report(
        "doc-x",
        171,
        &parsed,
        resolve_json,
        words_at,
        entity_types,
        patterns,
    );

    // Bar B fired on (e-1 → p-1) using the DEFAULT rule — proof the env default
    // is wired, not merely defined.
    assert_eq!(out.outcome.counts.deduped, 1);
    // The repeated (e-2 → p-1) ABOUT collapsed.
    assert_eq!(out.outcome.counts.exact_duplicates, 1);
    // Two survive: the CHARACTERIZES and one of the e-2 ABOUTs.
    assert_eq!(
        out.payload["relationships"]
            .as_array()
            .expect("array")
            .len(),
        2
    );
    assert_eq!(out.outcome.counts.accepted, 2);
    // CHARACTERIZES is not in the supplied allowlist, and the default mode is
    // ReportOnly — so it was STORED and flagged, not rejected. That is the
    // shipped behaviour, asserted here rather than assumed.
    assert_eq!(out.outcome.counts.rejected_by_pattern, 0);
    assert_eq!(out.outcome.counts.pattern_warnings, 1);
    assert_eq!(out.pattern_warnings.len(), 1);
    assert_eq!(out.rejections.len(), 2);
}
