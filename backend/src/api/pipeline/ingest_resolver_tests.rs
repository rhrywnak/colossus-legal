//! Unit tests for [`crate::api::pipeline::ingest_resolver`].
//!
//! Split into a sibling file (via `#[path]`) so the resolver module stays small —
//! the house pattern (`theme_scan_persist_tests.rs`, `scan_runs_tests.rs`).
//!
//! These pin the two policies that stand between the extraction output and the
//! graph's identity model:
//!
//! 1. the **party_type adapter**, without which person matching cannot run at all;
//! 2. the **auto-merge policy** — exact and normalized bind, fuzzy never does.
//!
//! Both are pure-function testable, so the whole identity contract is exercised
//! with no Neo4j and no pipeline.

use super::*;

// ── Fixtures ─────────────────────────────────────────────────────────────────

fn props(party_type: &str) -> serde_json::Value {
    serde_json::json!({
        "party_name": "Karen A. Tighe",
        "role": "judge",
        "party_type": party_type,
    })
}

// ── The party_type adapter (R1) ──────────────────────────────────────────────

/// The bug that made every person a duplicate.
///
/// Upstream `compatible_type` matches Person nodes on `"individual"` and returns
/// FALSE for anything else; every template emits `"person"`. Without this
/// translation the candidate list for a human party is empty, matching cannot
/// run, and each name variant becomes its own node.
#[test]
fn person_party_type_is_normalized_for_the_resolver() {
    let out = normalize_party_type(props("person"));
    assert_eq!(
        out["party_type"], "individual",
        "the template's 'person' must reach the resolver as 'individual', or \
         person matching silently never runs"
    );
    // Everything else is carried through untouched.
    assert_eq!(out["party_name"], "Karen A. Tighe");
    assert_eq!(out["role"], "judge");
}

#[test]
fn organization_party_type_is_left_alone() {
    // "organization" already matches on both sides — translating it would be a
    // gratuitous change to a working path.
    let out = normalize_party_type(props("organization"));
    assert_eq!(out["party_type"], "organization");
}

#[test]
fn already_normalized_input_is_idempotent() {
    // The adapter may run on rows that already carry the resolver's vocabulary
    // (e.g. a re-ingest). Applying it twice must not corrupt anything.
    let once = normalize_party_type(props("person"));
    let twice = normalize_party_type(once.clone());
    assert_eq!(once, twice);
}

#[test]
fn unknown_party_type_is_not_coerced() {
    // An unrecognized value must keep failing the upstream type check LOUDLY
    // rather than being silently coerced into matching Person — a wrong match is
    // worse than no match (Standing Rule 1).
    let out = normalize_party_type(props("corporation"));
    assert_eq!(out["party_type"], "corporation");
}

#[test]
fn missing_or_non_object_properties_pass_through_unchanged() {
    // Absent party_type: the resolver's own unwrap_or default applies, which is
    // the pre-existing behavior — the adapter must not invent a value.
    let no_type = serde_json::json!({ "party_name": "X" });
    assert_eq!(normalize_party_type(no_type.clone()), no_type);

    // A non-object properties blob must not panic or be rewritten.
    let not_an_object = serde_json::json!("unexpected");
    assert_eq!(
        normalize_party_type(not_an_object.clone()),
        not_an_object,
        "a malformed properties value is passed through, never coerced"
    );
}

// ── The auto-merge policy (rulings #2 / #3) ──────────────────────────────────

/// Only exact and normalized matches may bind two parties together.
///
/// Domain note: in a legal graph a false merge attributes one person's sworn
/// statements to another and is very hard to detect afterwards; a duplicate is
/// visible and fixable. So the policy is deliberately conservative, and this test
/// is the thing that keeps it that way — a future edit that lets fuzzy bind again
/// fails here and reads the reason.
#[test]
fn only_exact_and_normalized_are_auto_mergeable() {
    // Calls the PRODUCTION policy — a test that re-implemented the match would
    // pass even if production drifted.
    let mergeable = is_auto_mergeable;

    assert!(mergeable(&ResolutionMethod::ExactMatch));
    assert!(mergeable(&ResolutionMethod::NormalizedMatch));

    assert!(
        !mergeable(&ResolutionMethod::FuzzyMatch),
        "a Jaro-Winkler similarity score must never decide that two people are \
         the same person"
    );
    assert!(
        !mergeable(&ResolutionMethod::SemanticMatch),
        "semantic similarity likewise never auto-merges"
    );
    assert!(!mergeable(&ResolutionMethod::NewEntity));
}

/// The summary must report what HAPPENED, not what the matcher proposed.
///
/// A demoted fuzzy hit creates a new entity; reporting it as "fuzzy_match" would
/// tell an operator two parties were merged when they were not.
#[test]
fn demoted_matches_are_reported_as_not_merged() {
    let label = resolution_label;

    assert_eq!(
        label(&ResolutionMethod::FuzzyMatch),
        "fuzzy_match_not_merged"
    );
    assert_eq!(
        label(&ResolutionMethod::SemanticMatch),
        "semantic_match_not_merged"
    );
    // The binding methods keep their plain names.
    assert_eq!(label(&ResolutionMethod::ExactMatch), "exact_match");
    assert_eq!(
        label(&ResolutionMethod::NormalizedMatch),
        "normalized_match"
    );
    // And a genuine non-match is still reported as one — this arm exists so the
    // extracted function is pinned across ALL five variants, not just the four
    // the demotion policy touches.
    assert_eq!(label(&ResolutionMethod::NewEntity), "new_entity");
}

// ── The writer/resolver filter symmetry (R4) ─────────────────────────────────

/// The resolver must consider every entity type the WRITER will process.
///
/// `create_party_nodes` accepts all of PARTY_SUBTYPES. This filter previously
/// accepted only the raw "Party", so on a re-ingest of an already-ingested run —
/// where rows carry the resolved "Person"/"Organization" label — the resolver
/// skipped them, returned an empty map, and the writer fell through to a
/// slug-derived id, creating duplicates. The two filters must agree.
#[test]
fn resolver_filter_matches_the_writer_filter() {
    for resolved_form in ["Party", "Person", "Organization"] {
        assert!(
            PARTY_SUBTYPES.contains(&resolved_form),
            "{resolved_form} must be resolvable — the writer processes it"
        );
    }
    // And the filter must not be so wide it drags in non-party entities.
    assert!(!PARTY_SUBTYPES.contains(&"Evidence"));
    assert!(!PARTY_SUBTYPES.contains(&"Allegation"));
}
