//! Tests for `scenario_human_links` — task 2.10.
//!
//! The three things this module decides: which stored rows become links a card
//! can show, what sentence a linked card carries, and what the progress line
//! counts. Ruling R2 — no stance verb on a human-linked card — is fenced here and
//! again in `scenario_card_tests` against a whole card, because it is the ruling
//! most likely to be broken by a well-meaning later change.

use std::collections::HashMap;

use super::*;
use crate::domain::link_cut::LinkCut;
use crate::domain::wording::Wording;
use crate::dto::scenario_card::{CardConfidence, CardPinpoint, CardQuote, CardSpeaker};

fn wording() -> Wording {
    Wording::for_test()
}

fn labels() -> HashMap<String, String> {
    HashMap::from([
        (
            "alleg-41".to_string(),
            "¶41 — refused to divide the property amicably".to_string(),
        ),
        (
            "alleg-92".to_string(),
            "¶92 — failed to account for estate funds".to_string(),
        ),
    ])
}

fn row(node: &str, allegation: &str, cut: &str) -> EvidenceAllegationLinkRecord {
    EvidenceAllegationLinkRecord {
        graph_node_id: node.to_string(),
        allegation_id: allegation.to_string(),
        cut: cut.to_string(),
        authored_by: "roman".to_string(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

// ─── Resolving rows into links ───────────────────────────────────────────────

#[test]
fn a_stored_row_becomes_a_link_with_its_composed_label() {
    let out = resolve_links(
        vec![row("ev-1", "alleg-41", "against")],
        &labels(),
        &wording(),
    );

    let links = out.get("ev-1").expect("the node has links");
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].allegation_id, "alleg-41");
    assert_eq!(
        links[0].label,
        "¶41 — refused to divide the property amicably"
    );
    assert_eq!(links[0].cut, LinkCut::Against);
    // The chip reads as a LABEL, not as the middle of a sentence.
    assert_eq!(links[0].cut_label, "They'll use it against us");
}

#[test]
fn one_statement_can_bear_on_several_accusations() {
    // The lens model, which is why the panel uses checkboxes rather than a
    // dropdown: a statement genuinely bears on more than one thing.
    let out = resolve_links(
        vec![
            row("ev-1", "alleg-41", "against"),
            row("ev-1", "alleg-92", "supports"),
        ],
        &labels(),
        &wording(),
    );
    let links = out.get("ev-1").expect("the node has links");
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].cut, LinkCut::Against);
    assert_eq!(links[1].cut, LinkCut::Supports);
}

#[test]
fn links_are_grouped_by_the_statement_they_belong_to() {
    let out = resolve_links(
        vec![
            row("ev-1", "alleg-41", "against"),
            row("ev-2", "alleg-92", "supports"),
        ],
        &labels(),
        &wording(),
    );
    assert_eq!(out.get("ev-1").map(Vec::len), Some(1));
    assert_eq!(out.get("ev-2").map(Vec::len), Some(1));
    assert!(
        !out.contains_key("ev-3"),
        "a node with no rows has no entry"
    );
}

/// A cut token this build cannot read drops the link rather than guessing.
///
/// The worst possible guess here is "supports": it would tell a lawyer that a
/// statement the other side will wield is one of ours. Dropping keeps the card
/// defer-only, which is the honest state — nothing readable links it.
#[test]
fn an_unreadable_cut_drops_the_link_rather_than_defaulting() {
    let out = resolve_links(
        vec![row("ev-1", "alleg-41", "helpful")],
        &labels(),
        &wording(),
    );
    assert!(
        !out.contains_key("ev-1"),
        "an unreadable cut must not reach a card with a guessed reading"
    );
}

/// A link pointing at an accusation the graph no longer holds is dropped.
///
/// The 2026-07-24 orphaning, arriving here: a re-extraction changes the content
/// hashes and a stored link points at nothing. A chip reading the raw id would be
/// worse than no chip, and task 2.5 re-attaches by anchor.
#[test]
fn a_link_to_a_vanished_accusation_is_dropped() {
    let out = resolve_links(
        vec![row("ev-1", "alleg-gone", "against")],
        &labels(),
        &wording(),
    );
    assert!(!out.contains_key("ev-1"));
}

