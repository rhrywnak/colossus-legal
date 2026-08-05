//! Placement arithmetic tests (task 2.13).
//!
//! Every assertion here is about a NUMBER the human never sees, backing an act
//! they perform constantly. The properties that matter are: one drag writes one
//! ordinal, the same drag always lands in the same place, and a drop that cannot
//! be honoured is refused by name rather than approximated.

use super::*;

/// A fact the human has placed at `ordinal`.
fn placed(id: &str, ordinal: i32) -> OrderableFact {
    OrderableFact {
        graph_node_id: id.to_string(),
        sort_ordinal: Some(ordinal),
        code_ordinal: None,
        tier: FactTier::Backup,
    }
}

/// A fact nobody has dragged, carrying its `C-n` number.
fn unplaced(id: &str, code: i32) -> OrderableFact {
    OrderableFact {
        graph_node_id: id.to_string(),
        sort_ordinal: None,
        code_ordinal: Some(code),
        tier: FactTier::Backup,
    }
}

#[test]
fn dropping_between_two_placed_facts_takes_the_midpoint() {
    let facts = vec![placed("a", 1024), placed("b", 2048), placed("c", 3072)];
    let ordinal = plan_move(&facts, "c", Some("a"), Some("b")).expect("a legal drop");
    assert_eq!(ordinal, 1536, "the midpoint of 1024 and 2048");
    assert!(
        ordinal > 1024 && ordinal < 2048,
        "it must land strictly between them"
    );
}

#[test]
fn dropping_at_the_top_goes_a_step_before_the_first() {
    let facts = vec![placed("a", 1024), placed("b", 2048)];
    let ordinal = plan_move(&facts, "b", None, Some("a")).expect("a legal drop");
    assert_eq!(ordinal, 0);
    assert!(
        ordinal < 1024,
        "it must sort ahead of the fact it was dropped above"
    );
}

#[test]
fn dropping_at_the_bottom_goes_a_step_past_the_last() {
    let facts = vec![placed("a", 1024), placed("b", 2048)];
    let ordinal = plan_move(&facts, "a", Some("b"), None).expect("a legal drop");
    assert_eq!(ordinal, 2048 + ORDINAL_STEP);
}

#[test]
fn the_first_placement_in_an_untouched_list_takes_the_step() {
    // Every fact is unplaced (the state of all 46 on S-2 today). Dragging one to
    // the very top must still produce a number that sorts it ahead of the tail.
    let facts = vec![unplaced("a", 1), unplaced("b", 2), unplaced("c", 3)];
    let ordinal = plan_move(&facts, "c", None, Some("a")).expect("a legal drop");
    let first_provisional = ORDINAL_STEP; // "a" is first in the unplaced tail
    assert!(
        ordinal < first_provisional,
        "{ordinal} must sort ahead of the unplaced tail's first entry",
    );
}

#[test]
fn a_drop_into_the_unplaced_tail_lands_between_its_neighbours() {
    // The tail's provisional numbers are 1024, 2048, 3072 (ceiling 0, no placed
    // facts). Dropping "c" between "a" and "b" must land strictly between.
    let facts = vec![unplaced("a", 1), unplaced("b", 2), unplaced("c", 3)];
    let ordinal = plan_move(&facts, "c", Some("a"), Some("b")).expect("a legal drop");
    assert!(
        ordinal > ORDINAL_STEP && ordinal < 2 * ORDINAL_STEP,
        "{ordinal} must fall between the two provisional ordinals",
    );
}

#[test]
fn the_unplaced_tail_always_sorts_after_every_placed_fact() {
    // The property the whole two-region scheme rests on: a human who has placed
    // three facts sees those three first, and nothing they never touched can
    // jump ahead of them.
    let facts = vec![
        placed("p1", 10),
        placed("p2", 20),
        unplaced("u1", 1),
        unplaced("u2", 2),
    ];
    let order = effective_order(&facts);
    let ids: Vec<&str> = order.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, vec!["p1", "p2", "u1", "u2"]);
    let placed_max = order[1].1;
    assert!(
        order[2].1 > placed_max && order[3].1 > order[2].1,
        "provisional ordinals must exceed every stored one, in tail order",
    );
}

#[test]
fn an_untouched_list_keeps_exactly_its_c_ordinal_order() {
    // THE regression guard the ruling asked for by name: a scenario nobody has
    // dragged in must render identically to how it did before this task shipped.
    // Input order is deliberately shuffled — the sort, not the input, decides.
    let facts = vec![unplaced("c3", 30), unplaced("c1", 10), unplaced("c2", 20)];
    let order = effective_order(&facts);
    let ids: Vec<&str> = order.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, vec!["c1", "c2", "c3"], "C-ordinal order, untouched");
}

#[test]
fn un_numbered_candidates_sort_last_within_the_tail() {
    // Matches the card list's own rule rather than inventing a second one: a
    // candidate gather has not numbered yet goes to the end, not to the front
    // (which is where a `None`-sorts-first comparison would put it).
    let mut no_code = unplaced("nc", 0);
    no_code.code_ordinal = None;
    let facts = vec![no_code, unplaced("c1", 10)];
    let order = effective_order(&facts);
    let ids: Vec<&str> = order.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, vec!["c1", "nc"]);
}

