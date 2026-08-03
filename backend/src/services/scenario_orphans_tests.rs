// =============================================================================
// scenario_orphans_tests.rs — the D9 grouping, against the real 26
// =============================================================================
//
// The node ids below are the actual `scenario_fact_refs.graph_node_id` values on
// DEV for scenario S-2, read 2026-08-03. Real ids rather than invented ones,
// because the whole grouping rests on the Family-A id grammar and a made-up id
// would not prove it holds.

use super::*;

/// The live 26, verbatim: 16 from the discovery response, 8 from the CFS
/// interrogatory response, 2 from a court-of-appeals id that no longer exists.
fn the_live_twenty_six() -> Vec<String> {
    let discovery = [
        "5fd83f22", "1ef2612f", "839a18ab", "51d482bb", "2aebff35", "84f899ba", "ed80b860",
        "1947d7bd", "d11f9737", "0267a92b", "03d4a704", "c6579d65", "1234b958", "7262b4e1",
        "3a006b43", "700af54c",
    ];
    let interrogatory = [
        "5e73af15", "8bded562", "e277b4c7", "9c4d3886", "3b3dcb13", "1e0a415f", "e3486e27",
        "095d2301",
    ];
    let appeals = ["aa2176d2", "d9ea54a9"];

    let mut ids: Vec<String> = Vec::new();
    for hash in discovery {
        ids.push(format!(
            "doc-george-phillips-response-to-discovery:evidence:{hash}"
        ));
    }
    for hash in interrogatory {
        ids.push(format!(
            "doc-cfs-interrogatory-response-08-08-16:evidence:{hash}"
        ));
    }
    for hash in appeals {
        // Single `l` — see the module doc. The live document is `…rulling…`.
        ids.push(format!(
            "doc-court-of-appeals-ruling-01-12-2012:evidence:{hash}"
        ));
    }
    ids
}

/// The titles `documents` actually holds for those ids — measured. The
/// court-of-appeals id is deliberately absent, exactly as it is on DEV.
fn the_live_titles() -> HashMap<String, String> {
    HashMap::from([
        (
            "doc-george-phillips-response-to-discovery".to_string(),
            "GEORGE PHILLIPS RESPONSE TO DISCOVERY".to_string(),
        ),
        (
            "doc-cfs-interrogatory-response-08-08-16".to_string(),
            "CFS INTERROGATORY RESPONSE 08 08 16".to_string(),
        ),
    ])
}

#[test]
fn the_real_twenty_six_group_sixteen_eight_and_two() {
    // The measured split. It SUPERSEDES the mockup's illustrative 18/6/2 — which
    // the mockup itself says it would ("1.7C renders the real grouping").
    let response = group_orphans(&the_live_twenty_six(), &the_live_titles());

    assert_eq!(response.total, 26);
    assert_eq!(response.groups.len(), 3);
    let counts: Vec<usize> = response.groups.iter().map(|g| g.count).collect();
    assert_eq!(counts, vec![16, 8, 2], "biggest loss first");
}

#[test]
fn a_resolved_group_carries_the_documents_real_title() {
    let response = group_orphans(&the_live_twenty_six(), &the_live_titles());
    let biggest = &response.groups[0];

    assert_eq!(biggest.label, "GEORGE PHILLIPS RESPONSE TO DISCOVERY");
    assert_eq!(biggest.title_state, OrphanTitleState::Resolved);
    assert_eq!(biggest.count, 16);
}

#[test]
fn a_vanished_document_gets_an_id_slug_and_never_an_invented_title() {
    // §2.8, in terms: "it does not invent titles". These two refs point at
    // `doc-court-of-appeals-ruling-01-12-2012` (one `l`); the live document is
    // `doc-court-of-appeals-rulling-01-12-2012` (two). Guessing that they are the
    // same document, and labelling them "COURT OF APPEALS RULLING 01 12 2012",
    // would be the page asserting an identity nothing in the data supports.
    let response = group_orphans(&the_live_twenty_six(), &the_live_titles());
    let vanished = response
        .groups
        .iter()
        .find(|g| g.count == 2)
        .expect("the two-ref group");

    assert_eq!(vanished.title_state, OrphanTitleState::Unrecoverable);
    assert_eq!(vanished.label, "court-of-appeals-ruling-01-12-2012");
    assert!(
        !vanished.label.contains("RULLING"),
        "the label must not borrow the live document's title"
    );
    assert_eq!(
        vanished.document_id, "doc-court-of-appeals-ruling-01-12-2012",
        "the id is carried verbatim so the forensics stay checkable"
    );
}

#[test]
fn an_unrecognisable_id_becomes_its_own_group_rather_than_vanishing() {
    // Standing Rule 1: a total that quietly excludes what it could not parse is a
    // lying total, and the orphan strip's entire job is to not lose things.
    let ids = vec![
        "doc-a:evidence:1111".to_string(),
        "some-future-id-family-with-no-evidence-part".to_string(),
    ];
    let response = group_orphans(&ids, &HashMap::new());

    assert_eq!(response.total, 2);
    let unparseable = response
        .groups
        .iter()
        .find(|g| g.title_state == OrphanTitleState::Unparseable)
        .expect("the unparseable group must exist");
    assert_eq!(unparseable.count, 1);
    assert_eq!(unparseable.document_id, "");
}

#[test]
fn the_group_counts_always_sum_to_the_total() {
    // The invariant the summary line depends on: "26 saved references" over groups
    // adding to 24 would be visibly wrong on screen, and the reader would have no
    // way to know which number to trust.
    for ids in [
        the_live_twenty_six(),
        vec!["doc-a:evidence:1".to_string(), "garbage".to_string()],
        Vec::new(),
    ] {
        let response = group_orphans(&ids, &the_live_titles());
        let summed: usize = response.groups.iter().map(|g| g.count).sum();
        assert_eq!(summed, response.total, "groups must account for every ref");
    }
}

#[test]
fn nothing_orphaned_is_an_empty_list_not_an_error() {
    let response = group_orphans(&[], &HashMap::new());
    assert_eq!(response.total, 0);
    assert!(response.groups.is_empty());
}

#[test]
fn ordering_is_deterministic_when_counts_tie() {
    // A HashMap's iteration order is not stable, so two same-size groups would
    // swap places between requests and the strip would appear to reshuffle itself.
    let ids = vec![
        "doc-zebra:evidence:1".to_string(),
        "doc-alpha:evidence:2".to_string(),
    ];
    let first = group_orphans(&ids, &HashMap::new());
    let second = group_orphans(&ids, &HashMap::new());

    assert_eq!(first.groups, second.groups);
    assert_eq!(
        first.groups[0].label, "alpha",
        "equal counts break alphabetically"
    );
}

#[test]
fn the_distinct_document_ids_are_what_the_title_lookup_asks_for() {
    // Three documents behind 26 refs — the lookup must ask for three ids, not 26.
    let ids = document_ids_of(&the_live_twenty_six());
    assert_eq!(ids.len(), 3);
    assert!(ids.contains(&"doc-george-phillips-response-to-discovery".to_string()));
    assert!(ids.contains(&"doc-cfs-interrogatory-response-08-08-16".to_string()));
    assert!(ids.contains(&"doc-court-of-appeals-ruling-01-12-2012".to_string()));
}

#[test]
fn an_unparseable_id_contributes_no_document_to_look_up() {
    let ids = document_ids_of(&[
        "not-an-evidence-id".to_string(),
        ":evidence:abc".to_string(),
    ]);
    assert!(
        ids.is_empty(),
        "an empty document prefix is not a document id, got {ids:?}"
    );
}
