// Tests for `services::rehearsal_card_sections` — the two view assemblers.
//
// The three rules underneath (who counts as the other side, what order things
// read in, which point a card backs) are `services::rehearsal_cards`' and are
// tested there. What is asserted HERE is the layer this module adds: turning a
// card's stored ordinal into the C-code a reader sees, and carrying the stored
// gap sentences onto the wire.
//
// The C-code matters more than it looks. It is how Marie says "this one" out
// loud with a binder open in front of her, so a code on the wrong card, or a
// code invented for a card that has none, is worse than no code at all.

use super::*;

use crate::domain::settings::Settings;
use crate::repositories::pipeline_repository::scenario_fact_cards::CardRecord;
use crate::repositories::scenario_accusation_repository::RehearsalFactRow;
use chrono::{DateTime, Utc};
use uuid::Uuid;

fn at() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_757_000_000, 0).expect("a valid instant")
}

/// One card, blank apart from its title. Each test sets the fields it is about.
fn card(node: &str) -> CardRecord {
    CardRecord {
        scenario_id: Uuid::nil(),
        graph_node_id: node.to_string(),
        title: Some(format!("title of {node}")),
        backs_position: None,
        supports: None,
        watch_out: None,
        answer: None,
        title_authored_by: None,
        backs_position_authored_by: None,
        supports_authored_by: None,
        watch_out_authored_by: None,
        answer_authored_by: None,
        authored_by: "roman".to_string(),
        created_at: at(),
        edited_at: at(),
    }
}

fn fact(node: &str, speaker: Option<&str>, when: Option<&str>) -> RehearsalFactRow {
    RehearsalFactRow {
        graph_node_id: node.to_string(),
        quote: Some(format!("the words of {node}")),
        speaker: speaker.map(str::to_string),
        statement_type: Some("court_finding".to_string()),
        question: None,
        occurred_on: when.map(str::to_string),
        document_id: Some("doc-x".to_string()),
        document_title: Some("A Document".to_string()),
        page: Some(6),
    }
}

fn index<T>(rows: Vec<(String, T)>) -> HashMap<String, T> {
    rows.into_iter().collect()
}

fn ours() -> Vec<String> {
    vec!["marie awad".to_string()]
}

/// One card by the other side, answered, with a watch-out — enough to exercise
/// every branch of `card_sections` at once.
fn one_card_deck() -> (
    HashMap<String, CardRecord>,
    HashMap<String, RehearsalFactRow>,
) {
    let mut c = card("n1");
    c.answer = Some("The order speaks for itself.".to_string());
    c.watch_out = Some("They will call the opinion final.".to_string());
    (
        index(vec![("n1".to_string(), c)]),
        index(vec![(
            "n1".to_string(),
            fact("n1", Some("Judge Tighe"), Some("2012-04-12")),
        )]),
    )
}

// ─── The C-code ──────────────────────────────────────────────────────────────

/// A card with a stored ordinal is given its C-code.
#[test]
fn an_accusation_card_carries_the_code_its_ordinal_mints() {
    let (cards, facts) = one_card_deck();
    let ordinals = index(vec![("n1".to_string(), 116)]);
    let settings = Settings::for_test();

    let sections = card_sections(
        rehearsal_cards::DeckInput {
            cards: &cards,
            facts: &facts,
            ordinals: &ordinals,
            our_side: &ours(),
        },
        &settings,
    );

    assert_eq!(sections.accusation.len(), 1);
    assert_eq!(sections.accusation[0].code.as_deref(), Some("C-116"));
    assert_eq!(sections.accusation[0].who.as_deref(), Some("Judge Tighe"));
    assert_eq!(sections.accusation[0].when.as_deref(), Some("2012-04-12"));
}

/// THE FAILURE THAT MATTERS. A card with NO ordinal carries NO code.
///
/// A candidate that has never been numbered has no handle, and minting one from
/// its position in the list would give Marie a code that means nothing to anyone
/// holding the binder — and that would change the next time a card moved.
#[test]
fn a_card_with_no_ordinal_carries_no_code() {
    let (cards, facts) = one_card_deck();
    let ordinals = HashMap::new();
    let settings = Settings::for_test();

    let sections = card_sections(
        rehearsal_cards::DeckInput {
            cards: &cards,
            facts: &facts,
            ordinals: &ordinals,
            our_side: &ours(),
        },
        &settings,
    );

    assert!(sections.accusation[0].code.is_none());
    // The card is still SHOWN. A missing handle never hides a statement she has
    // to answer.
    assert_eq!(sections.accusation[0].title.as_deref(), Some("title of n1"));
}

// ─── The stored sentences ────────────────────────────────────────────────────

