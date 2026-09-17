// Tests for the deck payload's TACTIC CARD LIST — `services::practice_page`.
//
// A module of its own rather than more lines in `practice_page_tests`, which is
// already past the 300-line limit (Rule 17) and which this task has no business
// pushing further. The seam is honest: everything here is about ONE list and the
// round trip between a card's number and its name.
//
// `super::tests::settings()` is the sibling module's fixture — `pub(super)`, so
// both test modules under `practice_page` share one Settings snapshot rather
// than building two that could drift.

use super::tests::settings;
use super::*;

/// The dropdown's options ARE the vocabulary the pills are resolved against.
///
/// ## ⚑ The off-by-one is the whole risk, again
///
/// `tactic_name` indexes `tactic_names[card - 1]`; this pairs each name with
/// `index + 1`. If the two ever disagreed, the editor would offer "compound" and
/// store the card the pill calls something else — and both screens would render
/// perfectly. So this asserts the ROUND TRIP: every option's number, fed back
/// through `tactic_name`, must name that option.
#[test]
fn every_offered_card_resolves_back_to_its_own_name() {
    let s = settings();
    let payload = deck_payload(
        &s,
        DeckSources {
            scenario_id: Uuid::nil(),
            code: "S-6".to_string(),
            title: "Too many attorneys".to_string(),
            deck: vec![],
            points: vec![],
            receipts: &[],
            last: None,
            current: &[],
            open: None,
            attach_options: vec![],
            notes: &[],
            review: Default::default(),
        },
    );

    assert_eq!(
        payload.tactic_cards.len(),
        s.practice_read.tactic_names.len(),
        "the form must offer every card the vocabulary can name, and no other"
    );
    assert!(
        !payload.tactic_cards.is_empty(),
        "ANTI-VACUITY: an empty list would satisfy the round trip below forever"
    );

    for card in &payload.tactic_cards {
        assert_eq!(
            tactic_name(&s, Some(card.card)).as_deref(),
            Some(card.name.as_str()),
            "card {} is offered as {:?} but resolves to {:?}",
            card.card,
            card.name,
            tactic_name(&s, Some(card.card))
        );
    }

    // The numbering starts at ONE, because the column's CHECK is `BETWEEN 1 AND
    // 7`. A zero-based list would offer a card the database refuses.
    assert_eq!(payload.tactic_cards[0].card, 1);
    assert_eq!(
        payload.tactic_cards[0].name,
        s.practice_read.tactic_names[0]
    );
    // And no braid suffix ever reaches the form: a suffix is composed for a TAG.
    for card in &payload.tactic_cards {
        assert!(
            !card.name.contains(&s.practice_wording.tactic_braid_suffix),
            "the dropdown is offering a tag, not a card: {:?}",
            card.name
        );
    }
}
