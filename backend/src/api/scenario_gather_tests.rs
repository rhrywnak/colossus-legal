//! Unit tests for [`crate::api::scenario_gather`].
//!
//! Split into a sibling file (via `#[path]`) so the handler module stays within
//! the module-size limit — the same discipline as `theme_scan_persist_tests.rs`
//! and `scan_runs_tests.rs`. These are pure-function tests: `reconcile_candidates`
//! and the ordering helper take their inputs as plain values, so the whole
//! candidate-derivation contract is exercised with no database and no graph.

use super::*;

/// A minimal `BiasInstance` carrying just an id — enough to drive reconcile.
fn content(evidence_id: &str) -> BiasInstance {
    BiasInstance {
        evidence_id: evidence_id.to_string(),
        title: String::new(),
        verbatim_quote: None,
        question: None,
        page_number: None,
        pattern_tags: Vec::new(),
        stated_by: None,
        about: Vec::new(),
        document: None,
    }
}

/// A `scenario_fact_refs` row with the given node id, raw status token, and
/// optional role/note. Confidence defaults to `None` (an unscored / human-
/// curated ref); tests that exercise the confidence path use [`scored_ref`].
fn fact_ref(
    node: &str,
    status: &str,
    role: Option<&str>,
    note: Option<&str>,
) -> ScenarioFactRefRecord {
    ScenarioFactRefRecord {
        scenario_id: Uuid::nil(),
        graph_node_id: node.to_string(),
        role_in_this_scenario: role.map(str::to_string),
        status: status.to_string(),
        note: note.map(str::to_string),
        confidence: None,
        // Human-authored: no scan put this row here (the permanent semantics of
        // NULL, not a placeholder) — matches the `confidence: None` above.
        source_run_id: None,
        tagged_at: chrono::DateTime::<chrono::Utc>::UNIX_EPOCH,
    }
}

/// A merged/scanned ref: an `undecided` row carrying a model role + confidence
/// (exactly what the set-as-basis merge writes). Kept separate from [`fact_ref`]
/// so the common human-curated case stays terse while the scored case is loud
/// about what it is testing.
fn scored_ref(node: &str, role: &str, confidence: f32) -> ScenarioFactRefRecord {
    ScenarioFactRefRecord {
        scenario_id: Uuid::nil(),
        graph_node_id: node.to_string(),
        role_in_this_scenario: Some(role.to_string()),
        status: "undecided".to_string(),
        note: None,
        confidence: Some(confidence),
        // A merged row always knows which run scored it — the merge stamps this
        // alongside role and confidence, so a scored ref with a NULL source
        // would be an impossible state to fixture.
        source_run_id: Some(Uuid::nil()),
        tagged_at: chrono::DateTime::<chrono::Utc>::UNIX_EPOCH,
    }
}

#[test]
fn miss_is_undecided_and_lands_in_pool() {
    // A live pool node with NO ref row is derived Undecided, in the working
    // pool, WITHOUT any persistence (this is a pure fn — there is nothing to
    // persist to, which is the point of derive-on-read).
    let response = reconcile_candidates(vec![content("ev-1")], Vec::new(), &HashMap::new())
        .expect("no decode to fail");

    assert_eq!(response.pool.len(), 1);
    assert!(response.dropped.is_empty());
    assert_eq!(response.pool[0].content.evidence_id, "ev-1");
    assert_eq!(response.pool[0].status, FactStatus::Undecided);
    assert!(response.pool[0].role.is_none());
    assert!(response.pool[0].note.is_none());
    // A miss was never scored for this scenario → confidence is None ("unscored"),
    // NOT Some(0.0). The card must be able to tell the two apart (Standing Rule 1).
    assert!(
        response.pool[0].confidence.is_none(),
        "an undecided miss has no model confidence — None, never Some(0.0)"
    );
}

#[test]
fn scored_undecided_ref_carries_role_and_confidence_into_the_pool() {
    // The set-as-basis merge writes undecided rows with a model role + confidence.
    // Both must survive reconcile onto the CandidateDto so the workbench can
    // render "corroborates · 85%" (the whole point of this chunk).
    let refs = vec![scored_ref("ev-1", "corroborates", 0.85)];
    let response =
        reconcile_candidates(vec![content("ev-1")], refs, &HashMap::new()).expect("known token");

    assert_eq!(
        response.pool.len(),
        1,
        "an undecided scored pick stays in the pool"
    );
    assert_eq!(response.pool[0].status, FactStatus::Undecided);
    assert_eq!(response.pool[0].role.as_deref(), Some("corroborates"));
    assert_eq!(response.pool[0].confidence, Some(0.85));
}