#[test]
fn one_bad_row_does_not_take_the_good_ones_with_it() {
    // Partial failure must degrade to "this one link is missing", not to "this
    // statement has no links" — the second would silently re-lock a card the human
    // had already unlocked.
    let out = resolve_links(
        vec![
            row("ev-1", "alleg-41", "against"),
            row("ev-1", "alleg-gone", "against"),
        ],
        &labels(),
        &wording(),
    );
    assert_eq!(out.get("ev-1").map(Vec::len), Some(1));
}

// ─── The sentence (ruling R2) ────────────────────────────────────────────────

#[test]
fn the_summary_says_what_the_human_did_in_their_terms() {
    let links = resolve_links(
        vec![row("ev-1", "alleg-41", "against")],
        &labels(),
        &wording(),
    );
    let summary = link_summary(links.get("ev-1").expect("links"), &wording()).expect("a sentence");

    assert_eq!(
        summary,
        "You linked this to ¶41 — refused to divide the property amicably · \
         they'll use it against us."
    );
}

/// RULING R2, AS A TEST: no stance verb is emitted for a human-linked card.
///
/// The extraction said nothing about this statement. A sentence in the machine's
/// voice — "This supports ¶41" — would be the page claiming a finding nobody made.
/// Asserted against the canon verbs themselves, so a fifth verb added to
/// `stance_verb_for_edge` later is covered without anyone remembering this test.
#[test]
fn no_stance_verb_reaches_a_human_linked_card() {
    let links = resolve_links(
        vec![
            row("ev-1", "alleg-41", "against"),
            row("ev-1", "alleg-92", "supports"),
        ],
        &labels(),
        &wording(),
    );
    let summary = link_summary(links.get("ev-1").expect("links"), &wording()).expect("a sentence");

    for edge in crate::domain::case_state::partition::ConnectionTier::Topical.edge_types() {
        let Some(verb) = crate::domain::card_language::stance_verb_for_edge(edge) else {
            continue;
        };
        assert!(
            !summary.contains(&format!("This {verb}")),
            "the human-link sentence must not speak in the machine's voice \
             (found '{verb}'): {summary}"
        );
    }
    // And it says who did it, which is the whole point of the wording.
    assert!(summary.starts_with("You linked"), "{summary}");
}

#[test]
fn a_card_with_no_human_links_has_no_summary() {
    // The §7.5 slot must hold exactly one of three things. `None` here is what
    // leaves it to the stance or the defer reason.
    assert!(link_summary(&[], &wording()).is_none());
}

#[test]
fn every_accusation_appears_in_the_sentence() {
    // Listing one and dropping the rest would understate what the human said the
    // statement does — the same loss `CardBearsOn.elements` exists to prevent.
    let links = resolve_links(
        vec![
            row("ev-1", "alleg-41", "against"),
            row("ev-1", "alleg-92", "against"),
        ],
        &labels(),
        &wording(),
    );
    let summary = link_summary(links.get("ev-1").expect("links"), &wording()).expect("a sentence");
    assert!(summary.contains("¶41"), "{summary}");
    assert!(summary.contains("¶92"), "{summary}");
}

#[test]
fn the_sentence_is_composed_from_the_stored_template() {
    // R4, as a test: change the stored words and the card changes with them, with
    // no rebuild. If someone replaced the template with a `format!`, this fails.
    let mut custom = wording();
    custom.link_summary_template = "Bears on {allegations} ({cut}).".to_string();
    custom.link_cut_against_phrase = "a hazard".to_string();

    let links = resolve_links(vec![row("ev-1", "alleg-41", "against")], &labels(), &custom);
    let summary = link_summary(links.get("ev-1").expect("links"), &custom).expect("a sentence");

    assert_eq!(
        summary,
        "Bears on ¶41 — refused to divide the property amicably (a hazard)."
    );
}

