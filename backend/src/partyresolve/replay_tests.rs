// Tests for `partyresolve::replay`.
//
// The counters in this module are what the whole task is judged on, so they are
// tested against hand-built cases where the right answer is obvious. A miscount
// here would report a regression as a clean run.

use super::*;

const PERSON: &str = "Person";

fn replay(stored: &str, today: &str, with_fix: &str, via: &'static str) -> Replay {
    Replay {
        document: "doc-test".to_string(),
        surface: "Someone".to_string(),
        stored: stored.to_string(),
        today: today.to_string(),
        with_fix: with_fix.to_string(),
        via,
    }
}

fn existing() -> Vec<String> {
    vec!["person-a".to_string(), "person-b".to_string()]
}

/// The two counters that must read zero. A mention losing its node, and a
/// mention changing which node it has, are both regressions — and they are
/// different failures, so both are counted.
#[test]
fn a_mention_that_loses_its_node_or_changes_node_counts_as_a_regression() {
    let ids = existing();

    // Attached, then NEW: the node it had is gone from the result.
    let lost = vec![replay("person-a", "person-a", "person-new", "new")];
    assert_eq!(count_regressions(&lost, &ids), 1);

    // Attached to A, now attached to B: the mention changed person.
    let moved = vec![replay("person-a", "person-a", "person-b", "alias")];
    assert_eq!(count_regressions(&moved, &ids), 1);

    // Both at once, plus a clean row.
    let mixed = vec![
        replay("person-a", "person-a", "person-new", "new"),
        replay("person-a", "person-a", "person-b", "alias"),
        replay("person-a", "person-a", "person-a", "name"),
    ];
    assert_eq!(count_regressions(&mixed, &ids), 2);
}

/// The WIN is not a regression: a mention that was going to create a node and
/// now binds to a real one.
#[test]
fn a_mention_that_gains_a_node_is_not_a_regression() {
    let ids = existing();
    let won = vec![replay("person-new", "person-new", "person-a", "alias")];
    assert_eq!(count_regressions(&won, &ids), 0);
}

/// A mention that was NEW and stays NEW is neither a win nor a regression.
#[test]
fn a_mention_that_was_new_and_stays_new_counts_as_neither() {
    let ids = existing();
    let unchanged = vec![replay("person-new", "person-new", "person-new", "new")];
    assert_eq!(count_regressions(&unchanged, &ids), 0);
}

// ── classify ────────────────────────────────────────────────────────────

fn index() -> PartyAliasIndex {
    PartyAliasIndex::build(vec![
        (PERSON, "person-a", "Alice Anderson"),
        (PERSON, "person-a", "Alice"),
        (PERSON, "person-b", "Bob Brown"),
        (PERSON, "person-c", "Contested"),
        (PERSON, "person-d", "Contested"),
    ])
}

#[test]
fn an_unchanged_attachment_is_reported_as_a_name_match() {
    let ids = existing();
    assert_eq!(
        classify(
            &index(),
            PERSON,
            "Alice Anderson",
            "person-a",
            "person-a",
            &ids
        ),
        "name",
    );
}

#[test]
fn a_new_attachment_via_an_alias_is_reported_as_an_alias_match() {
    let ids = existing();
    assert_eq!(
        classify(&index(), PERSON, "Alice", "NEW", "person-a", &ids),
        "alias",
    );
}

#[test]
fn a_contested_string_is_reported_as_ambiguous() {
    let ids = existing();
    assert_eq!(
        classify(
            &index(),
            PERSON,
            "Contested",
            "NEW",
            "person-contested",
            &ids
        ),
        "ambiguous",
    );
}

#[test]
fn a_role_word_is_reported_as_stoplisted() {
    let ids = existing();
    assert_eq!(
        classify(
            &index(),
            PERSON,
            "the Court",
            "NEW",
            "person-the-court",
            &ids
        ),
        "stoplist",
    );
}

#[test]
fn an_unknown_name_that_stays_new_is_reported_as_new() {
    let ids = existing();
    assert_eq!(
        classify(
            &index(),
            PERSON,
            "Nobody Known",
            "NEW",
            "person-nobody-known",
            &ids
        ),
        "new",
    );
}

// ── render ──────────────────────────────────────────────────────────────