#[test]
fn human_curated_ref_has_no_confidence() {
    // A human include with a NULL confidence column must reconcile to None, not
    // 0.0 — a hand-curated fact carries no *model* score and reads "unscored".
    let refs = vec![fact_ref("ev-1", "included", Some("rebuts"), None)];
    let response =
        reconcile_candidates(vec![content("ev-1")], refs, &HashMap::new()).expect("known token");

    assert_eq!(response.pool[0].status, FactStatus::Included);
    assert_eq!(response.pool[0].role.as_deref(), Some("rebuts"));
    assert!(
        response.pool[0].confidence.is_none(),
        "a human-curated ref (NULL confidence) must be None, never Some(0.0)"
    );
}

#[test]
fn included_ref_lands_in_pool_with_role_and_note() {
    let refs = vec![fact_ref(
        "ev-1",
        "included",
        Some("rebuts"),
        Some("key denial"),
    )];
    let response =
        reconcile_candidates(vec![content("ev-1")], refs, &HashMap::new()).expect("known token");

    assert_eq!(
        response.pool.len(),
        1,
        "included belongs in the working pool"
    );
    assert!(response.dropped.is_empty());
    assert_eq!(response.pool[0].status, FactStatus::Included);
    assert_eq!(response.pool[0].role.as_deref(), Some("rebuts"));
    assert_eq!(response.pool[0].note.as_deref(), Some("key denial"));
}

#[test]
fn dropped_ref_lands_in_its_own_list() {
    let refs = vec![fact_ref("ev-1", "dropped", None, None)];
    let response =
        reconcile_candidates(vec![content("ev-1")], refs, &HashMap::new()).expect("known token");

    assert!(
        response.pool.is_empty(),
        "a dropped candidate must NOT appear in the working pool"
    );
    assert_eq!(response.dropped.len(), 1, "dropped goes in its own list");
    assert_eq!(response.dropped[0].status, FactStatus::Dropped);
    assert_eq!(response.dropped[0].content.evidence_id, "ev-1");
}

#[test]
fn undecided_and_included_share_the_pool_dropped_is_split_out() {
    // Three nodes, three fates: one undecided (no ref), one included, one
    // dropped. The pool holds the first two; dropped holds the third.
    let pool = vec![
        content("ev-undecided"),
        content("ev-included"),
        content("ev-dropped"),
    ];
    let refs = vec![
        fact_ref("ev-included", "included", None, None),
        fact_ref("ev-dropped", "dropped", None, None),
    ];
    let response = reconcile_candidates(pool, refs, &HashMap::new()).expect("known tokens");

    assert_eq!(response.pool.len(), 2);
    assert_eq!(response.dropped.len(), 1);
    assert_eq!(response.dropped[0].content.evidence_id, "ev-dropped");
}

#[test]
fn a_ref_with_no_matching_pool_node_is_simply_absent() {
    // The pool drives output: a ref pointing at a node NOT in the pool (e.g.
    // its Evidence was re-ingested under a new id) contributes no candidate.
    // It is not invented into the output, and — being neither dropped-in-pool
    // nor pool — it simply does not appear. (1a.3's un-drop tray, not gather,
    // is where such a ref would resurface.)
    let refs = vec![fact_ref("ev-orphan-ref", "included", None, None)];
    let response =
        reconcile_candidates(vec![content("ev-1")], refs, &HashMap::new()).expect("known token");

    assert_eq!(response.pool.len(), 1);
    assert_eq!(response.pool[0].content.evidence_id, "ev-1");
    assert_eq!(response.pool[0].status, FactStatus::Undecided);
}

#[test]
fn unknown_status_token_errs_loudly_not_bucketed() {
    // Standing Rule 1: a persisted status this build cannot interpret is a
    // data-integrity fault — a loud Err, NEVER silently bucketed as undecided.
    let refs = vec![fact_ref("ev-1", "archived", None, None)];
    let result = reconcile_candidates(vec![content("ev-1")], refs, &HashMap::new());

    assert!(
        matches!(result, Err(AppError::Internal { .. })),
        "an unrecognized status token must fail loudly, not default to undecided"
    );
}

/// Build an ordinal index from `(node, ordinal)` pairs.
fn ordinals(pairs: &[(&str, i32)]) -> HashMap<String, i32> {
    pairs.iter().map(|(n, o)| (n.to_string(), *o)).collect()
}

#[test]
fn the_ordinal_rides_onto_the_candidate() {
    // The chip the human speaks ("look at C-14") must reach the card. A
    // candidate with an assigned ordinal carries it; one without carries None
    // rather than a fabricated number.
    let index = ordinals(&[("ev-1", 14)]);
    let response = reconcile_candidates(vec![content("ev-1"), content("ev-2")], Vec::new(), &index)
        .expect("no decode to fail");

    let c1 = &response.pool[0];
    assert_eq!(c1.content.evidence_id, "ev-1");
    assert_eq!(c1.ordinal, Some(14));

    let c2 = &response.pool[1];
    assert_eq!(c2.content.evidence_id, "ev-2");
    assert!(
        c2.ordinal.is_none(),
        "an unnumbered candidate must carry None, never a positional stand-in"
    );
}

