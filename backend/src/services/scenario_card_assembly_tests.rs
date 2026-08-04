//! Unit tests for [`super`] — the pool-level card assembly.
//!
//! These cover what this module owns: the working-pool / set-aside partition, the
//! stable C-code ordering, and the page-text join. The per-card §7 contract is
//! tested in `scenario_card_tests`.

use super::*;

/// The seeded snapshot, threaded through to `build_card` exactly as the handler
/// threads it — so these tests exercise the real parameter path.
fn settings() -> Settings {
    Settings::for_test()
}
use crate::bias::dto::{ActorOption, DocumentRef};

/// A candidate with everything the record can carry, on a given page.
fn full_instance() -> BiasInstance {
    BiasInstance {
        evidence_id: "ev-1".to_string(),
        title: "Interrogatory answer 14".to_string(),
        verbatim_quote: Some("I do not recall that meeting.".to_string()),
        question: Some("Did you attend the meeting on March 3, 2019?".to_string()),
        page_number: Some(14),
        pattern_tags: Vec::new(),
        stated_by: Some(ActorOption {
            id: "actor-1".to_string(),
            name: "R. Phillips".to_string(),
            actor_type: "person".to_string(),
            tagged_statement_count: 3,
        }),
        about: Vec::new(),
        document: Some(DocumentRef {
            id: "doc-7".to_string(),
            title: "CFS interrogatory responses".to_string(),
            document_type: Some("discovery_response".to_string()),
        }),
    }
}

// ── Partition, ordering and the page-text join ───────────────────────────────
//
// These moved here with `assemble`: they exercise pure domain logic (the
// set-aside partition, the C-code ordering, the page-text join), not HTTP.

/// A minimal pool candidate on a given page.
fn pool_item(id: &str, page: Option<i64>) -> BiasInstance {
    let mut instance = full_instance();
    instance.evidence_id = id.to_string();
    instance.page_number = page;
    instance
}

/// A ref-state map from `(node, status)` pairs.
fn states(pairs: &[(&str, FactStatus)]) -> HashMap<String, CardRefState> {
    pairs
        .iter()
        .map(|(id, status)| {
            (
                (*id).to_string(),
                CardRefState {
                    status: Some(*status),
                    ..CardRefState::default()
                },
            )
        })
        .collect()
}

fn ordinals(pairs: &[(&str, i32)]) -> HashMap<String, i32> {
    pairs
        .iter()
        .map(|(id, n)| ((*id).to_string(), *n))
        .collect()
}

#[test]
fn set_aside_items_are_partitioned_into_their_own_list() {
    // The client must not have to filter. Same split the gather endpoint serves.
    let response = assemble(
        vec![
            pool_item("ev-1", None),
            pool_item("ev-2", None),
            pool_item("ev-3", None),
        ],
        &HashMap::new(),
        &states(&[
            ("ev-1", FactStatus::Included),
            ("ev-2", FactStatus::Dropped),
            ("ev-3", FactStatus::Undecided),
        ]),
        &ordinals(&[("ev-1", 1), ("ev-2", 2), ("ev-3", 3)]),
        &HashMap::new(),
        &settings(),
        &Default::default(),
    );

    assert_eq!(response.pool.len(), 2, "included + undecided are the pool");
    assert_eq!(response.set_aside.len(), 1);
    assert_eq!(response.set_aside[0].graph_node_id, "ev-2");
}

#[test]
fn a_candidate_with_no_ref_row_is_undecided_and_in_the_pool() {
    // The derive-on-read contract: no row means nobody has ruled, which is a
    // working candidate, not a missing one.
    let response = assemble(
        vec![pool_item("ev-1", None)],
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &settings(),
        &Default::default(),
    );
    assert_eq!(response.pool.len(), 1);
    assert_eq!(response.pool[0].status_label, "Not yet decided");
}

#[test]
fn cards_sort_by_ordinal_numerically_not_lexicographically() {
    // "C-10" sorts before "C-9" as a string. The human's list must not reorder
    // itself when the tenth candidate arrives.
    let response = assemble(
        vec![pool_item("ev-a", None), pool_item("ev-b", None)],
        &HashMap::new(),
        &HashMap::new(),
        &ordinals(&[("ev-a", 10), ("ev-b", 9)]),
        &HashMap::new(),
        &settings(),
        &Default::default(),
    );
    assert_eq!(response.pool[0].code.as_deref(), Some("C-9"));
    assert_eq!(response.pool[1].code.as_deref(), Some("C-10"));
}

#[test]
fn un_numbered_candidates_sort_last_and_deterministically() {
    // A candidate gather has not numbered yet still appears — at the end, in a
    // stable position, rather than jumping around between requests.
    let response = assemble(
        vec![
            pool_item("ev-z", None),
            pool_item("ev-a", None),
            pool_item("ev-1", None),
        ],
        &HashMap::new(),
        &HashMap::new(),
        &ordinals(&[("ev-1", 1)]),
        &HashMap::new(),
        &settings(),
        &Default::default(),
    );
    assert_eq!(response.pool[0].code.as_deref(), Some("C-1"));
    assert_eq!(response.pool[1].graph_node_id, "ev-a");
    assert_eq!(response.pool[2].graph_node_id, "ev-z");
}

#[test]
fn page_text_is_joined_by_document_and_page() {
    // The index is keyed `doc:page`, so a candidate on page 2 must not pick up
    // page 1's text — that would put a quote in a context it never appeared in.
    let mut page_text = HashMap::new();
    page_text.insert(
        page_key("doc-7", 1),
        "PAGE ONE. I do not recall that meeting. tail one".to_string(),
    );
    page_text.insert(
        page_key("doc-7", 2),
        "PAGE TWO. I do not recall that meeting. tail two".to_string(),
    );

    let response = assemble(
        vec![pool_item("ev-1", Some(2))],
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &page_text,
        &settings(),
        &Default::default(),
    );
    assert!(response.pool[0].quote.context_before.contains("PAGE TWO"));
    assert!(!response.pool[0].quote.context_before.contains("PAGE ONE"));
}

#[test]
fn a_candidate_with_no_stored_page_text_still_serves_a_card() {
    // Quote-in-context is the one element that degrades rather than fails: the
    // quote, pinpoint and viewer link still get the human to the passage.
    let response = assemble(
        vec![pool_item("ev-1", Some(2))],
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &settings(),
        &Default::default(),
    );
    let card = &response.pool[0];
    assert!(card.quote.context_before.is_empty());
    assert_eq!(card.quote.text, "I do not recall that meeting.");
    assert_eq!(
        card.pinpoint.viewer_href,
        "/documents/doc-7?page=2&tab=document"
    );
}

#[test]
fn the_page_key_is_scoped_to_its_document() {
    // Two documents both have a page 14; the key must keep them apart.
    assert_ne!(page_key("doc-7", 14), page_key("doc-8", 14));
    assert_eq!(page_key("doc-7", 14), "doc-7:14");
}
