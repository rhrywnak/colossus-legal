//! Tests for the card label — the heading a human reads on every block.

use super::*;

fn card(c_number: Option<i32>) -> Card {
    Card {
        evidence_id: "doc-phillips-admissions:evidence:af3fbc2d".to_string(),
        c_number,
        ..Card::default()
    }
}

/// ⚑ A numbered card shows its C-number; an unnumbered one shows its id.
///
/// Not a cosmetic choice. Under the widening MOST cards in a list are outside
/// the scenario's numbered pool — S-11's top sixty carried 14 numbered and 46
/// not — so the fallback is the common path, and a blank heading there would
/// leave the majority of the page unciteable.
#[test]
fn a_numbered_card_shows_its_c_number_and_an_unnumbered_one_shows_its_id() {
    assert_eq!(card_label(&card(Some(54))), "C-54");
    assert_eq!(card_label(&card(Some(1))), "C-1");
    assert_eq!(
        card_label(&card(None)),
        "doc-phillips-admissions:evidence:af3fbc2d",
        "the id is the fallback, never a blank — an unnumbered card must stay findable"
    );
}

/// The label is never empty, whatever the card carries.
#[test]
fn the_label_is_never_blank() {
    for c_number in [None, Some(0), Some(292)] {
        assert!(!card_label(&card(c_number)).is_empty());
    }
    // Even a card with no id at all produces something, rather than a heading
    // that reads `### 7. ` and sends the reader looking for a missing word.
    let empty = Card::default();
    assert_eq!(card_label(&empty), "");
}
