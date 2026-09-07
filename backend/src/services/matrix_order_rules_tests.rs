// §3's RULES: what is hidden, what folds into what, and what is never dropped.
//
// The ORDER these produce is asserted in `matrix_order_order_tests`. What is
// asserted here is membership — which is the half that can silently take a piece
// of evidence off a proof surface.

use super::test_support::{hidden_ids, item, visible_ids};
use super::*;

// ─── The hide rules ──────────────────────────────────────────────────────────

/// `does_not_belong` hides, and says the machine did it.
#[test]
fn a_does_not_belong_item_is_hidden_as_machine_excluded() {
    let mut excluded = item("excluded");
    excluded.role = Some(EdgeRole::DoesNotBelong);
    let ordered = order_items(&[excluded, item("kept-in")]);

    assert_eq!(visible_ids(&ordered), vec!["kept-in"]);
    assert_eq!(hidden_ids(&ordered), vec!["excluded"]);
    assert_eq!(
        ordered[1].hidden_reason,
        Some(HiddenReason::MachineExcluded)
    );
}

/// `remove` hides, and says a human did it.
#[test]
fn a_removed_item_is_hidden_as_human_removed() {
    let mut removed = item("removed");
    removed.ruling = Some(MatrixRuling::Remove);
    let ordered = order_items(&[removed, item("shown")]);

    assert_eq!(visible_ids(&ordered), vec!["shown"]);
    assert_eq!(ordered[1].hidden_reason, Some(HiddenReason::HumanRemoved));
}

/// The two hidden reasons are distinguishable on the wire.
///
/// They render alike, but WHY an item is missing from a proof list is exactly the
/// kind of fact that must not have to be inferred from its absence.
#[test]
fn the_two_hidden_reasons_carry_different_tokens() {
    assert_eq!(HiddenReason::MachineExcluded.code(), "does_not_belong");
    assert_eq!(HiddenReason::HumanRemoved.code(), "removed");
}

/// A human's KEEP overrules the machine's retraction.
///
/// Otherwise the Keep button on a retracted item is a control that visibly does
/// nothing, and `does_not_belong` becomes the one machine claim a human cannot
/// answer.
#[test]
fn a_human_keep_overrules_does_not_belong() {
    let mut rescued = item("rescued");
    rescued.role = Some(EdgeRole::DoesNotBelong);
    rescued.ruling = Some(MatrixRuling::Keep);

    let ordered = order_items(&[rescued]);
    assert_eq!(visible_ids(&ordered), vec!["rescued"]);
}

/// A human's REMOVE is never overruled — not by a role, and not by a rank.
#[test]
fn a_human_remove_wins_over_every_machine_claim() {
    let mut struck = item("struck");
    struck.role = Some(EdgeRole::DirectProof);
    struck.rank = Some(1);
    struck.confidence = Some(LinkConfidence::High);
    struck.ruling = Some(MatrixRuling::Remove);

    let ordered = order_items(&[struck]);
    assert!(visible_ids(&ordered).is_empty());
    assert_eq!(hidden_ids(&ordered), vec!["struck"]);
}

/// Hidden items keep §3's order among themselves, so the revealed group is not a
/// jumble.
#[test]
fn hidden_items_are_ordered_by_the_same_keys() {
    let mut second = item("hidden-rank-5");
    second.role = Some(EdgeRole::DoesNotBelong);
    second.rank = Some(5);
    let mut first = item("hidden-rank-1");
    first.role = Some(EdgeRole::DoesNotBelong);
    first.rank = Some(1);

    let ordered = order_items(&[second, first]);
    assert_eq!(hidden_ids(&ordered), vec!["hidden-rank-1", "hidden-rank-5"]);
}

// ─── The duplicate fold ──────────────────────────────────────────────────────

/// A duplicate folds into its target and becomes a "×2".
#[test]
fn a_duplicate_folds_into_its_target() {
    let target = item("target");
    let mut dup = item("dup");
    dup.role = Some(EdgeRole::DuplicateOf);
    dup.duplicate_of = Some("target");

    let ordered = order_items(&[target, dup]);
    assert_eq!(visible_ids(&ordered), vec!["target"]);
    assert_eq!(ordered[0].occurrences, 2);
}

/// Two duplicates of one target make it a "×3".
#[test]
fn several_duplicates_accumulate_on_one_target() {
    let target = item("target");
    let mut d1 = item("dup-1");
    d1.role = Some(EdgeRole::DuplicateOf);
    d1.duplicate_of = Some("target");
    let mut d2 = item("dup-2");
    d2.role = Some(EdgeRole::DuplicateOf);
    d2.duplicate_of = Some("target");

    let ordered = order_items(&[target, d1, d2]);
    assert_eq!(ordered.len(), 1);
    assert_eq!(ordered[0].occurrences, 3);
}

