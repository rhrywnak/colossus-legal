// Tests for `services::rehearsal_cards`.
//
// Three read-only views over one set of cards. What is asserted hardest is the
// definition of "the other side", because it is defined by ELIMINATION and the
// failure direction matters: an unlisted speaker must put an EXTRA card in front
// of Marie, never hide one she has to answer.

use super::deck_fixtures::*;
use super::*;

// ─── The Accusation ──────────────────────────────────────────────────────────

/// A statement by somebody who is not us is the other side's.
#[test]
fn a_statement_by_the_other_side_appears() {
    let cards = index(vec![("n1".to_string(), card("n1"))]);
    let facts = index(vec![(
        "n1".to_string(),
        fact("n1", Some("George Phillips"), None),
    )]);
    let ordinals = HashMap::new();
    let ours = ours_list();

    let rows = accusation_cards(deck(&cards, &facts, &ordinals, &ours));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].who.as_deref(), Some("George Phillips"));
}

/// OUR OWN statement does not.
#[test]
fn our_own_statement_is_not_an_accusation() {
    let cards = index(vec![("n1".to_string(), card("n1"))]);
    let facts = index(vec![(
        "n1".to_string(),
        fact("n1", Some("Marie Awad"), None),
    )]);
    let ordinals = HashMap::new();
    let ours = ours_list();

    assert!(accusation_cards(deck(&cards, &facts, &ordinals, &ours)).is_empty());
}

/// THE CASE FOLD. "Marie Awad" and "marie awad" are one person.
///
/// The stored list is lowercased by `token_list_of`, and the graph already holds
/// one man as two rows ("George Phillips" / "George R. Phillips"). A
/// case-sensitive test would have been a third way for that to happen.
#[test]
fn the_speaker_comparison_folds_case_and_whitespace() {
    for spelling in ["Marie Awad", "marie awad", "  MARIE AWAD  "] {
        let cards = index(vec![("n1".to_string(), card("n1"))]);
        let facts = index(vec![("n1".to_string(), fact("n1", Some(spelling), None))]);
        let ordinals = HashMap::new();
        let ours = ours_list();
        assert!(
            accusation_cards(deck(&cards, &facts, &ordinals, &ours)).is_empty(),
            "{spelling} is ours"
        );
    }
}

/// A statement with NO recorded speaker counts as the other side.
///
/// The safe direction: it shows an extra card she may not have to answer, rather
/// than hiding one she does. Documentary evidence genuinely has no speaker, and
/// on this case most of it is theirs.
#[test]
fn a_statement_with_no_speaker_counts_as_the_other_side() {
    let cards = index(vec![("n1".to_string(), card("n1"))]);
    let facts = index(vec![("n1".to_string(), fact("n1", None, None))]);
    let ordinals = HashMap::new();
    let ours = ours_list();
    assert_eq!(
        accusation_cards(deck(&cards, &facts, &ordinals, &ours)).len(),
        1
    );
}

/// A card whose statement is not in the fact map still appears.
///
/// A stale pointer is the 2026-07-24 defect class. Dropping the card would hide
/// that the deck names a statement nothing answers.
#[test]
fn a_card_whose_statement_is_missing_still_appears() {
    let cards = index(vec![("gone".to_string(), card("gone"))]);
    let facts = HashMap::new();
    let ordinals = HashMap::new();
    let ours = ours_list();
    let rows = accusation_cards(deck(&cards, &facts, &ordinals, &ours));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].quote, None);
}

/// CHRONOLOGICAL, oldest first, with undated statements LAST.
///
/// A missing date cannot be the oldest thing in a chronology, and an empty string
/// would sort before every real date.
#[test]
fn accusations_read_oldest_first_with_undated_last() {
    let cards = index(vec![
        ("late".to_string(), card("late")),
        ("early".to_string(), card("early")),
        ("undated".to_string(), card("undated")),
        ("blank".to_string(), card("blank")),
    ]);
    let facts = index(vec![
        (
            "late".to_string(),
            fact("late", Some("Phillips"), Some("2014-07-10")),
        ),
        (
            "early".to_string(),
            fact("early", Some("Phillips"), Some("2009-11-05")),
        ),
        (
            "undated".to_string(),
            fact("undated", Some("Phillips"), None),
        ),
        (
            "blank".to_string(),
            fact("blank", Some("Phillips"), Some("   ")),
        ),
    ]);
    let ordinals = HashMap::new();
    let ours = ours_list();

    let rows = accusation_cards(deck(&cards, &facts, &ordinals, &ours));
    let order: Vec<&str> = rows.iter().map(|r| r.graph_node_id.as_str()).collect();
    assert_eq!(order[0], "early");
    assert_eq!(order[1], "late");
    // The two undated ones follow, ordered by node id so two reads agree.
    assert_eq!(&order[2..], &["blank", "undated"]);
}

/// A card with NO answer is still shown, with `answer: None`.
///
/// The caller renders the stored gap sentence. An unanswered accusation is the
/// most important thing on this page, and hiding it would hide the work.
#[test]
fn an_unanswered_accusation_is_shown_with_no_answer() {
    let cards = index(vec![("n1".to_string(), card("n1"))]);
    let facts = index(vec![("n1".to_string(), fact("n1", Some("Phillips"), None))]);
    let ordinals = HashMap::new();
    let ours = ours_list();
    let rows = accusation_cards(deck(&cards, &facts, &ordinals, &ours));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].answer, None);
}

/// A blank answer is the same as none.
#[test]
fn a_blank_answer_reads_as_none() {
    let mut c = card("n1");
    c.answer = Some("   ".to_string());
    let cards = index(vec![("n1".to_string(), c)]);
    let facts = index(vec![("n1".to_string(), fact("n1", Some("Phillips"), None))]);
    let ordinals = HashMap::new();
    let ours = ours_list();
    assert_eq!(
        accusation_cards(deck(&cards, &facts, &ordinals, &ours))[0].answer,
        None
    );
}