/// Both gap sentences come off the settings store, never a literal.
#[test]
fn the_gap_sentences_are_the_stored_ones() {
    let (cards, facts) = one_card_deck();
    let ordinals = index(vec![("n1".to_string(), 116)]);
    let settings = Settings::for_test();

    let sections = card_sections(
        rehearsal_cards::DeckInput {
            cards: &cards,
            facts: &facts,
            ordinals: &ordinals,
            our_side: &ours(),
        },
        &settings,
    );

    assert_eq!(sections.gap, settings.fact_card_wording.no_cards_notice);
    assert_eq!(
        sections.accusation[0].answer_gap,
        settings.fact_card_wording.no_answer_notice
    );
}

/// The answer gap rides on EVERY card, answered or not.
///
/// The section renders a mixed list. One notice held at section level would make
/// the browser decide per row whether it applied — a judgment about an absence,
/// which the honest-gap law puts on this side of the wire.
#[test]
fn the_answer_gap_rides_on_a_card_that_already_has_an_answer() {
    let (cards, facts) = one_card_deck();
    let ordinals = index(vec![("n1".to_string(), 116)]);
    let settings = Settings::for_test();

    let sections = card_sections(
        rehearsal_cards::DeckInput {
            cards: &cards,
            facts: &facts,
            ordinals: &ordinals,
            our_side: &ours(),
        },
        &settings,
    );

    assert!(sections.accusation[0].answer.is_some(), "it is answered");
    assert!(!sections.accusation[0].answer_gap.is_empty());
}

// ─── What to watch for ───────────────────────────────────────────────────────

#[test]
fn a_watch_out_reaches_the_wire_with_its_code() {
    let (cards, facts) = one_card_deck();
    let ordinals = index(vec![("n1".to_string(), 116)]);
    let settings = Settings::for_test();

    let sections = card_sections(
        rehearsal_cards::DeckInput {
            cards: &cards,
            facts: &facts,
            ordinals: &ordinals,
            our_side: &ours(),
        },
        &settings,
    );

    assert_eq!(sections.watch_for.len(), 1);
    assert_eq!(sections.watch_for[0].code.as_deref(), Some("C-116"));
    assert_eq!(
        sections.watch_for[0].text,
        "They will call the opinion final."
    );
}

/// A deck with no cards produces empty lists and the stored sentence — never a
/// section that silently is not there.
#[test]
fn an_empty_deck_still_produces_the_sections_and_the_notice() {
    let cards = HashMap::new();
    let facts = HashMap::new();
    let ordinals = HashMap::new();
    let settings = Settings::for_test();

    let sections = card_sections(
        rehearsal_cards::DeckInput {
            cards: &cards,
            facts: &facts,
            ordinals: &ordinals,
            our_side: &ours(),
        },
        &settings,
    );

    assert!(sections.accusation.is_empty());
    assert!(sections.watch_for.is_empty());
    assert_eq!(sections.gap, settings.fact_card_wording.no_cards_notice);
}

// ─── The proof under each point ──────────────────────────────────────────────

/// A card that backs point 2 appears under 2, with its code and its quote.
#[test]
fn a_proof_lands_under_its_point_with_its_code() {
    let mut c = card("n1");
    c.backs_position = Some(2);
    let cards = index(vec![("n1".to_string(), c)]);
    let facts = index(vec![(
        "n1".to_string(),
        fact("n1", Some("Judge Tighe"), Some("2012-04-12")),
    )]);
    let ordinals = index(vec![("n1".to_string(), 116)]);

    let proofs = point_proofs(
        rehearsal_cards::DeckInput {
            cards: &cards,
            facts: &facts,
            ordinals: &ordinals,
            our_side: &ours(),
        },
        &ordinals,
    );

    let under_two = proofs.get(&2).expect("point 2 has backing");
    assert_eq!(under_two.len(), 1);
    assert_eq!(under_two[0].code.as_deref(), Some("C-116"));
    assert_eq!(under_two[0].title.as_deref(), Some("title of n1"));
    assert_eq!(under_two[0].who.as_deref(), Some("Judge Tighe"));
    assert_eq!(under_two[0].when.as_deref(), Some("2012-04-12"));
    assert_eq!(under_two[0].quote.as_deref(), Some("the words of n1"));
    assert!(proofs.get(&1).is_none(), "point 1 has none");
}

/// An unnumbered proof still appears under its point, without a code.
#[test]
fn a_proof_with_no_ordinal_appears_without_a_code() {
    let mut c = card("n1");
    c.backs_position = Some(1);
    let cards = index(vec![("n1".to_string(), c)]);
    let facts = index(vec![("n1".to_string(), fact("n1", Some("Phillips"), None))]);
    let ordinals = HashMap::new();

    let proofs = point_proofs(
        rehearsal_cards::DeckInput {
            cards: &cards,
            facts: &facts,
            ordinals: &ordinals,
            our_side: &ours(),
        },
        &ordinals,
    );

    let under_one = proofs.get(&1).expect("point 1 has backing");
    assert!(under_one[0].code.is_none());
    assert_eq!(under_one[0].title.as_deref(), Some("title of n1"));
}
