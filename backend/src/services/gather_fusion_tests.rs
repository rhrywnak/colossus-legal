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
    assert_eq!(fused[0].rank, Some(1));
    assert_eq!(fused[2].rank, Some(3));
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
        assert_eq!(card.rank, Some(index + 1), "rank is the 1-based position");
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

// ---------------------------------------------------------------------------
// Conservation by construction — the tail
// ---------------------------------------------------------------------------

/// ⚑ The identity HOLDS after the append, at any read depth.
///
/// Before the tail it could not: the reads return the most RELEVANT cards while
/// the pool is defined by MEMBERSHIP, and no top-K of the first contains the
/// second. Measured on the real corpus, 204 of S-9's 292 were absent.
#[test]
fn the_tail_makes_conservation_hold_however_little_the_reads_reached() {
    let baseline = ids(&["kept", "unreached-b", "unreached-a"]);
    // The reads reached one of the three, and one card outside the pool.
    let retrieved = fuse(&ids(&["outside", "kept"]), &[], RRF_K);
    assert_eq!(
        conservation_gap(&baseline, &retrieved).len(),
        2,
        "two of today's cards were not reached — the state the tail exists for"
    );

    let with_tail = append_conservation_tail(retrieved, &baseline);

    assert!(
        conservation_gap(&baseline, &with_tail).is_empty(),
        "after the append nothing visible yesterday is invisible today"
    );
    assert_eq!(with_tail.len(), 4, "two retrieved plus the two appended");
}

/// The appended cards are marked, scoreless, and BELOW every ranked card.
///
/// "Ranked 120th" and "neither read reached this" are different statements. A
/// list that rendered them identically would be lying by omission, so the
/// placement is carried per card and the tail sits last.
#[test]
fn tail_cards_are_marked_scoreless_and_last() {
    let baseline = ids(&["zzz-unreached", "kept"]);
    let with_tail = append_conservation_tail(fuse(&ids(&["kept"]), &[], RRF_K), &baseline);

    let ranked = &with_tail[0];
    assert_eq!(ranked.evidence_id, "kept");
    assert_eq!(ranked.placement, CardPlacement::Ranked);
    assert!(
        ranked.fused_score > 0.0,
        "a retrieved card has a real score"
    );

    let tail = &with_tail[1];
    assert_eq!(tail.evidence_id, "zzz-unreached");
    assert_eq!(tail.placement, CardPlacement::ConservationTail);
    assert_eq!(
        tail.fused_score, 0.0,
        "no read scored it, so it has no score"
    );
    assert_eq!(tail.vector_rank, None);
    assert_eq!(tail.lexical_rank, None);
    assert_eq!(
        tail.rank, None,
        "no read returned it, so there is no rank — carried in the TYPE rather than \
         needing a placement check to interpret a number"
    );
}

/// The tail is in ID order, so two runs append the same cards the same way.
#[test]
fn the_tail_is_ordered_so_two_runs_agree() {
    let baseline = ids(&["c", "a", "b"]);
    let first = append_conservation_tail(Vec::new(), &baseline);
    let second = append_conservation_tail(Vec::new(), &baseline);

    assert_eq!(
        first
            .iter()
            .map(|c| c.evidence_id.as_str())
            .collect::<Vec<_>>(),
        ["a", "b", "c"]
    );
    assert_eq!(first, second);
    assert!(
        first.iter().all(|c| c.rank.is_none()),
        "nothing here was retrieved, so nothing here has a rank"
    );
}

/// A card the reads DID reach is never duplicated into the tail.
#[test]
fn a_retrieved_card_is_not_appended_again() {
    let baseline = ids(&["kept"]);
    let with_tail = append_conservation_tail(fuse(&ids(&["kept"]), &[], RRF_K), &baseline);

    assert_eq!(with_tail.len(), 1, "one card, not two");
    assert_eq!(with_tail[0].placement, CardPlacement::Ranked);
}

/// Nothing to append is a no-op, and the placements survive untouched.
#[test]
fn a_fully_reached_pool_appends_nothing() {
    let ranked = fuse(&ids(&["a", "b"]), &[], RRF_K);
    let with_tail = append_conservation_tail(ranked.clone(), &ids(&["a", "b"]));

    assert_eq!(with_tail, ranked);
    assert!(with_tail
        .iter()
        .all(|c| c.placement == CardPlacement::Ranked));
}

/// The placement token spells the way the DTO will carry it.
#[test]
fn the_placement_tokens_match_their_serde_spelling() {
    for (placement, token) in [
        (CardPlacement::Ranked, "ranked"),
        (CardPlacement::ConservationTail, "conservation_tail"),
    ] {
        assert_eq!(
            serde_json::to_value(placement).expect("serializes"),
            serde_json::json!(token),
            "L2c renders from this token; it must not drift"
        );
    }
}
