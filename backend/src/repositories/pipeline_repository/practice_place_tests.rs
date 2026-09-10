//! The WHOLE-DECK placement rules, asserted without a database.
//!
//! A sibling of `practice_reorder_tests.rs` rather than more of it. That file
//! pins what a drag MEANS within one side — the rule a person notices when it is
//! wrong — and every assertion in it predates tonight. These pin the two things
//! the 2026-09-10 save fix added, which are a different subject:
//!
//! * the returned order is a PERMUTATION OF THE DECK, so `write_order` can
//!   renumber `0..N-1` without colliding with the rows it did not touch;
//! * the other side, and the hidden rows, come back in the slots they went in at.
//!
//! ## Why the fixture is INTERLEAVED
//!
//! Because DEV is. Read read-only on 2026-09-10, scenario S-1 holds 32 George
//! rows and 10 of Chuck's woven through `sort_order` 1..42 — George at 1-5,
//! Chuck at 6-9, a hidden George row at 10, Chuck at 11-16, and so on. A fixture
//! with one side's rows in a contiguous block would pass with a rule that
//! appended the other side wholesale, which is the wrong rule and the one this
//! module exists to forbid.

use uuid::Uuid;

use super::{placed_after, position_within_side, resequenced, NewPosition};
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
        updated_at: chrono::DateTime::from_timestamp(1_755_000_000, 0)
            .expect("a fixed, valid instant"),
    }
}

/// An INTERLEAVED deck, in `sort_order`: g1 g2 · c3 c4 · g5 · c6 · g7.
///
/// Four George rows and three of Chuck's, in the shape DEV actually holds. The
/// numbers are the slot indexes plus one, so an expectation reads as positions.
fn deck() -> Vec<PracticeQuestionRecord> {
    vec![
        question(1, "george"),
        question(2, "george"),
        question(3, "chuck"),
        question(4, "chuck"),
        question(5, "george"),
        question(6, "chuck"),
        question(7, "george"),
    ]
}

fn ids(ns: &[u128]) -> Vec<Uuid> {
    ns.iter().map(|n| Uuid::from_u128(*n)).collect()
}

fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

// ─── The permutation property, which is what stops the 500 ───────────────────

/// Every id, exactly once. The property `write_order` depends on.
///
/// This is the test that would have caught the bug. `write_order` renumbers the
/// ids it is handed to `0..N-1`; `practice_questions_order_unique` spans the
/// scenario; so anything short of the whole deck leaves rows holding numbers the
/// second phase is about to hand out, and Postgres rolls the transaction back.
/// The count and the uniqueness together say "nothing was left over".
#[test]
fn a_two_sided_drag_returns_the_whole_deck_exactly_once_each() {
    let out = resequenced(&deck(), id(7), Some(id(1))).expect("a real move names a position");

    assert_eq!(out.len(), 7, "every row of the deck comes back: {out:?}");
    let mut sorted = out.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), 7, "and no id appears twice: {out:?}");
    for n in 1..=7u128 {
        assert!(out.contains(&id(n)), "id {n} went missing: {out:?}");
    }
}

/// The other side does not move — slot for slot, not merely in relative order.
///
/// Chuck's rows sit at slots 2, 3 and 5 of this deck. After a George drag they
/// must still be at slots 2, 3 and 5. The stronger claim is deliberate: a rule
/// that only preserved their relative ORDER could still shunt all three to the
/// end, silently rewriting the position of every row for a gesture that touched
/// none of them.
#[test]
fn the_other_side_keeps_its_slots_not_just_its_order() {
    let out = resequenced(&deck(), id(7), Some(id(1))).expect("a real move names a position");

    assert_eq!(out[2], id(3), "Chuck's first row left slot 2: {out:?}");
    assert_eq!(out[3], id(4), "Chuck's second row left slot 3: {out:?}");
    assert_eq!(out[5], id(6), "Chuck's third row left slot 5: {out:?}");
}

/// The dragged row lands where it was asked, among its OWN side's slots.
///
/// George holds slots 0, 1, 4 and 6. Dragging g7 onto g1 makes George's order
/// `7 1 2 5`, and those four ids fill those four slots in that sequence — so the
/// whole-deck answer is `7 1 · 3 4 · 2 · 6 · 5`.
#[test]
fn the_dragged_row_lands_where_it_was_asked() {
    let out = resequenced(&deck(), id(7), Some(id(1))).expect("a real move names a position");
    assert_eq!(out, ids(&[7, 1, 3, 4, 2, 6, 5]));
}

