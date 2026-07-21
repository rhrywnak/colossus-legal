//! Unit tests for [`crate::api::pipeline::ingest_resolver`].
//!
//! Split into a sibling file (via `#[path]`) so the resolver module stays small —
//! the house pattern (`theme_scan_persist_tests.rs`, `scan_runs_tests.rs`).
//!
//! These pin the policies that stand between the extraction output and the
//! graph's identity model:
//!
//! 1. the **party_type adapter**, without which person matching cannot run at all
//!    — pinned both as a pure function AND as wired into the conversion path;
//! 2. the **auto-merge policy** — exact and normalized bind, fuzzy never does;
//! 3. the **resolver/writer filter contract** — the resolver must consider every
//!    row the party writer will write.
//!
//! `resolve_parties` reaches no external service (`NormalizedEntityResolver` does
//! only in-memory string comparison), so the whole identity contract is exercised
//! here with no Neo4j and no pipeline — including the behavioral tests, which
//! call the production entry point rather than its private helpers.

use super::*;

use crate::models::document_status::ENTITY_PARTY;

// ── Fixtures ─────────────────────────────────────────────────────────────────

fn props(party_type: &str) -> serde_json::Value {
    serde_json::json!({
        "party_name": "Karen A. Tighe",
        "role": "judge",
        "party_type": party_type,
    })
}