/// A row that folded nothing reports one occurrence, so the renderer's
/// "above one" rule leaves it unmarked.
#[test]
fn a_row_that_folded_nothing_reports_one_occurrence() {
    let ordered = order_items(&[item("alone")]);
    assert_eq!(ordered[0].occurrences, 1);
}

/// A duplicate whose target is NOT in this list stands alone.
///
/// This is the stale-pointer case — the same class of defect that orphaned 26
/// curated rulings on 2026-07-24. Folding it into nothing would delete a piece of
/// evidence with nothing on screen to say so.
#[test]
fn an_orphaned_duplicate_stands_alone_rather_than_vanishing() {
    let mut orphan = item("orphan");
    orphan.role = Some(EdgeRole::DuplicateOf);
    orphan.duplicate_of = Some("a-card-that-is-not-here");

    let ordered = order_items(&[orphan]);
    assert_eq!(visible_ids(&ordered), vec!["orphan"]);
    assert_eq!(ordered[0].occurrences, 1);
}

/// A duplicate whose target is HIDDEN stands alone.
///
/// Folding it under a removed row would carry it out of the list, so one Remove
/// would silently take a second item with it.
#[test]
fn a_duplicate_of_a_hidden_target_stands_alone() {
    let mut target = item("removed-target");
    target.ruling = Some(MatrixRuling::Remove);
    let mut dup = item("dup");
    dup.role = Some(EdgeRole::DuplicateOf);
    dup.duplicate_of = Some("removed-target");

    let ordered = order_items(&[target, dup]);
    assert_eq!(visible_ids(&ordered), vec!["dup"]);
    assert_eq!(ordered[0].occurrences, 1);
}

/// A duplicate a human has RULED on stands alone.
///
/// The ruling is a statement about that row. A row somebody kept is not a "×1" on
/// another row's tally, and folding it would erase an act the ledger records.
#[test]
fn a_duplicate_a_human_ruled_on_stands_alone() {
    let target = item("target");
    let mut dup = item("dup");
    dup.role = Some(EdgeRole::DuplicateOf);
    dup.duplicate_of = Some("target");
    dup.ruling = Some(MatrixRuling::Keep);

    let ordered = order_items(&[target, dup]);
    assert_eq!(visible_ids(&ordered), vec!["dup", "target"]);
    assert_eq!(ordered[0].occurrences, 1);
    assert_eq!(ordered[1].occurrences, 1);
}

/// A duplicate pointing at ITSELF stands alone rather than folding into its own
/// tally and disappearing.
#[test]
fn a_self_referential_duplicate_stands_alone() {
    let mut looped = item("loop");
    looped.role = Some(EdgeRole::DuplicateOf);
    looped.duplicate_of = Some("loop");

    let ordered = order_items(&[looped]);
    assert_eq!(visible_ids(&ordered), vec!["loop"]);
    assert_eq!(ordered[0].occurrences, 1);
}

// ─── Conservation ────────────────────────────────────────────────────────────

/// Every input item is accounted for: one row each, minus the folded ones, whose
/// count shows up in an occurrence tally.
///
/// The single most important property in this module. Any rule added later that
/// drops an item silently fails here.
#[test]
fn every_item_is_either_a_row_or_counted_in_one() {
    let target = item("target");
    let mut dup = item("dup");
    dup.role = Some(EdgeRole::DuplicateOf);
    dup.duplicate_of = Some("target");
    let mut excluded = item("excluded");
    excluded.role = Some(EdgeRole::DoesNotBelong);
    let mut removed = item("removed");
    removed.ruling = Some(MatrixRuling::Remove);
    let plain = item("plain");

    let inputs = [target, dup, excluded, removed, plain];
    let ordered = order_items(&inputs);

    let accounted: usize = ordered.iter().map(|o| o.occurrences).sum();
    assert_eq!(accounted, inputs.len(), "an item was dropped: {ordered:?}");
}

/// An empty list orders to an empty list, without panicking on the `hidden` map.
#[test]
fn an_empty_list_orders_to_an_empty_list() {
    assert!(order_items(&[]).is_empty());
}

/// The order does not depend on the input order — the same set in a different
/// sequence produces the same list.
#[test]
fn the_result_does_not_depend_on_input_order() {
    let mut a = item("a");
    a.rank = Some(2);
    let mut b = item("b");
    b.rank = Some(1);
    let mut c = item("c");
    c.rank = Some(3);

    let forwards = order_items(&[a, b, c]);
    let backwards = order_items(&[c, b, a]);
    assert_eq!(forwards, backwards);
}