/// A hidden row is in the order, in its own slot, and is never dropped.
///
/// `list_deck` returns hidden rows (they are skipped by the SCREEN, not by the
/// store), so they hold `sort_order` values like any other row. Leaving one out
/// of the returned order would leave it holding a number the renumber is about to
/// reuse — the same collision, from the other direction. A hidden row on the
/// dragged side moves with its side, exactly as it did before tonight.
#[test]
fn a_hidden_row_keeps_its_place_in_the_order() {
    let mut deck = deck();
    deck[3].hidden_at =
        Some(chrono::DateTime::from_timestamp(1_755_000_100, 0).expect("a fixed, valid instant"));

    let out = resequenced(&deck, id(7), Some(id(1))).expect("a real move names a position");
    assert_eq!(out.len(), 7, "the hidden row is still in the deck: {out:?}");
    assert_eq!(out[3], id(4), "and still in its own slot: {out:?}");
}

/// A ONE-SIDED deck answers exactly as it did before the fix.
///
/// The case that never had the bug: with only one side, "this side" and "the
/// whole deck" are the same list, so the returned order is unchanged. Pinned so
/// the fix is visibly a widening rather than a re-definition.
#[test]
fn a_one_sided_deck_is_unchanged_by_the_whole_deck_rule() {
    let deck: Vec<PracticeQuestionRecord> = (1..=4).map(|n| question(n, "george")).collect();

    assert_eq!(
        resequenced(&deck, id(4), Some(id(1))).expect("a real move"),
        ids(&[4, 1, 2, 3])
    );
    assert_eq!(
        resequenced(&deck, id(2), None).expect("dropping past the end is a move"),
        ids(&[1, 3, 4, 2])
    );
}

// ─── The change log's number ─────────────────────────────────────────────────

/// The logged position is the one a person can SEE — within the side.
///
/// g7 dragged to the top of George's side is George's first question, and the
/// change log has always said so. It is now the twentieth-something entry of a
/// whole-deck order, and reporting THAT number would put a figure in Chuck's
/// "Changed since your last sitting" box that matches nothing on his screen.
#[test]
fn the_logged_position_counts_within_the_side() {
    let out = resequenced(&deck(), id(7), Some(id(1))).expect("a real move names a position");

    // Slot 0 of the whole deck, and also George's first — they agree here.
    assert_eq!(
        position_within_side(&deck(), &out, id(7), "george"),
        Some(0)
    );
    // Chuck's last row is at whole-deck slot 5 and is Chuck's THIRD question.
    assert_eq!(position_within_side(&deck(), &out, id(6), "chuck"), Some(2));
    // g5 is at whole-deck slot 6 and is George's fourth.
    assert_eq!(
        position_within_side(&deck(), &out, id(5), "george"),
        Some(3)
    );
}

/// An id the order does not hold names no position, and does not default to 0.
///
/// Standing Rule 1. `commit_order` treats this as the broken invariant it is; a
/// `.unwrap_or(0)` there would write "moved to the top" into the record of a
/// question that did not move at all.
#[test]
fn a_position_for_an_absent_id_is_none_and_not_zero() {
    let out = resequenced(&deck(), id(7), Some(id(1))).expect("a real move");
    assert_eq!(position_within_side(&deck(), &out, id(404), "george"), None);
}

// ─── Adding a question where you are ─────────────────────────────────────────

/// A new question lands immediately BELOW the row it names, on its own side.
///
/// The new row was inserted at `next_sort_order`, so it starts in the deck's last
/// slot — slot 7 here. Asking for it after g1 makes George's order `1 99 2 5 7`,
/// which fills George's five slots (0, 1, 4, 6 and the new last one) in that
/// sequence: `1 99 · 3 4 · 2 · 6 · 5 7`.
#[test]
fn add_after_lands_immediately_below_that_row() {
    let out = placed_after(&deck(), id(99), "george", NewPosition::After(id(1)))
        .expect("a same-side after");
    assert_eq!(out, ids(&[1, 99, 3, 4, 2, 6, 5, 7]));

    // The same claim without the slot arithmetic, because that is the sentence a
    // person would use: on George's side, the new row is the one straight after
    // g1. `position_within_side` reads the order the way the change log does.
    assert_eq!(
        position_within_side(&deck(), &out, id(1), "george"),
        Some(0),
        "g1 is still George's first"
    );
    assert_eq!(
        position_within_side(&deck(), &out, id(99), "george"),
        Some(1),
        "and the new question is George's second — immediately below it"
    );
}