/// Build a Party row shaped the way the pipeline DB hands it to Ingest.
///
/// Only the four fields the resolver reads carry meaning — `entity_type` (the
/// filter), and `item_data`'s `label` (what the matcher compares) plus
/// `party_name` / `party_type` (identity and candidate-type selection).
/// `resolve_parties` reads no other field on `ExtractionItemRecord` — in
/// particular it does NOT filter on `review_status` or `graph_status`, because
/// approval is enforced upstream by the `get_approved_items_for_document` query
/// that loads these rows. The remaining fields are therefore inert scaffolding
/// held at neutral values, and a failing test here is traceable to one of the
/// four above rather than to an implicit status filter.
fn party_item(id: i32, entity_type: &str, name: &str, party_type: &str) -> ExtractionItemRecord {
    ExtractionItemRecord {
        id,
        run_id: 1,
        document_id: "doc-1".to_string(),
        entity_type: entity_type.to_string(),
        item_data: serde_json::json!({
            "label": name,
            "properties": {
                "party_name": name,
                "role": "judge",
                "party_type": party_type,
            },
        }),
        verbatim_quote: None,
        grounding_status: None,
        grounded_page: None,
        review_status: "approved".to_string(),
        reviewed_by: None,
        reviewed_at: None,
        review_notes: None,
        graph_status: "pending".to_string(),
        neo4j_node_id: None,
        resolved_entity_type: None,
    }
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

/// The adapter is WIRED, not merely correct.
///
/// The five tests above call [`normalize_party_type`] directly, so all of them
/// still pass if the call in [`to_extracted_entity`] is deleted — which is
/// precisely how the fix is reverted. This test closes that gap by going through
/// the real entry point: a template-shaped `party_type: "person"` row is handed
/// to the production [`resolve_parties`], and the assertion is that it MATCHED an
/// existing Person node.
///
/// That match is only reachable if the conversion path normalized the token:
/// upstream `compatible_type` selects Person candidates on `"individual"` and
/// returns FALSE for `"person"`, so without the adapter the candidate list is
/// empty, the matcher has nothing to compare, and the party resolves as a new
/// entity with a slug id.
///
/// ## Why through `resolve_parties` rather than exposing `to_extracted_entity`
///
/// Testing via the nearest public caller needs no `pub(crate)` widening and no
/// `#[cfg(test)]` accessor — production visibility stays as designed. It also
/// buys strictly more: it exercises the adapter, the filter, the real
/// three-step matcher and the upstream type contract together. A unit test on a
/// widened `to_extracted_entity` would assert the token is `"individual"` and
/// still pass if upstream changed the token it expects — this one fails, which
/// is the correct outcome for a broken contract.
#[tokio::test]
async fn a_person_matches_an_existing_node_through_the_real_conversion_path() {
    // The id deliberately is NOT `person-{slug(label)}`. Had it been
    // "person-karen-a-tighe", the id the NEW-entity branch mints from the same
    // name would be byte-identical, and the second assertion below would hold
    // whether or not the party actually matched — proving nothing. An id no slug
    // can produce makes "reused the existing node" and "minted a fresh one" two
    // distinguishable outcomes.
    let existing = vec![KnownEntity {
        entity_type: ENTITY_PERSON.to_string(),
        id: "person-legacy-0417".to_string(),
        label: "Karen A. Tighe".to_string(),
        properties: serde_json::json!({"name": "Karen A. Tighe", "role": "judge"}),
    }];
    // The shape every extraction template emits: party_type "person".
    let items = vec![party_item(1, ENTITY_PARTY, "Karen A. Tighe", "person")];

    let (map, summary) = resolve_parties(&items, &existing)
        .await
        .expect("resolution is pure in-memory and cannot fail here");

    assert_eq!(
        summary.matched_existing, 1,
        "a template-shaped 'person' row must reach the resolver as 'individual' \
         and match the existing Person node; 0 here means the adapter is not \
         wired into the conversion path and every name variant mints a new node"
    );
    assert_eq!(
        map.get("Karen A. Tighe").map(|r| r.neo4j_id.as_str()),
        Some("person-legacy-0417"),
        "the matched party must reuse the EXISTING node id; a slug-derived id here \
         means resolution fell through to the new-entity branch"
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

/// The resolver must ADMIT every entity type the writer will process.
///
/// `create_party_nodes` filters on all of `PARTY_SUBTYPES`; `resolve_parties`
/// must use the same predicate. If the two disagree, a row the writer will write
/// gets no resolution decision, and the writer falls through to a slug-derived id
/// — the duplicate-node path.
///
/// This asserts on `resolve_parties`'s OUTPUT, not on the contents of
/// `PARTY_SUBTYPES`. The previous version of this test checked that the constant
/// contained three strings, which was already true before the fix and is
/// separately guarded in `document_status`; it therefore passed unchanged when
/// the filter was reverted to `i.entity_type == "Party"`. `total_parties` is
/// computed directly from the filtered list, so it observes the predicate itself.
///
/// ## Scope note: a contract guard, not a live-bug regression test
///
/// The re-ingest scenario this fix was written for cannot currently arise:
/// `ITEM_SELECT_COLUMNS` projects the RAW `entity_type` (the COALESCE onto
/// `resolved_entity_type` was removed in `dc03f84`), so Ingest always sees
/// `"Party"` regardless of how many times a document is processed. Aligning the
/// two filters is still correct — the writer's filter is the wider one, and a
/// resolver that ignores rows the writer writes is a defect waiting for the next
/// producer — but this test guards the CONTRACT between the two functions, and
/// should not be read as proof that a re-ingest duplicate path was closed.
#[tokio::test]
async fn resolver_admits_every_party_type_the_writer_writes() {
    // One row per subtype, each with a distinct name so the per-name dedup
    // inside resolve_parties cannot collapse them and mask a dropped row.
    let items: Vec<ExtractionItemRecord> = PARTY_SUBTYPES
        .iter()
        .enumerate()
        .map(|(i, subtype)| party_item(i as i32, subtype, &format!("Party Number {i}"), "person"))
        .collect();
    // No existing nodes: this test is about which rows are CONSIDERED, so every
    // row resolving as a new entity is the expected outcome.
    let existing: Vec<KnownEntity> = Vec::new();

    let (map, summary) = resolve_parties(&items, &existing)
        .await
        .expect("resolution is pure in-memory and cannot fail here");

    assert_eq!(
        summary.total_parties,
        PARTY_SUBTYPES.len(),
        "every subtype the writer accepts must reach the resolver; a short count \
         means the two filters disagree and the writer will write rows that got \
         no resolution decision"
    );
    for (i, subtype) in PARTY_SUBTYPES.iter().enumerate() {
        let name = format!("Party Number {i}");
        assert!(
            map.contains_key(&name),
            "{subtype} row produced no resolution-map entry, so the writer would \
             fall through to a slug-derived id"
        );
    }
}

/// The filter must not be so wide it drags in non-party entities.
///
/// Those are written by `create_entity_node`'s generic path, which is the
/// INVERSE of the writer's Party filter; resolving them here would give them a
/// party id and double-write them.
#[tokio::test]
async fn resolver_ignores_entities_the_party_writer_does_not_own() {
    let items = vec![
        party_item(1, "Evidence", "Exhibit A", "person"),
        party_item(2, "Allegation", "Count One", "person"),
    ];
    let existing: Vec<KnownEntity> = Vec::new();

    let (map, summary) = resolve_parties(&items, &existing)
        .await
        .expect("resolution is pure in-memory and cannot fail here");

    assert_eq!(
        summary.total_parties, 0,
        "non-party entities must not be resolved as parties"
    );
    assert!(
        map.is_empty(),
        "non-party entities must contribute no resolution-map entries; an entry \
         here would hand a party id to an entity the generic writer also writes"
    );
}