/// The summary must carry the two must-be-zero counters by name, and must name
/// every regression it counted — a bare number would leave an operator no way to
/// act on it.
#[test]
fn the_summary_names_the_counters_and_lists_every_regression() {
    let ids = existing();
    let rows = vec![
        replay("person-a", "person-a", "person-a", "name"),
        replay("NEW", "NEW", "person-a", "alias"),
        replay("person-a", "person-a", "person-b", "alias"),
    ];
    let out = render(&rows, &index(), &ids);

    assert!(out.contains("Mentions walked            : 3"), "{out}");
    assert!(out.contains("Now resolve to an existing : 1"), "{out}");
    assert!(out.contains("Now NEW (were attached)    : 0"), "{out}");
    assert!(out.contains("Moved to a DIFFERENT node  : 1"), "{out}");
    assert!(
        out.contains("REGRESSION: doc-test \"Someone\" person-a -> person-b"),
        "a counted regression must be named, not just totalled: {out}"
    );
}

/// Ambiguous strings are reported even when no mention hit one this run — they
/// are a property of the graph, and they are the operator's merge worklist.
#[test]
fn the_report_lists_contested_strings_even_when_nothing_hit_them() {
    let out = render(&[], &index(), &existing());
    assert!(
        out.contains("STRINGS CLAIMED BY MORE THAN ONE NODE (1)"),
        "{out}"
    );
    assert!(out.contains("\"contested\""), "{out}");
    assert!(
        out.contains("person-c") && out.contains("person-d"),
        "both claimants must be named so the merge can be planned: {out}"
    );
}

// ── build_index ─────────────────────────────────────────────────────────────
//
// The properties-bag traversal is the join between what Neo4j returned and what
// the matcher compares. If it silently returned a name-only index, the fix would
// report zero effect on every run and look like a no-op rather than a break.

fn known(id: &str, name: &str, properties: serde_json::Value) -> colossus_extract::KnownEntity {
    colossus_extract::KnownEntity {
        entity_type: "Person".to_string(),
        id: id.to_string(),
        label: name.to_string(),
        properties,
    }
}

#[test]
fn build_index_reads_aliases_out_of_the_properties_bag() {
    let idx = PartyAliasIndex::from_known_entities(&[known(
        "person-judge-tighe",
        "Judge Tighe",
        serde_json::json!({"name": "Judge Tighe", "aliases": ["Karen A. Tighe", "Tighe"]}),
    )]);
    assert_eq!(
        idx.lookup("Person", "Karen A. Tighe"),
        AliasLookup::Matched("person-judge-tighe".to_string()),
    );
    // ...and the canonical name is indexed too, which is what makes a
    // name/alias collision detectable as ambiguous.
    assert_eq!(
        idx.lookup("Person", "Judge Tighe"),
        AliasLookup::Matched("person-judge-tighe".to_string()),
    );
}

#[test]
fn build_index_tolerates_a_node_with_no_aliases_key_or_a_wrong_shape() {
    let idx = PartyAliasIndex::from_known_entities(&[
        // Pre-alias-writer node: no `aliases` key at all.
        known(
            "person-a",
            "Alice Anderson",
            serde_json::json!({"name": "Alice Anderson", "role": "witness"}),
        ),
        // Present but the wrong JSON shape — a string, not an array.
        known(
            "person-b",
            "Bob Brown",
            serde_json::json!({"name": "Bob Brown", "aliases": "Bobby"}),
        ),
    ]);
    // Both names still index...
    assert_eq!(
        idx.lookup("Person", "Alice Anderson"),
        AliasLookup::Matched("person-a".to_string()),
    );
    assert_eq!(
        idx.lookup("Person", "Bob Brown"),
        AliasLookup::Matched("person-b".to_string()),
    );
    // ...and the malformed alias is skipped rather than panicking or binding.
    assert_eq!(idx.lookup("Person", "Bobby"), AliasLookup::NoMatch);
}

// ── classify: the two arms that stayed uncovered ────────────────────────────

/// The alias index finds node A, but the canonical name already resolved the
/// mention to node B. The name wins, and the report must say so.
#[test]
fn a_mention_whose_name_beat_its_alias_is_reported_as_a_name_match() {
    assert_eq!(
        classify(
            &index(),
            PERSON,
            "Alice",
            "person-b",
            "person-b",
            &existing()
        ),
        "name",
    );
}

/// The canonical name matched an existing node the alias index knows nothing
/// about — still a name match, not a "new".
#[test]
fn a_mention_the_index_does_not_know_but_the_name_resolved_is_a_name_match() {
    assert_eq!(
        classify(
            &index(),
            PERSON,
            "Unindexed Spelling",
            "NEW",
            "person-b",
            &existing()
        ),
        "name",
    );
}
