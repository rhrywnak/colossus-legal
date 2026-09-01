//! Tests for reciprocal-rank fusion and the conservation identity.
//!
//! All pure — hand-built lists, no corpus. That is the point: a card in one
//! read only, a tie, and an empty side are states the live data may not happen
//! to exhibit today, and they are exactly the ones that break silently.

use super::*;

fn ids(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_string()).collect()
}

fn find<'a>(fused: &'a [FusedCard], id: &str) -> &'a FusedCard {
    fused
        .iter()
        .find(|c| c.evidence_id == id)
        .unwrap_or_else(|| panic!("{id} is missing from the fused list"))
}

/// ⚑ A card BOTH reads found beats a card either read found first alone.
///
/// This is the whole reason for fusing rather than concatenating. The lexical
/// read knows `$50,000`; the vector read knows "the money he deposited". A card
/// they agree on is the one a human meant.
#[test]
fn agreement_between_the_two_reads_outranks_either_read_alone() {
    let vector = ids(&["only-vector", "agreed"]);
    let lexical = ids(&["only-lexical", "agreed"]);

    let fused = fuse(&vector, &lexical, RRF_K);

    assert_eq!(
        fused[0].evidence_id, "agreed",
        "a card at rank 2 in both must beat a card at rank 1 in one"
    );
    let agreed = find(&fused, "agreed");
    assert_eq!(agreed.vector_rank, Some(2));
    assert_eq!(agreed.lexical_rank, Some(2));
    // 1/62 + 1/62 = 0.032258…, against 1/61 = 0.016393… for a rank-1 singleton.
    assert!((agreed.fused_score - 2.0 / 62.0).abs() < 1e-12);
    assert!(agreed.fused_score > find(&fused, "only-vector").fused_score);
}

/// A card only ONE read returned still reaches the list, and says which read.
///
/// The trigram half exists to find cards full text cannot; dropping a card
/// because only one read saw it would throw that away.
#[test]
fn a_card_only_one_read_found_still_reaches_the_list() {
    let fused = fuse(&ids(&["v-only"]), &ids(&["l-only"]), RRF_K);

    assert_eq!(fused.len(), 2);
    let v = find(&fused, "v-only");
    assert_eq!(v.vector_rank, Some(1));
    assert_eq!(
        v.lexical_rank, None,
        "None means that read did not return it"
    );
    let l = find(&fused, "l-only");
    assert_eq!(l.vector_rank, None);
    assert_eq!(l.lexical_rank, Some(1));
}

/// Ties break on the id, so the same inputs always produce the same order.
///
/// Two cards at the same rank in the same single read score identically. Left
/// to the map's iteration order the two would swap between runs, and a
/// measurement taken today could not be compared with one taken tomorrow.
#[test]
fn ties_break_on_the_id_so_two_runs_agree() {
    // Same rank in the same read on purpose: two separate one-card reads.
    let first = fuse(&ids(&["zebra"]), &ids(&["alpha"]), RRF_K);
    let second = fuse(&ids(&["zebra"]), &ids(&["alpha"]), RRF_K);

    assert!(
        (first[0].fused_score - first[1].fused_score).abs() < 1e-12,
        "a genuine tie"
    );
    assert_eq!(first[0].evidence_id, "alpha", "ascending id breaks the tie");
    assert_eq!(first[1].evidence_id, "zebra");
    assert_eq!(first, second, "and it is the same order every run");
}

/// One read returning nothing is a normal state, not an error.
///
/// A scenario whose query has no lexical hits — every term stopworded, say —
/// still has a vector pool, and the fused list must be that pool in order.
#[test]
fn an_empty_side_leaves_the_other_sides_order_intact() {
    let vector = ids(&["a", "b", "c"]);

    let fused = fuse(&vector, &[], RRF_K);
    assert_eq!(
        fused
            .iter()
            .map(|c| c.evidence_id.as_str())
            .collect::<Vec<_>>(),
        ["a", "b", "c"],
        "with one read empty the fused order IS the other read's order"
    );
    assert_eq!(fused[0].rank, 1);
    assert_eq!(fused[2].rank, 3);
    assert!(fused.iter().all(|c| c.lexical_rank.is_none()));

    assert!(
        fuse(&[], &[], RRF_K).is_empty(),
        "both empty is an empty list"
    );
}

/// Ranks are 1-based, dense, and in score order.
#[test]
fn the_output_ranks_are_dense_and_ordered() {
    let fused = fuse(&ids(&["a", "b", "c"]), &ids(&["c", "b"]), RRF_K);

    for (index, card) in fused.iter().enumerate() {
        assert_eq!(card.rank, index + 1, "rank is the 1-based position");
    }
    for pair in fused.windows(2) {
        assert!(
            pair[0].fused_score >= pair[1].fused_score,
            "the list must be in descending score order"
        );
    }
    assert_eq!(
        fused[0].evidence_id, "c",
        "rank 3 + rank 1 beats rank 2 + rank 2"
    );
}

/// A read that returns the same id twice is not double-counted.
///
/// It would be a defect in that read, and scoring it twice would promote the
/// broken card to the top of the gather instead of leaving it where it belongs.
#[test]
fn a_duplicate_within_one_read_is_counted_once() {
    let fused = fuse(&ids(&["dup", "dup", "other"]), &[], RRF_K);

    assert_eq!(fused.len(), 2, "the duplicate is one card, not two");
    let dup = find(&fused, "dup");
    assert_eq!(dup.vector_rank, Some(1), "the FIRST occurrence is the rank");
    assert!((dup.fused_score - 1.0 / 61.0).abs() < 1e-12, "scored once");
}

// ---------------------------------------------------------------------------
// The conservation identity
// ---------------------------------------------------------------------------

/// ⚑ Conservation, as a property rather than a printed line.
///
/// Every card in today's subject-only pool still appears. A card visible
/// yesterday and invisible today is a defect, not a ranking.
#[test]
fn every_card_in_todays_pool_survives_into_the_ranked_list() {
    let baseline = ids(&["kept-1", "kept-2", "kept-3"]);
    // The ranked list re-orders them and adds two the widening reached.
    let fused = fuse(
        &ids(&["added-1", "kept-3", "kept-1"]),
        &ids(&["kept-2", "added-2"]),
        RRF_K,
    );

    assert!(
        conservation_gap(&baseline, &fused).is_empty(),
        "the ranked gather adds and re-orders; it never drops"
    );
    assert_eq!(
        fused.len(),
        5,
        "and it did add the two the widening reached"
    );
}

/// A violation NAMES the cards, so it is something to go and look at.
#[test]
fn a_conservation_violation_names_the_cards_that_vanished() {
    let baseline = ids(&["kept", "vanished-a", "vanished-b"]);
    let fused = fuse(&ids(&["kept"]), &[], RRF_K);

    assert_eq!(
        conservation_gap(&baseline, &fused),
        vec!["vanished-a", "vanished-b"],
        "a bare false would tell an operator nothing about which card to chase"
    );
}

/// An empty baseline conserves trivially — a scenario whose subject has no
/// evidence is a real state, not a failure.
#[test]
fn an_empty_baseline_is_conserved_by_anything() {
    assert!(conservation_gap(&[], &fuse(&ids(&["a"]), &[], RRF_K)).is_empty());
    assert!(conservation_gap(&[], &[]).is_empty());
}
