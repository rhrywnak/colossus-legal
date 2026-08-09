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
use crate::domain::confidence_band::ConfidenceBand;
use crate::repositories::pipeline_repository::{RuledCardReasonRow, ScenarioFactRefRecord};

/// A candidate with everything the record can carry, on a given page.
fn full_instance() -> BiasInstance {
    BiasInstance {
        evidence_id: "ev-1".to_string(),
        title: "Interrogatory answer 14".to_string(),
        verbatim_quote: Some("I do not recall that meeting.".to_string()),
        question: Some("Did you attend the meeting on March 3, 2019?".to_string()),
        statement_type: None,
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
        PoolIndexes {
            human: HumanTouchIndex {
                question_overrides: &Default::default(),
                links: &HashMap::new(),
            },
            proposals: &HashMap::new(),
        },
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
        PoolIndexes {
            human: HumanTouchIndex {
                question_overrides: &Default::default(),
                links: &HashMap::new(),
            },
            proposals: &HashMap::new(),
        },
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
        PoolIndexes {
            human: HumanTouchIndex {
                question_overrides: &Default::default(),
                links: &HashMap::new(),
            },
            proposals: &HashMap::new(),
        },
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
        PoolIndexes {
            human: HumanTouchIndex {
                question_overrides: &Default::default(),
                links: &HashMap::new(),
            },
            proposals: &HashMap::new(),
        },
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
        PoolIndexes {
            human: HumanTouchIndex {
                question_overrides: &Default::default(),
                links: &HashMap::new(),
            },
            proposals: &HashMap::new(),
        },
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
        PoolIndexes {
            human: HumanTouchIndex {
                question_overrides: &Default::default(),
                links: &HashMap::new(),
            },
            proposals: &HashMap::new(),
        },
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

// ── The projection reaching the cards (2026-08-08) ───────────────────────────

/// One admitted verdict, folded into the group the assembler is handed.
fn group(node: &str, role: &str, confidence: f32) -> ProposalGroup {
    ProposalGroup {
        representative: node.to_string(),
        covers: vec![node.to_string()],
        role: Some(role.to_string()),
        confidence: Some(confidence),
        reason: Some("the judge's reason".to_string()),
    }
}

/// The assembler's own index shape: representative node → its group.
fn proposal_index<'a>(groups: &'a [ProposalGroup]) -> ProposalIndex<'a> {
    groups
        .iter()
        .map(|g| (g.representative.as_str(), g))
        .collect()
}

fn assemble_with(
    pool: Vec<BiasInstance>,
    ref_states: &HashMap<String, CardRefState>,
    proposals: &ProposalIndex<'_>,
    settings: &Settings,
) -> ScenarioCardsResponse {
    assemble(
        pool,
        &HashMap::new(),
        ref_states,
        &ordinals(&[("ev-1", 1), ("ev-2", 2)]),
        &HashMap::new(),
        settings,
        PoolIndexes {
            human: HumanTouchIndex {
                question_overrides: &Default::default(),
                links: &HashMap::new(),
            },
            proposals,
        },
    )
}

#[test]
fn a_projected_verdict_reaches_its_card_as_a_proposal() {
    // THE path the whole projection exists to deliver: a node with NO reference
    // row, carrying a live verdict, comes out of the assembler as a card that says
    // what the scan proposed — role, reason, and a BAND computed from the verdict's
    // own score. Before this, a card with no row got the empty default and the
    // scan's judgment reached the human only through a Merge they had to press.
    let settings = settings();
    let groups = [group("ev-1", "supports", 0.91)];

    let response = assemble_with(
        vec![pool_item("ev-1", Some(14))],
        &HashMap::new(),
        &proposal_index(&groups),
        &settings,
    );

    let card = &response.pool[0];
    let proposal = card.proposed.as_ref().expect("the node was proposed");
    // Ruling R3: the reason rides on the CARD, not on the proposal. Asserted here
    // rather than on `proposal` because that is the whole change — the sentence
    // has to outlive the proposal that a ruling ends.
    assert_eq!(card.scan_reason.as_deref(), Some("the judge's reason"));
    assert!(
        proposal
            .role_label
            .as_deref()
            .is_some_and(|l| l.contains("supports")),
        "the chip carries the canon stance word: {:?}",
        proposal.role_label
    );
    // The band comes from the VERDICT's score — the same number the retired merge
    // used to copy into the column — so a proposed card is banded like any other.
    assert_eq!(card.confidence.band, ConfidenceBand::High);
    // And it is still an unruled candidate: a proposal is a view, not a row.
    assert_eq!(card.status, FactStatus::Undecided);
    assert!(
        card.tier.is_none(),
        "nothing has weighed a card nobody has ruled"
    );
}

#[test]
fn a_node_with_a_reference_row_ignores_the_projection_entirely() {
    // R-a at the assembly seam. `project` already drops ruled nodes, so a
    // populated entry here could only arrive from a future caller that forgot —
    // and the assembler must still prefer the stored row rather than letting a
    // verdict overwrite a human's decision on the way to the screen.
    let settings = settings();
    let groups = [group("ev-1", "supports", 0.91)];

    let response = assemble_with(
        vec![pool_item("ev-1", Some(14))],
        &states(&[("ev-1", FactStatus::Included)]),
        &proposal_index(&groups),
        &settings,
    );

    let card = &response.pool[0];
    assert!(card.proposed.is_none(), "a ruled card is never a proposal");
    assert_eq!(card.status, FactStatus::Included);
}

