// §3's ORDER: the four sort keys, and the tie-break that makes the list stable.
//
// The order is a CLAIM the page makes to a lawyer, so each key is asserted on its
// own (with the keys above it held equal) and then all four together. The rules
// that decide what is in the list at all live in `matrix_order_rules_tests`.

use super::test_support::{item, visible_ids};
use super::*;

// ─── Key 1: a human's keep ───────────────────────────────────────────────────

/// A kept item leads, even when the machine ranked it last.
///
/// This is the key that makes the page readable as reviewed work. If rank came
/// first, an item Roman confirmed could sit below eleven the machine liked more,
/// and the reader would have no way to see that anyone had been through it.
#[test]
fn a_kept_item_leads_however_the_machine_ranked_it() {
    let mut kept = item("kept");
    kept.rank = Some(40);
    kept.ruling = Some(MatrixRuling::Keep);
    let mut top = item("machine-top");
    top.rank = Some(1);

    let ordered = order_items(&[top, kept]);
    assert_eq!(visible_ids(&ordered), vec!["kept", "machine-top"]);
}

/// Two kept items fall back to the remaining keys among themselves — being kept
/// puts an item in the front group, it does not make it first.
#[test]
fn kept_items_are_ordered_among_themselves_by_rank() {
    let mut later = item("kept-later");
    later.ruling = Some(MatrixRuling::Keep);
    later.rank = Some(9);
    let mut sooner = item("kept-sooner");
    sooner.ruling = Some(MatrixRuling::Keep);
    sooner.rank = Some(2);

    let ordered = order_items(&[later, sooner]);
    assert_eq!(visible_ids(&ordered), vec!["kept-sooner", "kept-later"]);
}

// ─── Key 2: rank ─────────────────────────────────────────────────────────────

/// Rank ascends: 1 is the pass's best item within a stance.
#[test]
fn rank_ascends() {
    let (mut a, mut b, mut c) = (item("third"), item("first"), item("second"));
    a.rank = Some(3);
    b.rank = Some(1);
    c.rank = Some(2);
    let ordered = order_items(&[a, b, c]);
    assert_eq!(visible_ids(&ordered), vec!["first", "second", "third"]);
}

/// An UNRANKED item sorts after every ranked one.
///
/// The regression this catches is specific: `Option<i64>` orders `None` first in
/// Rust, so the obvious implementation would float the 36 edges the pass never
/// reached to the top of every paragraph.
#[test]
fn an_unranked_item_sorts_after_every_ranked_one() {
    let unranked = item("unranked");
    let mut ranked_last = item("ranked-99");
    ranked_last.rank = Some(99);
    let ordered = order_items(&[unranked, ranked_last]);
    assert_eq!(visible_ids(&ordered), vec!["ranked-99", "unranked"]);
}

// ─── Key 3: confidence ───────────────────────────────────────────────────────

/// With rank equal, confidence decides: high, then medium, then unrated.
#[test]
fn confidence_breaks_a_rank_tie_high_medium_then_unrated() {
    let mut high = item("high");
    let mut medium = item("medium");
    let mut unrated = item("unrated");
    for i in [&mut high, &mut medium, &mut unrated] {
        i.rank = Some(4);
    }
    high.confidence = Some(LinkConfidence::High);
    medium.confidence = Some(LinkConfidence::Medium);

    let ordered = order_items(&[unrated, medium, high]);
    assert_eq!(visible_ids(&ordered), vec!["high", "medium", "unrated"]);
}

/// Confidence never overrules rank. A high-confidence item ranked 7 stays below a
/// medium one ranked 2 — the pass's ordering within a stance is the stronger
/// claim, and §3 puts it first for that reason.
#[test]
fn confidence_does_not_overrule_rank() {
    let mut high_but_late = item("high-rank-7");
    high_but_late.rank = Some(7);
    high_but_late.confidence = Some(LinkConfidence::High);
    let mut medium_but_early = item("medium-rank-2");
    medium_but_early.rank = Some(2);
    medium_but_early.confidence = Some(LinkConfidence::Medium);

    let ordered = order_items(&[high_but_late, medium_but_early]);
    assert_eq!(visible_ids(&ordered), vec!["medium-rank-2", "high-rank-7"]);
}

// ─── Key 4: document date ────────────────────────────────────────────────────

/// OLDEST FIRST. The earliest record of a thing is the one a hearing wants.
#[test]
fn document_date_ascends_oldest_first() {
    let mut older = item("2009");
    let mut newer = item("2014");
    older.document_date = Some("2009-11-05");
    newer.document_date = Some("2014-07-10");
    let ordered = order_items(&[newer, older]);
    assert_eq!(visible_ids(&ordered), vec!["2009", "2014"]);
}

/// An UNDATED item sorts last, and so does one carrying the empty string.
///
/// Four DEV documents hold `document_date = ''` and six hold no property at all.
/// An empty string compares BELOW every real date, so treating it as a date would
/// put the undated documents at the very top of every paragraph — which is the
/// specific defect this test exists to prevent.
#[test]
fn an_undated_document_sorts_last_and_so_does_an_empty_date() {
    let mut dated = item("dated");
    dated.document_date = Some("2012-02-28");
    let missing = item("no-date-property");
    let mut blank = item("empty-string-date");
    blank.document_date = Some("");

    let ordered = order_items(&[blank, missing, dated]);
    assert_eq!(visible_ids(&ordered)[0], "dated");
    assert_eq!(visible_ids(&ordered).len(), 3, "nothing was dropped");
}

// ─── The tie-break that makes the list stable ────────────────────────────────

/// Items equal on all four keys order by id, so two reads agree.
///
/// Without this the sort is arbitrary among ties, and a page that claims its
/// order is a ranking would visibly reshuffle on refresh.
#[test]
fn items_equal_on_every_key_order_by_id() {
    let ordered = order_items(&[item("ccc"), item("aaa"), item("bbb")]);
    assert_eq!(visible_ids(&ordered), vec!["aaa", "bbb", "ccc"]);
}

/// All four keys at once, on one list — the shape §3 describes end to end.
#[test]
fn the_four_keys_apply_in_order_on_one_list() {
    let mut kept = item("d-kept-rank-30");
    kept.ruling = Some(MatrixRuling::Keep);
    kept.rank = Some(30);

    let mut rank1 = item("a-rank-1");
    rank1.rank = Some(1);

    let mut rank2_high = item("b-rank-2-high");
    rank2_high.rank = Some(2);
    rank2_high.confidence = Some(LinkConfidence::High);

    let mut rank2_medium_old = item("c-rank-2-medium-2009");
    rank2_medium_old.rank = Some(2);
    rank2_medium_old.confidence = Some(LinkConfidence::Medium);
    rank2_medium_old.document_date = Some("2009-01-01");

    let mut rank2_medium_new = item("e-rank-2-medium-2014");
    rank2_medium_new.rank = Some(2);
    rank2_medium_new.confidence = Some(LinkConfidence::Medium);
    rank2_medium_new.document_date = Some("2014-01-01");

    let ordered = order_items(&[rank2_medium_new, rank2_high, rank1, kept, rank2_medium_old]);
    assert_eq!(
        visible_ids(&ordered),
        vec![
            "d-kept-rank-30",
            "a-rank-1",
            "b-rank-2-high",
            "c-rank-2-medium-2009",
            "e-rank-2-medium-2014",
        ]
    );
}
