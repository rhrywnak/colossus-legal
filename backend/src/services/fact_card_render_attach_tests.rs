// Tests for `services::fact_card_render::attach_fact_cards`.
//
// `render_card` is exercised exhaustively next door; what is asserted HERE is the
// pairing — that every card in the response finds its stored row, and that a card
// with no stored row keeps `card: None` rather than an empty block. The bug this
// guards against is the cheap one: iterating `pool` and forgetting `set_aside`,
// which would silently blank every set-aside card's fact card on the working page.

use super::render_fixtures::*;
use super::*;

use crate::domain::confidence_band::ConfidenceBand;
use crate::domain::fact_status::FactStatus;
use crate::dto::scenario_card::{
    CardConfidence, CardPinpoint, CardQuote, CardSpeaker, ScenarioCard, ScenarioCardsResponse,
};

/// A card carrying nothing but its node id.
///
/// Built as a literal rather than through `build_card`: `attach_fact_cards` reads
/// exactly two fields (`graph_node_id` to look up, `card` to write), and routing
/// through the full builder would make these tests fail for reasons that have
/// nothing to do with attaching.
fn bare_card(node: &str) -> ScenarioCard {
    ScenarioCard {
        code: None,
        graph_node_id: node.to_string(),
        source_date: None,
        card: None,
        tier: None,
        sort_ordinal: None,
        display_ordinal: None,
        quote: CardQuote {
            text: "I do not recall.".to_string(),
            context_before: String::new(),
            context_after: String::new(),
            context_before_complete: true,
            context_after_complete: true,
            context_before_notice: None,
            context_after_notice: None,
            question: None,
            question_authorship: None,
        },
        pinpoint: CardPinpoint {
            document_id: "doc-tighe".to_string(),
            document_title: "Opinion and order".to_string(),
            label: "Opinion and order at 6".to_string(),
            page: Some(6),
            viewer_href: "/documents/doc-tighe?page=6&tab=document".to_string(),
        },
        speaker: CardSpeaker {
            name: None,
            attribution: "extracted".to_string(),
        },
        statement_kind: None,
        stance: None,
        bears_on: Vec::new(),
        grounding: None,
        confidence: CardConfidence {
            band: ConfidenceBand::Unscored,
            label: "Not scored".to_string(),
        },
        status: FactStatus::Undecided,
        status_label: "Not yet decided".to_string(),
        defer_required: false,
        defer_required_reason: None,
        defer_reason: None,
        proposed: None,
        scan_reason: None,
        human_links: Vec::new(),
        human_link_summary: None,
    }
}

fn response(pool: &[&str], set_aside: &[&str]) -> ScenarioCardsResponse {
    ScenarioCardsResponse {
        pool: pool.iter().map(|n| bare_card(n)).collect(),
        set_aside: set_aside.iter().map(|n| bare_card(n)).collect(),
        link_progress: None,
        no_target_notice: None,
        never_scanned_notice: None,
        proposal_source: None,
    }
}

/// Run the real `attach_fact_cards` over a response, with the stored rows named.
fn attach(response: &mut ScenarioCardsResponse, stored: &[&str]) {
    let allegations = allegations();
    let points = points();
    let wording = words();
    let cards: HashMap<String, CardRecord> = stored
        .iter()
        .map(|node| {
            let mut record = bare_record();
            record.graph_node_id = (*node).to_string();
            record.title = Some(format!("the card for {node}"));
            ((*node).to_string(), record)
        })
        .collect();
    attach_fact_cards(
        response,
        &cards,
        RenderContext {
            allegations: &allegations,
            talking_points: &points,
            wording: &wording,
        },
    );
}

#[test]
fn a_pool_card_with_a_stored_row_gets_its_card() {
    let mut payload = response(&["n1"], &[]);
    attach(&mut payload, &["n1"]);

    let block = payload.pool[0]
        .card
        .as_ref()
        .expect("the pool card should carry its fact card");
    assert_eq!(block.title.as_deref(), Some("the card for n1"));
}

/// THE REASON THIS FILE EXISTS. `set_aside` is the other half of the chain.
///
/// A set-aside card is a fact a human moved OUT of the running order but has not
/// deleted. Its drafted title, answer and watch-out are exactly what they need in
/// order to decide whether to bring it back, so blanking them would make the
/// decision harder on the one screen where it is made.
#[test]
fn a_set_aside_card_with_a_stored_row_also_gets_its_card() {
    let mut payload = response(&[], &["n2"]);
    attach(&mut payload, &["n2"]);

    let block = payload.set_aside[0]
        .card
        .as_ref()
        .expect("the set-aside card should carry its fact card too");
    assert_eq!(block.title.as_deref(), Some("the card for n2"));
}

/// A card nobody has drafted keeps `card: None` — absent, not an empty block.
///
/// The two states mean different things to the browser: `None` renders no card at
/// all, an empty block renders five blank rows and a draft-free title bar, which
/// would read as "somebody drafted this and left every field blank".
#[test]
fn a_card_with_no_stored_row_stays_absent() {
    let mut payload = response(&["n1", "n3"], &["n4"]);
    attach(&mut payload, &["n1"]);

    assert!(payload.pool[0].card.is_some(), "n1 has a stored row");
    assert!(payload.pool[1].card.is_none(), "n3 has none");
    assert!(payload.set_aside[0].card.is_none(), "n4 has none");
}

/// A stored row for a node that is in NEITHER list changes nothing.
///
/// The card map is a scenario-wide read; the response is the filtered deck. A row
/// for a fact that is no longer in either list is ordinary, and it must not
/// resurrect a card or panic.
#[test]
fn a_stored_row_with_no_card_in_the_response_is_ignored() {
    let mut payload = response(&["n1"], &[]);
    attach(&mut payload, &["n1", "not-in-the-deck"]);

    assert_eq!(payload.pool.len(), 1);
    assert!(payload.set_aside.is_empty());
}