#[test]
fn count_proposed_counts_across_both_lists() {
    // The live number (R-e), counted over the finished payload so it and the cards
    // it describes are the same set by construction.
    let settings = settings();
    let groups = [group("ev-1", "supports", 0.91)];

    // One proposed card, one ruled-out card that lands in `set_aside`.
    let response = assemble_with(
        vec![pool_item("ev-1", Some(14)), pool_item("ev-2", Some(15))],
        &states(&[("ev-2", FactStatus::Dropped)]),
        &proposal_index(&groups),
        &settings,
    );

    assert_eq!(response.pool.len(), 1);
    assert_eq!(response.set_aside.len(), 1);
    assert_eq!(
        count_proposed(&response),
        1,
        "exactly the proposed card is counted"
    );

    // Both lists are walked deliberately. An excluded card can carry no proposal
    // today — precedence forbids it — so this asserts the WALK rather than a
    // reachable state: a future change to the partition must not be able to lose a
    // number by moving a card between the two lists.
    let mut moved = response;
    let card = moved.pool.remove(0);
    moved.set_aside.push(card);
    assert_eq!(
        count_proposed(&moved),
        1,
        "the count follows the card across the partition"
    );
}

#[test]
fn nothing_proposed_is_a_zero_count_and_no_proposal_fields() {
    // The ordinary state of every scenario nothing has scanned, and of every one
    // whose picks are all ruled. It must be a clean zero rather than an absence
    // the caller has to interpret.
    let settings = settings();
    let response = assemble_with(
        vec![pool_item("ev-1", Some(14))],
        &HashMap::new(),
        &ProposalIndex::new(),
        &settings,
    );

    assert_eq!(count_proposed(&response), 0);
    assert!(response.pool[0].proposed.is_none());
    assert_eq!(
        response.pool[0].confidence.band,
        ConfidenceBand::Unscored,
        "no verdict, no band — never a fabricated score"
    );
}

// ─── The scan's reason survives the ruling (ONE_CARD_GRAMMAR, ruling R3) ─────

/// One stored reference row, with only the fields these two tests read.
///
/// Written out rather than defaulted because `ScenarioFactRefRecord` has no
/// `Default` and should not gain one: it mirrors a table whose every column is
/// either NOT NULL or meaningfully nullable, and a `..Default::default()` here
/// would quietly invent a status.
fn ref_row(node: &str, source_run_id: Option<uuid::Uuid>) -> ScenarioFactRefRecord {
    ScenarioFactRefRecord {
        scenario_id: uuid::Uuid::nil(),
        graph_node_id: node.to_string(),
        role_in_this_scenario: None,
        status: "included".to_string(),
        note: None,
        confidence: None,
        source_run_id,
        tagged_at: chrono::Utc::now(),
        defer_reason: None,
        tier: "backup".to_string(),
        sort_ordinal: None,
        tier_updated_by: None,
        tier_updated_at: None,
        order_updated_by: None,
        order_updated_at: None,
    }
}

#[test]
fn an_included_cards_reason_is_served_from_its_source_run() {
    // THE defect ruling R3 kills. Precedence law R-a drops every node with a
    // reference row before the projection groups anything, so the instant a human
    // pressed Include the judge's sentence was gone from the payload — and that
    // sentence is the one-sentence answer to "why does this matter here", which is
    // exactly what an included fact has to keep answering.
    let run = uuid::Uuid::from_u128(7);
    let refs = vec![ref_row("ev-1", Some(run))];

    let (runs, nodes) = ruled_reason_keys(&refs);
    assert_eq!(runs, vec![run], "the ruling's own run is what gets asked");
    assert_eq!(nodes, vec!["ev-1".to_string()]);

    let mut states = build_ref_states(refs).expect("a well-formed row decodes");
    attach_ruled_reasons(
        &mut states,
        vec![RuledCardReasonRow {
            run_id: run,
            graph_node_id: "ev-1".to_string(),
            reason: Some("they admit the request existed".to_string()),
        }],
    );

    assert_eq!(
        states["ev-1"].scan_reason.as_deref(),
        Some("they admit the request existed"),
    );
    assert!(
        states["ev-1"].proposal.is_none(),
        "recovering the reason must not resurrect the PROPOSAL — a ruled card is \
         not proposed, and the Proposed filter reads exactly that field",
    );
}

#[test]
fn a_null_provenance_ruling_serves_no_reason() {
    // Rulings made before `source_run_id` was written cite no run, so there is
    // nothing to look a reason up by. The card shows none — the absent-not-fake
    // law — and, just as importantly, the row is never even asked about: a query
    // guaranteed to return nothing is work nobody uses.
    let refs = vec![ref_row("ev-1", None)];

    let (runs, nodes) = ruled_reason_keys(&refs);
    assert!(
        runs.is_empty() && nodes.is_empty(),
        "a ruling with no provenance contributes no lookup key",
    );

    let states = build_ref_states(refs).expect("a well-formed row decodes");
    assert!(states["ev-1"].scan_reason.is_none());
}

#[test]
fn a_reason_for_a_node_this_payload_does_not_serve_is_ignored() {
    // A verdict whose reference row has since been removed describes a card this
    // response does not carry. Inserting a bare state for it would put a card-less
    // entry in the map that every later pass would have to know to skip.
    let run = uuid::Uuid::from_u128(7);
    let mut states =
        build_ref_states(vec![ref_row("ev-1", Some(run))]).expect("a well-formed row decodes");

    attach_ruled_reasons(
        &mut states,
        vec![RuledCardReasonRow {
            run_id: run,
            graph_node_id: "ev-gone".to_string(),
            reason: Some("about a card that is not here".to_string()),
        }],
    );

    assert_eq!(states.len(), 1);
    assert!(states["ev-1"].scan_reason.is_none());
}
