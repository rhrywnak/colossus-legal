// Tests for `services::rehearsal_cards` — her points, and what to watch for.
//
// Split from `rehearsal_cards_tests.rs` for the module-size limit (Rule 17).
// Both halves build on the fixtures in `rehearsal_cards_fixtures.rs`.

use super::deck_fixtures::*;
use super::*;

// ─── Her Points — the 3.9 pairing ────────────────────────────────────────────

/// A card backing point 2 appears under point 2.
#[test]
fn a_card_appears_under_the_point_it_backs() {
    let mut c = card("n1");
    c.backs_position = Some(2);
    let cards = index(vec![("n1".to_string(), c)]);
    let facts = index(vec![("n1".to_string(), fact("n1", Some("Phillips"), None))]);
    let ordinals = HashMap::new();
    let ours = ours_list();

    let backing = point_backing(deck(&cards, &facts, &ordinals, &ours));
    assert_eq!(backing.len(), 1);
    assert_eq!(backing[&2].len(), 1);
    assert_eq!(backing[&2][0].quote.as_deref(), Some("the words of n1"));
}

/// A card backing NO point appears under none.
#[test]
fn a_card_backing_no_point_appears_nowhere() {
    let cards = index(vec![("n1".to_string(), card("n1"))]);
    let facts = index(vec![("n1".to_string(), fact("n1", Some("Phillips"), None))]);
    let ordinals = HashMap::new();
    let ours = ours_list();
    assert!(point_backing(deck(&cards, &facts, &ordinals, &ours)).is_empty());
}

/// Several cards under one point read oldest first.
#[test]
fn several_cards_under_one_point_read_oldest_first() {
    let mut late = card("late");
    late.backs_position = Some(1);
    let mut early = card("early");
    early.backs_position = Some(1);
    let cards = index(vec![
        ("late".to_string(), late),
        ("early".to_string(), early),
    ]);
    let facts = index(vec![
        (
            "late".to_string(),
            fact("late", Some("P"), Some("2014-07-10")),
        ),
        (
            "early".to_string(),
            fact("early", Some("P"), Some("2009-11-05")),
        ),
    ]);
    let ordinals = HashMap::new();
    let ours = ours_list();

    let backing = point_backing(deck(&cards, &facts, &ordinals, &ours));
    let order: Vec<&str> = backing[&1]
        .iter()
        .map(|p| p.graph_node_id.as_str())
        .collect();
    assert_eq!(order, vec!["early", "late"]);
}

/// OUR OWN statements back her points too.
///
/// The Accusation section is the other side's; a point is backed by whatever
/// proves it, and her own certified letter is exactly that. A filter here would
/// have quietly removed her best evidence from the section that needs it.
#[test]
fn our_own_statements_can_back_a_point() {
    let mut c = card("n1");
    c.backs_position = Some(1);
    let cards = index(vec![("n1".to_string(), c)]);
    let facts = index(vec![(
        "n1".to_string(),
        fact("n1", Some("Marie Awad"), None),
    )]);
    let ordinals = HashMap::new();
    let ours = ours_list();
    assert_eq!(
        point_backing(deck(&cards, &facts, &ordinals, &ours))[&1].len(),
        1
    );
}

// ─── What to Watch For ───────────────────────────────────────────────────────

/// Every card's warning appears, with its C-code.
#[test]
fn a_watch_out_appears_with_its_code() {
    let mut c = card("n1");
    c.watch_out = Some("They will say the judge approved it.".to_string());
    let cards = index(vec![("n1".to_string(), c)]);
    let facts = HashMap::new();
    let ordinals = index(vec![("n1".to_string(), 116)]);
    let ours = ours_list();

    let rows = watch_outs(deck(&cards, &facts, &ordinals, &ours));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].code.as_deref(), Some("C-116"));
    assert_eq!(rows[0].text, "They will say the judge approved it.");
}

/// A card with NO warning contributes nothing — an empty row would be a warning
/// about nothing on the section whose job is to say what to expect.
#[test]
fn a_card_with_no_watch_out_contributes_nothing() {
    let cards = index(vec![("n1".to_string(), card("n1"))]);
    let facts = HashMap::new();
    let ordinals = HashMap::new();
    let ours = ours_list();
    assert!(watch_outs(deck(&cards, &facts, &ordinals, &ours)).is_empty());
}

/// Warnings read in C-code order, with un-numbered cards LAST.
///
/// The same rule the card list uses for an un-numbered candidate, rather than a
/// second convention for the same question.
#[test]
fn watch_outs_read_in_code_order_with_unnumbered_last() {
    let mut first = card("first");
    first.watch_out = Some("one".to_string());
    let mut second = card("second");
    second.watch_out = Some("two".to_string());
    let mut unnumbered = card("unnumbered");
    unnumbered.watch_out = Some("three".to_string());

    let cards = index(vec![
        ("second".to_string(), second),
        ("unnumbered".to_string(), unnumbered),
        ("first".to_string(), first),
    ]);
    let facts = HashMap::new();
    let ordinals = index(vec![("first".to_string(), 4), ("second".to_string(), 91)]);
    let ours = ours_list();

    let codes: Vec<Option<String>> = watch_outs(deck(&cards, &facts, &ordinals, &ours))
        .into_iter()
        .map(|w| w.code)
        .collect();
    assert_eq!(
        codes,
        vec![Some("C-4".to_string()), Some("C-91".to_string()), None]
    );
}

/// A blank warning is the same as none.
#[test]
fn a_blank_watch_out_contributes_nothing() {
    let mut c = card("n1");
    c.watch_out = Some("  \n ".to_string());
    let cards = index(vec![("n1".to_string(), c)]);
    let facts = HashMap::new();
    let ordinals = HashMap::new();
    let ours = ours_list();
    assert!(watch_outs(deck(&cards, &facts, &ordinals, &ours)).is_empty());
}

/// An empty deck produces three empty views, and panics on none of them.
#[test]
fn an_empty_deck_produces_three_empty_views() {
    let cards = HashMap::new();
    let facts = HashMap::new();
    let ordinals = HashMap::new();
    let ours = ours_list();
    let input = deck(&cards, &facts, &ordinals, &ours);
    assert!(accusation_cards(input).is_empty());
    assert!(point_backing(input).is_empty());
    assert!(watch_outs(input).is_empty());
}