// ─── The progress line ───────────────────────────────────────────────────────

/// A card with no machine stance — the stuck class, built for the progress tests.
///
/// Constructed as a literal rather than through `build_card`: `link_progress`
/// reads exactly two fields, and routing through the full builder would make
/// these tests fail for reasons that have nothing to do with counting.
fn stuck_card(node: &str, links: Vec<CardHumanLink>) -> ScenarioCard {
    ScenarioCard {
        code: None,
        graph_node_id: node.to_string(),
        // Task 2.13: link_progress reads neither, so the honest fixture is an
        // unruled candidate — no weight, no place.
        tier: None,
        sort_ordinal: None,
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
            document_id: "doc-7".to_string(),
            document_title: "CFS responses".to_string(),
            label: "CFS responses at 14".to_string(),
            page: Some(14),
            viewer_href: "/documents/doc-7?page=14&tab=document".to_string(),
        },
        speaker: CardSpeaker {
            name: None,
            attribution: "extracted".to_string(),
        },
        statement_kind: None,
        // THE stuck condition: the extraction linked this to nothing.
        stance: None,
        bears_on: Vec::new(),
        grounding: None,
        confidence: CardConfidence {
            band: crate::domain::confidence_band::ConfidenceBand::Unscored,
            label: "Not scored".to_string(),
        },
        status: crate::domain::fact_status::FactStatus::Undecided,
        status_label: "Not yet decided".to_string(),
        defer_required: links.is_empty(),
        defer_required_reason: None,
        defer_reason: None,
        human_links: links,
        human_link_summary: None,
    }
}

/// The same card, after a human linked it.
fn linked(node: &str) -> ScenarioCard {
    let resolved = resolve_links(
        vec![row(node, "alleg-41", "against")],
        &labels(),
        &wording(),
    );
    stuck_card(node, resolved.get(node).cloned().unwrap_or_default())
}

/// A card the MACHINE linked — it was never stuck.
fn machine_linked(node: &str) -> ScenarioCard {
    let mut card = stuck_card(node, Vec::new());
    card.stance = Some(crate::dto::scenario_card::CardStance {
        verb: "supports".to_string(),
        object: "¶41".to_string(),
        summary: "This supports ¶41".to_string(),
    });
    card.defer_required = false;
    card
}

#[test]
fn the_progress_line_counts_the_stuck_pile_and_how_much_is_cleared() {
    let cards = [stuck_card("ev-1", Vec::new()), linked("ev-2")];
    let line = link_progress(cards.iter(), &wording()).expect("a line");
    assert_eq!(line, "1 of 2 linked.");
}

/// A card the MACHINE linked was never stuck, and is in neither number.
///
/// The denominator is the pile this feature exists to clear. Counting every card
/// would make the number read as triage progress, which the queue already reports
/// separately — two numbers claiming to be the same thing.
#[test]
fn a_machine_linked_card_is_not_part_of_the_stuck_pile() {
    let cards = [stuck_card("ev-1", Vec::new()), machine_linked("ev-3")];
    let line = link_progress(cards.iter(), &wording()).expect("a line");
    assert_eq!(line, "0 of 1 linked.");
}

#[test]
fn nothing_stuck_means_no_progress_line_at_all() {
    // A sentence about an empty problem is noise on a screen that should be quiet.
    assert!(link_progress([machine_linked("ev-3")].iter(), &wording()).is_none());
    assert!(link_progress([].iter(), &wording()).is_none());
}

#[test]
fn the_progress_line_is_composed_from_the_stored_template() {
    let mut custom = wording();
    custom.link_progress_template = "{linked}/{total} done".to_string();
    let cards = [stuck_card("ev-1", Vec::new()), linked("ev-2")];
    assert_eq!(
        link_progress(cards.iter(), &custom).expect("a line"),
        "1/2 done"
    );
}
