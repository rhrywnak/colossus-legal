//! Behavioural tests for the pre-ingest edge bar.
//!
//! Every test asserts a specific input produces a specific verdict and a
//! specific count — a clean compile proves nothing here (CLAUDE.md rule 6). The
//! fixtures are shaped like the real Penzien pass-2 output: `Evidence` sources,
//! `Party`/`Allegation` targets, and the four relationship types that document
//! actually produced.

use super::*;

/// This case's supersession rule, supplied by the caller exactly as the pass-2
/// step supplies it. The vocabulary is the CASE's, not the module's.
fn bar_b() -> SupersedeRule {
    ("ABOUT".to_string(), "CHARACTERIZES".to_string())
}

fn e(from: &str, rel: &str, to: &str) -> EdgeCandidate {
    EdgeCandidate {
        from_key: from.to_string(),
        rel_type: rel.to_string(),
        to_key: to.to_string(),
    }
}

/// The entity types the Penzien brief's pass 2 worked with.
fn types() -> HashMap<String, String> {
    [
        ("evidence-014", "Evidence"),
        ("evidence-015", "Evidence"),
        ("evidence-031", "Evidence"),
        ("party-001", "Party"),
        ("party-004", "Party"),
        ("ctx:allegation-038", "Allegation"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

/// The appellate schema's allowlist AS AUTHORED — two entries, which is the
/// measured state that forced `PatternMode::ReportOnly` to be the default.
fn patterns_as_authored() -> Vec<PatternTriple> {
    vec![
        ("Evidence".into(), "ABOUT".into(), "Party".into()),
        ("Evidence".into(), "STATED_BY".into(), "Party".into()),
    ]
}

/// The same allowlist COMPLETED, as it would be once the schema-authoring job
/// lands — the state under which `Enforce` becomes safe.
fn patterns_completed() -> Vec<PatternTriple> {
    let mut p = patterns_as_authored();
    p.push(("Evidence".into(), "ABOUT".into(), "Allegation".into()));
    p.push(("Evidence".into(), "CHARACTERIZES".into(), "Party".into()));
    p.push((
        "Evidence".into(),
        "CHARACTERIZES".into(),
        "Allegation".into(),
    ));
    p
}

// ── Rule 1 — exact duplicates ────────────────────────────────────────────────

#[test]
fn identical_edge_twice_keeps_the_first_and_drops_the_second() {
    let edges = vec![
        e("evidence-014", "ABOUT", "party-001"),
        e("evidence-014", "ABOUT", "party-001"),
    ];
    let out = apply_edge_bar(
        &edges,
        &types(),
        &patterns_completed(),
        Some(&bar_b()),
        PatternMode::Enforce,
    );
    assert_eq!(out.verdicts[0], EdgeVerdict::Accept);
    assert_eq!(
        out.verdicts[1],
        EdgeVerdict::Reject(RejectReason::ExactDuplicate)
    );
    assert_eq!(out.counts.exact_duplicates, 1);
    assert_eq!(out.counts.accepted, 1);
}

#[test]
fn same_pair_with_different_types_is_not_a_duplicate() {
    // ABOUT and REBUTS between the same two nodes are two different claims.
    // Only rule 2 may collapse a pair, and only for ABOUT-vs-CHARACTERIZES.
    let edges = vec![
        e("evidence-015", "ABOUT", "ctx:allegation-038"),
        e("evidence-015", "REBUTS", "ctx:allegation-038"),
    ];
    let out = apply_edge_bar(
        &edges,
        &types(),
        &[],
        Some(&bar_b()),
        PatternMode::ReportOnly,
    );
    assert_eq!(out.counts.exact_duplicates, 0);
    assert_eq!(out.counts.accepted, 2);
}

// ── Rule 2 — Bar B ───────────────────────────────────────────────────────────

#[test]
fn characterizes_supersedes_about_on_the_same_pair() {
    let edges = vec![
        e("evidence-031", "ABOUT", "party-004"),
        e("evidence-031", "CHARACTERIZES", "party-004"),
    ];
    let out = apply_edge_bar(
        &edges,
        &types(),
        &patterns_completed(),
        Some(&bar_b()),
        PatternMode::Enforce,
    );
    assert_eq!(
        out.verdicts[0],
        EdgeVerdict::Reject(RejectReason::SupersededBy {
            stronger: "CHARACTERIZES".into(),
        })
    );
    assert_eq!(out.verdicts[1], EdgeVerdict::Accept);
    assert_eq!(out.counts.deduped, 1);
}

#[test]
fn bar_b_is_order_independent() {
    // The CHARACTERIZES arriving FIRST must produce the same outcome. This is
    // the property the two-pass implementation exists for: a single pass would
    // keep the ABOUT here and drop it in the previous test.
    let edges = vec![
        e("evidence-031", "CHARACTERIZES", "party-004"),
        e("evidence-031", "ABOUT", "party-004"),
    ];
    let out = apply_edge_bar(
        &edges,
        &types(),
        &patterns_completed(),
        Some(&bar_b()),
        PatternMode::Enforce,
    );
    assert_eq!(out.verdicts[0], EdgeVerdict::Accept);
    assert_eq!(
        out.verdicts[1],
        EdgeVerdict::Reject(RejectReason::SupersededBy {
            stronger: "CHARACTERIZES".into(),
        })
    );
    assert_eq!(out.counts.deduped, 1);
}

#[test]
fn about_survives_when_the_characterizes_is_to_a_different_target() {
    // The census's real shape: one assertion characterizing a Party while being
    // ABOUT an Allegation. Different targets, so both edges are kept.
    let edges = vec![
        e("evidence-031", "ABOUT", "ctx:allegation-038"),
        e("evidence-031", "CHARACTERIZES", "party-004"),
    ];
    let out = apply_edge_bar(
        &edges,
        &types(),
        &patterns_completed(),
        Some(&bar_b()),
        PatternMode::Enforce,
    );
    assert_eq!(out.counts.deduped, 0);
    assert_eq!(out.counts.accepted, 2);
}

#[test]
fn characterizes_never_loses_to_about() {
    // Bar B is one-directional by ruling: the stronger edge wins the pair.
    let edges = vec![
        e("evidence-031", "CHARACTERIZES", "party-004"),
        e("evidence-031", "CHARACTERIZES", "party-004"),
    ];
    let out = apply_edge_bar(
        &edges,
        &types(),
        &patterns_completed(),
        Some(&bar_b()),
        PatternMode::Enforce,
    );
    // The second is an exact duplicate, not a Bar-B casualty — the reason must
    // be the accurate one or the operator log misattributes the removal.
    assert_eq!(
        out.verdicts[1],
        EdgeVerdict::Reject(RejectReason::ExactDuplicate)
    );
    assert_eq!(out.counts.deduped, 0);
}

// ── Rule 3 — the allowlist, both modes ───────────────────────────────────────

#[test]
fn enforce_rejects_an_edge_outside_the_allowlist() {
    let edges = vec![e("evidence-014", "ABOUT", "ctx:allegation-038")];
    let out = apply_edge_bar(
        &edges,
        &types(),
        &patterns_as_authored(),
        Some(&bar_b()),
        PatternMode::Enforce,
    );
    assert_eq!(
        out.verdicts[0],
        EdgeVerdict::Reject(RejectReason::PatternNotAllowed {
            from_type: "Evidence".into(),
            to_type: "Allegation".into(),
        })
    );
    assert_eq!(out.counts.rejected_by_pattern, 1);
    assert_eq!(out.counts.accepted, 0);
}

#[test]
fn enforce_accepts_the_same_edge_once_the_allowlist_declares_it() {
    // The mutation's other direction: the edge is unchanged, the allowlist is
    // not. This is what the schema-completion job buys.
    let edges = vec![e("evidence-014", "ABOUT", "ctx:allegation-038")];
    let out = apply_edge_bar(
        &edges,
        &types(),
        &patterns_completed(),
        Some(&bar_b()),
        PatternMode::Enforce,
    );
    assert_eq!(out.verdicts[0], EdgeVerdict::Accept);
    assert_eq!(out.counts.rejected_by_pattern, 0);
    assert_eq!(out.counts.accepted, 1);
}

#[test]
fn report_only_stores_the_same_edge_and_counts_the_miss() {
    // The shipped default. Nothing is lost; the miss is visible.
    let edges = vec![e("evidence-014", "ABOUT", "ctx:allegation-038")];
    let out = apply_edge_bar(
        &edges,
        &types(),
        &patterns_as_authored(),
        Some(&bar_b()),
        PatternMode::ReportOnly,
    );
    assert_eq!(
        out.verdicts[0],
        EdgeVerdict::AcceptWithPatternWarning {
            from_type: "Evidence".into(),
            to_type: "Allegation".into(),
        }
    );
    assert_eq!(out.counts.accepted, 1);
    assert_eq!(out.counts.pattern_warnings, 1);
    assert_eq!(out.counts.rejected_by_pattern, 0);
}

#[test]
fn an_unknown_endpoint_type_reports_a_question_mark_not_an_empty_string() {
    // Standing Rule 1: "type Party, no such pattern" and "type unknown" are
    // different operator problems and must not read alike in the log.
    let edges = vec![e("evidence-014", "ABOUT", "ctx:evidence-999")];
    let out = apply_edge_bar(
        &edges,
        &types(),
        &patterns_completed(),
        Some(&bar_b()),
        PatternMode::Enforce,
    );
    assert_eq!(
        out.verdicts[0],
        EdgeVerdict::Reject(RejectReason::PatternNotAllowed {
            from_type: "Evidence".into(),
            to_type: "?".into(),
        })
    );
}

// ── The whole bar over one realistic document ────────────────────────────────

#[test]
fn the_penzien_shape_loses_exactly_its_duplicate_pairs_and_nothing_else() {
    // Two assertions in the shape the census measured: each ABOUT a Party it
    // also characterizes (the 66/66 duplication), plus a legitimate
    // ABOUT → Allegation that must survive.
    let edges = vec![
        e("evidence-031", "ABOUT", "party-004"),
        e("evidence-031", "CHARACTERIZES", "party-004"),
        e("evidence-031", "ABOUT", "ctx:allegation-038"),
        e("evidence-015", "ABOUT", "party-001"),
        e("evidence-015", "CHARACTERIZES", "party-001"),
    ];
    let out = apply_edge_bar(
        &edges,
        &types(),
        &patterns_completed(),
        Some(&bar_b()),
        PatternMode::Enforce,
    );
    assert_eq!(out.counts.deduped, 2, "both duplicated pairs collapse");
    assert_eq!(out.counts.accepted, 3);
    assert_eq!(out.counts.exact_duplicates, 0);
    assert_eq!(out.counts.rejected_by_pattern, 0);
    assert!(!out.is_clean(), "a run that removed edges is not clean");
}

#[test]
fn a_document_with_nothing_to_remove_reports_clean() {
    let edges = vec![
        e("evidence-014", "ABOUT", "party-001"),
        e("evidence-015", "ABOUT", "ctx:allegation-038"),
    ];
    let out = apply_edge_bar(
        &edges,
        &types(),
        &patterns_completed(),
        Some(&bar_b()),
        PatternMode::Enforce,
    );
    assert!(out.is_clean());
    assert_eq!(out.counts.accepted, 2);
}

#[test]
fn an_empty_output_is_clean_and_not_an_error() {
    // Distinguishable from "we did not look": counts are all zero and the
    // verdict list is empty, which the caller logs as such.
    let out = apply_edge_bar(
        &[],
        &types(),
        &patterns_completed(),
        Some(&bar_b()),
        PatternMode::Enforce,
    );
    assert!(out.verdicts.is_empty());
    assert!(out.is_clean());
    assert_eq!(out.counts.accepted, 0);
}

// ── The payload filter — what storage actually receives ──────────────────────

fn resolve_json(r: &serde_json::Value) -> (String, String, String) {
    (
        r["from_entity"].as_str().unwrap_or("").to_string(),
        r["to_entity"].as_str().unwrap_or("").to_string(),
        r["relationship_type"].as_str().unwrap_or("").to_string(),
    )
}

#[test]
fn the_filtered_payload_is_what_storage_would_have_stored_minus_the_rejects() {
    // The wiring proof: it is not enough that the bar forms a verdict, the
    // payload handed to `store_pass2_relationships` must actually be shorter.
    let parsed = serde_json::json!({
        "relationships": [
            {"relationship_type": "ABOUT", "from_entity": "evidence-031", "to_entity": "party-004"},
            {"relationship_type": "CHARACTERIZES", "from_entity": "evidence-031", "to_entity": "party-004"},
            {"relationship_type": "ABOUT", "from_entity": "evidence-014", "to_entity": "party-001"},
            {"relationship_type": "ABOUT", "from_entity": "evidence-014", "to_entity": "party-001"},
        ]
    });
    let r = filter_pass2_payload(
        &parsed,
        resolve_json,
        &types(),
        &patterns_completed(),
        Some(&bar_b()),
        PatternMode::Enforce,
    );
    let kept = r.payload["relationships"].as_array().expect("array");
    assert_eq!(
        kept.len(),
        2,
        "one Bar-B dedupe and one exact duplicate removed"
    );
    assert_eq!(kept[0]["relationship_type"], "CHARACTERIZES");
    assert_eq!(kept[1]["relationship_type"], "ABOUT");
    assert_eq!(r.outcome.counts.deduped, 1);
    assert_eq!(r.outcome.counts.exact_duplicates, 1);
    // The rejections carry their input index, so the caller can quote the
    // offending edge from the ORIGINAL payload in its log line.
    assert_eq!(r.rejections.len(), 2);
    assert_eq!(r.rejections[0].0, 0);
    assert_eq!(r.rejections[1].0, 3);
}

#[test]
fn disabling_the_bar_would_store_every_edge_the_model_sent() {
    // The mutation's other direction, stated as a test rather than as a comment:
    // with no rule able to fire (no CHARACTERIZES to supersede anything, no
    // duplicates, ReportOnly so the allowlist cannot reject), the payload is
    // unchanged. If this ever differs from the input length, the bar has grown a
    // rule nobody declared.
    let parsed = serde_json::json!({
        "relationships": [
            {"relationship_type": "ABOUT", "from_entity": "evidence-031", "to_entity": "party-004"},
            {"relationship_type": "REBUTS", "from_entity": "evidence-015", "to_entity": "ctx:allegation-038"},
        ]
    });
    let r = filter_pass2_payload(
        &parsed,
        resolve_json,
        &types(),
        &patterns_as_authored(),
        Some(&bar_b()),
        PatternMode::ReportOnly,
    );
    assert_eq!(
        r.payload["relationships"].as_array().expect("array").len(),
        2
    );
    assert!(r.rejections.is_empty());
    assert_eq!(r.outcome.counts.accepted, 2);
    // ReportOnly still SAW the miss — that is the whole difference between this
    // mode and having no bar at all. One of the two edges (Evidence ABOUT Party)
    // IS in the as-authored allowlist and passes cleanly; the REBUTS is not, and
    // is the one counted. Exactly the split the schema-completion job closes.
    assert_eq!(r.outcome.counts.pattern_warnings, 1);
}

#[test]
fn a_payload_without_a_relationships_array_is_echoed_not_emptied() {
    let parsed = serde_json::json!({"entities": []});
    let r = filter_pass2_payload(
        &parsed,
        resolve_json,
        &types(),
        &patterns_completed(),
        Some(&bar_b()),
        PatternMode::Enforce,
    );
    assert_eq!(r.payload, parsed, "the payload is returned unchanged");
    assert!(r.outcome.verdicts.is_empty());
    assert!(r.rejections.is_empty());
    assert_eq!(r.outcome.counts.accepted, 0);
}

#[test]
fn the_bar_is_wired_only_into_pass_2_and_never_touches_pass_1() {
    // Ruling: pass-1's structural CONTAINED_IN layer is plumbing the case-health
    // pane reads, and the bar must never see it. That invariant lives in WHERE
    // the filter is called from, so it is asserted against the source — the same
    // mechanism the scenario page's fences use, and the only one available for
    // "this function is not called from there".
    let pass2 = include_str!("steps/llm_extract_pass2.rs");
    assert!(
        pass2.contains("edge_bar_report::apply_and_report"),
        "the bar must be wired into the pass-2 step"
    );
    let pass1 = include_str!("steps/llm_extract.rs");
    assert!(
        !pass1.contains("edge_bar"),
        "pass 1 must not invoke the edge bar — its CONTAINED_IN layer is exempt by ruling"
    );
    let ingest = include_str!("steps/ingest.rs");
    assert!(
        !ingest.contains("edge_bar"),
        "the bar runs pre-storage, not at graph-write time; a second application would double-count"
    );
}