#[test]
fn tiers_group_ahead_of_ordinals() {
    // Weight decides the block, position decides the seat. A `background` fact
    // with a tiny ordinal must not outrank a `carries` fact with a large one —
    // otherwise starring something would silently reorder the list.
    let mut heavy = placed("heavy", 9000);
    heavy.tier = FactTier::Carries;
    let mut light = placed("light", 10);
    light.tier = FactTier::Background;
    let order = effective_order(&[light, heavy]);
    let ids: Vec<&str> = order.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, vec!["heavy", "light"]);
}

#[test]
fn adjacent_ordinals_refuse_rather_than_renumbering() {
    // Gap exhaustion is a REFUSAL. Any returned number here would either equal a
    // neighbour or require rewriting another row — both forbidden.
    let facts = vec![placed("a", 100), placed("b", 101), placed("c", 5000)];
    let refusal = plan_move(&facts, "c", Some("a"), Some("b"))
        .expect_err("there is no integer strictly between 100 and 101");
    assert_eq!(refusal, MoveRefusal::GapExhausted);
}

#[test]
fn ten_successive_midpoint_drops_fit_before_exhaustion() {
    // The claim ORDINAL_STEP's doc makes, asserted rather than asserted-by-comment:
    // repeatedly dropping into the same gap must survive ten drags. If somebody
    // lowers the step, this fails here rather than on Roman's screen.
    let low = 0;
    let mut high = ORDINAL_STEP;
    let mut drops = 0;
    while high - low > 1 {
        let facts = vec![placed("a", low), placed("b", high), placed("m", 999_999)];
        let mid = plan_move(&facts, "m", Some("a"), Some("b")).expect("still room");
        assert!(mid > low && mid < high);
        high = mid;
        drops += 1;
    }
    assert!(
        drops >= 10,
        "only {drops} midpoint drops fit before exhaustion"
    );
}

#[test]
fn a_dragged_fact_that_is_not_here_is_refused_by_name() {
    let facts = vec![placed("a", 1024)];
    let refusal = plan_move(&facts, "ghost", Some("a"), None).expect_err("not in the scenario");
    assert_eq!(refusal, MoveRefusal::DraggedNotHere);
}

#[test]
fn a_neighbour_that_is_not_here_is_refused_by_name() {
    // The stale-page case: the human drops beside a row somebody else removed.
    // Distinguishable from every other refusal so the message can say so.
    let facts = vec![placed("a", 1024), placed("b", 2048)];
    let refusal = plan_move(&facts, "a", Some("ghost"), None).expect_err("the neighbour is gone");
    assert_eq!(refusal, MoveRefusal::NeighbourNotHere);
    let refusal_above =
        plan_move(&facts, "a", None, Some("ghost")).expect_err("the neighbour is gone");
    assert_eq!(refusal_above, MoveRefusal::NeighbourNotHere);
}

#[test]
fn a_facts_own_position_does_not_influence_where_it_lands() {
    // The dragged card is lifted out before the neighbours are read. If its own
    // ordinal were still in play, dropping it back between the same two rows
    // would compute a different number each time and the list would creep.
    let facts = vec![placed("a", 1024), placed("b", 2048), placed("c", 1536)];
    let first = plan_move(&facts, "c", Some("a"), Some("b")).expect("legal");
    let second = plan_move(&facts, "c", Some("a"), Some("b")).expect("legal");
    assert_eq!(
        first, second,
        "the same drop must always yield the same ordinal"
    );
    assert_eq!(first, 1536);
}

#[test]
fn naming_the_neighbours_backwards_is_refused_not_guessed() {
    // "after b, before a" describes a drop that cannot exist. Computing a number
    // from it would store a confident position for an incoherent request.
    let facts = vec![placed("a", 1024), placed("b", 2048), placed("c", 4096)];
    let refusal = plan_move(&facts, "c", Some("b"), Some("a"))
        .expect_err("the neighbours are the wrong way round");
    assert_eq!(refusal, MoveRefusal::GapExhausted);
}

#[test]
fn an_empty_block_yields_the_step_rather_than_zero() {
    // Leaves room to drop something ABOVE this fact later without immediately
    // needing a negative ordinal.
    let facts = vec![placed("only", 5)];
    assert_eq!(plan_move(&facts, "only", None, None), Ok(ORDINAL_STEP));
}

#[test]
fn extreme_ordinals_saturate_rather_than_overflowing() {
    // A row hand-edited to i32::MAX must not panic the read path in debug or
    // wrap into a negative ordinal in release — either would reorder the list
    // through arithmetic nobody asked for.
    let facts = vec![placed("a", i32::MAX), unplaced("b", 1)];
    let ordinal = plan_move(&facts, "b", Some("a"), None).expect("still legal");
    assert_eq!(ordinal, i32::MAX, "saturated, not wrapped");
    assert!(ordinal > 0);
}
