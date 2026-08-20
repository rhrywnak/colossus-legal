//! The ordering rule, asserted without a database.
//!
//! `resequenced` is pure precisely so this is possible: the rule that decides
//! where a dropped question lands is the part a person notices when it is wrong,
//! and it should not need a Postgres connection to pin.

use uuid::Uuid;

use super::resequenced;
use crate::repositories::pipeline_repository::practice::PracticeQuestionRecord;

/// One question on a named side, numbered so the tests read as positions.
fn question(n: u128, side: &str) -> PracticeQuestionRecord {
    PracticeQuestionRecord {
        id: Uuid::from_u128(n),
        scenario_id: Uuid::nil(),
        side: side.to_string(),
        text: format!("question {n}"),
        tactic: None,
        braid_rows: None,
        source_kind: "manual".to_string(),
        source_ref: None,
        receipt: None,
        watch_for: None,
        stronger: None,
        stronger_lean: None,
        pair_said: None,
        pair_admitted: None,
        sort_order: i32::try_from(n).unwrap_or(1),
        flag_note: None,
        deck_key: None,
        kind: "cross".to_string(),
        follows_key: None,
        source_line: None,
        hidden_at: None,
        draft_by: None,
    }
}

/// Five George questions, 1..=5, plus two of Chuck's to prove the side filter.
fn deck() -> Vec<PracticeQuestionRecord> {
    let mut out: Vec<PracticeQuestionRecord> = (1..=5).map(|n| question(n, "george")).collect();
    out.push(question(90, "chuck"));
    out.push(question(91, "chuck"));
    out
}

fn ids(ns: &[u128]) -> Vec<Uuid> {
    ns.iter().map(|n| Uuid::from_u128(*n)).collect()
}

/// Dragging the last question onto the first puts it FIRST.
#[test]
fn a_question_dropped_on_the_first_lands_first() {
    let out = resequenced(&deck(), Uuid::from_u128(5), Some(Uuid::from_u128(1)))
        .expect("a real move names a position");
    assert_eq!(out, ids(&[5, 1, 2, 3, 4]));
}

/// Dragging the first onto the last puts it immediately ABOVE the last.
///
/// "Onto" means "where that row is", so the target is pushed down — it does not
/// mean "after". A drop past the end is the separate case below.
#[test]
fn a_question_dropped_on_another_takes_that_ones_place() {
    let out = resequenced(&deck(), Uuid::from_u128(1), Some(Uuid::from_u128(5)))
        .expect("a real move names a position");
    assert_eq!(out, ids(&[2, 3, 4, 1, 5]));
}

/// A drop past the final row means LAST, and `None` is how that arrives.
#[test]
fn no_target_means_the_end_of_the_side() {
    let out =
        resequenced(&deck(), Uuid::from_u128(2), None).expect("dropping past the end is a move");
    assert_eq!(out, ids(&[1, 3, 4, 5, 2]));
}

/// Dropping a row onto the one directly BELOW it changes nothing — by design.
///
/// "Drop onto Y" means "land immediately above Y". A row is already immediately
/// above its own successor, so the gesture asks for the arrangement that already
/// exists. It returns a valid order that happens to equal the current one rather
/// than `None`: nothing is wrong, and the caller writing it back is a no-op the
/// database absorbs.
///
/// Pinned because it is the case a reader is most likely to mistake for a bug —
/// and because the scenario-facts drag has exactly the same property, so a
/// "fix" here would make the two surfaces disagree about what a drag means.
/// Moving DOWN past one row is done by dropping onto the row after it, which the
/// assertion below shows.
#[test]
fn a_drop_onto_the_next_row_down_asks_for_the_order_it_already_has() {
    let out = resequenced(&deck(), Uuid::from_u128(2), Some(Uuid::from_u128(3)))
        .expect("the gesture is legal, it simply asks for no change");
    assert_eq!(out, ids(&[1, 2, 3, 4, 5]));

    // Down past one row: drop onto the row AFTER the one being passed.
    let moved = resequenced(&deck(), Uuid::from_u128(2), Some(Uuid::from_u128(4)))
        .expect("a real move names a position");
    assert_eq!(moved, ids(&[1, 3, 2, 4, 5]));
}

/// A drop onto itself names no position, and is not an error.
///
/// A person does this constantly by accident — starting a drag and changing
/// their mind. Answering it with a 400 would put a red notice on screen for it.
#[test]
fn a_drop_onto_itself_is_nothing_to_do_and_not_a_failure() {
    assert_eq!(
        resequenced(&deck(), Uuid::from_u128(3), Some(Uuid::from_u128(3))),
        None
    );
}

/// The other side is not in the returned order, and cannot be a target.
///
/// Domain note: George's questions and Chuck's are two ordered lists sharing a
/// table. Dragging a cross question in among the directs would produce a deck
/// that deals a Chuck question in a George sitting — a different question, not a
/// re-ordered one.
#[test]
fn a_target_on_the_other_side_is_refused_and_the_other_side_never_moves() {
    assert_eq!(
        resequenced(&deck(), Uuid::from_u128(1), Some(Uuid::from_u128(90))),
        None
    );

    let out = resequenced(&deck(), Uuid::from_u128(1), Some(Uuid::from_u128(3)))
        .expect("a same-side move is fine");
    assert!(
        !out.contains(&Uuid::from_u128(90)) && !out.contains(&Uuid::from_u128(91)),
        "Chuck's questions must not appear in George's re-sequence: {out:?}"
    );
    assert_eq!(out.len(), 5, "only this side is re-sequenced");
}

/// A question the deck does not hold names no position.
#[test]
fn an_unknown_question_names_no_position() {
    assert_eq!(
        resequenced(&deck(), Uuid::from_u128(404), Some(Uuid::from_u128(1))),
        None
    );
    assert_eq!(
        resequenced(&deck(), Uuid::from_u128(1), Some(Uuid::from_u128(404))),
        None
    );
}