/// `after = None` appends — which is what the add route has always done.
///
/// The new row is already last in the deck when it arrives here, so the answer is
/// the deck unchanged with the new id on the end. Pinned because it is the path
/// every existing caller takes, and a regression in it would move every question
/// anybody adds from the bottom box.
#[test]
fn add_without_after_still_appends() {
    let out = placed_after(&deck(), id(99), "george", NewPosition::End)
        .expect("no after is a real answer");
    assert_eq!(out, ids(&[1, 2, 3, 4, 5, 6, 7, 99]));
}

/// After the LAST row of a side means the end of that side.
///
/// g7 is George's last question but not the deck's last row — Chuck's c6 is
/// ahead of it in slots and the new row is behind it. "After g7" therefore means
/// the new row keeps the end-of-side position it arrived with.
#[test]
fn add_after_the_last_row_of_a_side_is_the_end_of_that_side() {
    let out = placed_after(&deck(), id(99), "george", NewPosition::After(id(7)))
        .expect("a same-side after");
    assert_eq!(out, ids(&[1, 2, 3, 4, 5, 6, 7, 99]));
}

/// `after` on the OTHER side names no position here.
///
/// The route answers this one with a 400 that names the field — placing a Chuck
/// question below a George row is not a thing the deck can express — and this is
/// the pure half of that refusal. Distinct from a row the deck does not hold at
/// all, which the route answers 404; both arrive here as `None` because the
/// difference is the caller's to tell, not this function's.
#[test]
fn add_after_a_row_on_the_other_side_names_no_position() {
    assert_eq!(
        placed_after(&deck(), id(99), "george", NewPosition::After(id(3))),
        None
    );
    assert_eq!(
        placed_after(&deck(), id(99), "chuck", NewPosition::After(id(1))),
        None
    );
}

/// `after` naming a row this deck does not hold names no position.
#[test]
fn add_after_an_unknown_row_names_no_position() {
    assert_eq!(
        placed_after(&deck(), id(99), "george", NewPosition::After(id(404))),
        None
    );
}

/// The add's answer is a permutation too — deck plus one, each exactly once.
#[test]
fn add_returns_the_whole_deck_plus_the_new_row() {
    let out = placed_after(&deck(), id(99), "chuck", NewPosition::After(id(3)))
        .expect("a same-side after");
    assert_eq!(out.len(), 8);
    let mut sorted = out.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), 8, "no id twice: {out:?}");
    assert!(out.contains(&id(99)), "the new row is in it: {out:?}");
    // Chuck's slots are 2, 3 and 5, plus the new last one. Chuck's order becomes
    // `3 99 4 6`, so slot 3 takes the new row.
    assert_eq!(out, ids(&[1, 2, 3, 99, 5, 4, 7, 6]));
}

/// The TOP of a side, which `after` alone cannot name.
///
/// George holds slots 0, 1, 4 and 6, plus the new last one. Asking for the start
/// makes George's order `99 1 2 5 7`, filling those five slots in sequence.
#[test]
fn at_start_puts_the_new_row_at_the_top_of_its_side() {
    let out = placed_after(&deck(), id(99), "george", NewPosition::Start).expect("the top");
    assert_eq!(out, ids(&[99, 1, 3, 4, 2, 6, 5, 7]));
    assert_eq!(
        position_within_side(&deck(), &out, id(99), "george"),
        Some(0)
    );
    // And Chuck's rows never moved, exactly as for a drag.
    assert_eq!((out[2], out[3], out[5]), (id(3), id(4), id(6)));
}

/// The top of an EMPTY side is the same place as the end of it.
///
/// The first question ever added to Chuck's half of a George-only deck. Both
/// positions mean "the only row", and neither may fail — a fence that refused
/// this would make an empty side impossible to start.
#[test]
fn at_start_on_a_side_with_no_rows_yet_is_the_only_row() {
    let one_sided: Vec<PracticeQuestionRecord> = (1..=3).map(|n| question(n, "george")).collect();

    let start = placed_after(&one_sided, id(99), "chuck", NewPosition::Start)
        .expect("the first row of a side is a real position");
    let end = placed_after(&one_sided, id(99), "chuck", NewPosition::End)
        .expect("and so is the end of it");
    assert_eq!(start, end, "on an empty side the two are the same place");
    assert_eq!(start, ids(&[1, 2, 3, 99]));
}