#[test]
fn the_pool_is_ordered_by_ordinal_regardless_of_graph_order() {
    // The graph hands back its own order; the workbench's contract is ascending
    // candidate-id order, so gather sorts. C-2 must sit between C-1 and C-10 —
    // this also pins that ordering is NUMERIC, not the lexicographic order a
    // string chip would produce ("C-10" < "C-2").
    let index = ordinals(&[("ev-c10", 10), ("ev-c1", 1), ("ev-c2", 2)]);
    let response = reconcile_candidates(
        vec![content("ev-c10"), content("ev-c2"), content("ev-c1")],
        Vec::new(),
        &index,
    )
    .expect("no decode to fail");

    let order: Vec<i32> = response.pool.iter().filter_map(|c| c.ordinal).collect();
    assert_eq!(order, vec![1, 2, 10], "pool must be in ascending id order");
}

#[test]
fn ruling_on_a_card_never_moves_it() {
    // The side-effect-free ordering guarantee: a human's Include must not
    // reshuffle the list under them. Same pool, same ordinals, one card ruled
    // on — the order is identical. (Their spatial memory of the list is part of
    // their curation state; a list that jumps destroys it.)
    let index = ordinals(&[("ev-a", 1), ("ev-b", 2), ("ev-c", 3)]);
    let pool = || vec![content("ev-a"), content("ev-b"), content("ev-c")];

    let before = reconcile_candidates(pool(), Vec::new(), &index).expect("known tokens");
    // ev-b is now Included AND carries a high model score — neither may promote
    // or demote it.
    let after = reconcile_candidates(
        pool(),
        vec![fact_ref("ev-b", "included", Some("rebuts"), None)],
        &index,
    )
    .expect("known tokens");

    let ids = |r: &GatherCandidatesResponse| -> Vec<String> {
        r.pool
            .iter()
            .map(|c| c.content.evidence_id.clone())
            .collect()
    };
    assert_eq!(
        ids(&before),
        ids(&after),
        "a ruling must never change a card's position"
    );
}

#[test]
fn dropped_candidates_keep_their_ordinals_and_their_own_order() {
    // Drop excludes, it never deletes — and the id survives, because "we looked
    // at C-31 and dropped it" must stay sayable. The dropped list is ordered by
    // the same rule as the pool.
    let index = ordinals(&[("ev-x", 31), ("ev-y", 7)]);
    let refs = vec![
        fact_ref("ev-x", "dropped", None, None),
        fact_ref("ev-y", "dropped", None, None),
    ];
    let response = reconcile_candidates(vec![content("ev-x"), content("ev-y")], refs, &index)
        .expect("known tokens");

    assert!(response.pool.is_empty());
    let order: Vec<i32> = response.dropped.iter().filter_map(|c| c.ordinal).collect();
    assert_eq!(order, vec![7, 31], "dropped keeps its ids, in id order");
}

#[test]
fn fallback_definition_has_no_target() {
    // The gather fallback must have `target: None` so the shared resolver
    // falls through to the case default — the whole reason it exists.
    assert!(fallback_definition().target.is_none());
}

#[test]
fn map_subject_error_unresolvable_is_503_naming_config_key() {
    // An unresolvable subject is a MISCONFIGURATION → 503, and the message
    // must name the env var that fixes it (distinct from a 200 empty pool).
    let err = map_subject_error(Uuid::nil(), SubjectResolveError::Unresolvable);
    match err {
        AppError::ServiceUnavailable { message } => assert!(
            message.contains("CASE_DEFAULT_SUBJECT_NAME"),
            "503 message must name the config key: {message}"
        ),
        other => panic!("expected 503 ServiceUnavailable, got {other:?}"),
    }
}

#[test]
fn map_subject_error_lookup_failed_is_internal_500() {
    use serde::de::Error as _;
    // A graph fault while resolving the default subject is a server-side 500,
    // not a config problem. Construct the wrapped BiasRepositoryError via
    // serde's `custom` so no live Neo4j is needed (mirrors theme_scan's tests).
    let source = crate::bias::repository::BiasRepositoryError::Deserialize(
        neo4rs::DeError::custom("subjects query failed"),
    );
    let err = map_subject_error(
        Uuid::nil(),
        SubjectResolveError::DefaultLookupFailed { source },
    );
    assert!(
        matches!(err, AppError::Internal { .. }),
        "a graph-layer lookup fault must map to 500 Internal"
    );
}
