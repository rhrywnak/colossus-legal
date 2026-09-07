// Tests for `domain::matrix_edge`.
//
// Two read-only vocabularies and one sort order. The order is the part worth
// asserting hardest: §3 states it in prose, and prose in an instruction is not
// something a build can check.

use super::*;

/// Every role token round-trips. A token this build writes and cannot read would
/// be an item nobody can classify — but more to the point, these tokens come from
/// a pass that ran OUTSIDE this build, so the round trip is the only proof that
/// `code()` and `try_from` describe the same five words.
#[test]
fn every_role_code_parses_back_to_itself() {
    for role in EdgeRole::ALL {
        assert_eq!(EdgeRole::try_from(role.code()), Ok(*role));
    }
}

/// The five tokens, pinned by literal, because the GRAPH holds them: renaming one
/// here would silently reclassify 1,194 edges.
#[test]
fn the_role_tokens_are_the_five_the_pass_wrote() {
    let codes: Vec<&str> = EdgeRole::ALL.iter().map(|r| r.code()).collect();
    assert_eq!(
        codes,
        vec![
            "direct_proof",
            "their_own_words",
            "context",
            "duplicate_of",
            "does_not_belong"
        ]
    );
}

/// An unknown role is refused by name, and the refusal lists what this build does
/// know — so an operator can see at a glance whether the frontend is behind a
/// newer pass or the graph holds a typo.
#[test]
fn an_unknown_role_is_refused_by_name() {
    let err = EdgeRole::try_from("probably_relevant").expect_err("not a role");
    assert_eq!(err.token, "probably_relevant");
    assert!(err.to_string().contains("probably_relevant"));
    assert!(err.to_string().contains("does_not_belong"));
}

/// EXACTLY ONE role hides its item.
///
/// If `Context` or `DuplicateOf` ever answered `true` here, proof would vanish
/// from a list a lawyer reads — 339 context edges and 144 duplicates, gone with
/// nothing on screen to say so.
#[test]
fn only_does_not_belong_hides_the_item() {
    let hiding: Vec<&str> = EdgeRole::ALL
        .iter()
        .filter(|r| r.hides_the_item())
        .map(|r| r.code())
        .collect();
    assert_eq!(hiding, vec!["does_not_belong"]);
}

/// Exactly one role folds its item under another.
#[test]
fn only_duplicate_of_is_a_duplicate() {
    let folding: Vec<&str> = EdgeRole::ALL
        .iter()
        .filter(|r| r.is_duplicate())
        .map(|r| r.code())
        .collect();
    assert_eq!(folding, vec!["duplicate_of"]);
}

/// Hiding and folding are different things, and no role does both. A role that
/// did would fold an item under a target and then hide it there, taking the
/// target's whole "×N" with it.
#[test]
fn no_role_both_hides_and_folds() {
    for role in EdgeRole::ALL {
        assert!(
            !(role.hides_the_item() && role.is_duplicate()),
            "{} both hides and folds",
            role.code()
        );
    }
}

/// Every confidence token round-trips.
#[test]
fn every_confidence_code_parses_back_to_itself() {
    for c in LinkConfidence::ALL {
        assert_eq!(LinkConfidence::try_from(c.code()), Ok(*c));
    }
}

/// THE ORDER §3 STATES, asserted as an order rather than as three numbers.
///
/// "high > medium > older-untagged". An untagged item must never outrank a rated
/// one: 286 edges predate the linking pass, and floating them to the top of every
/// paragraph would put the least-examined proof in front of the reader first.
#[test]
fn confidence_sorts_high_then_medium_then_low_then_unrated() {
    let mut weights: Vec<u8> = LinkConfidence::ALL
        .iter()
        .map(|c| c.sort_weight())
        .collect();
    weights.push(LinkConfidence::UNRATED_SORT_WEIGHT);

    let mut sorted = weights.clone();
    sorted.sort_unstable();
    assert_eq!(
        weights, sorted,
        "ALL is declared strongest-first, so its weights must already ascend"
    );

    assert!(LinkConfidence::High.sort_weight() < LinkConfidence::Medium.sort_weight());
    assert!(LinkConfidence::Medium.sort_weight() < LinkConfidence::Low.sort_weight());
    assert!(LinkConfidence::Low.sort_weight() < LinkConfidence::UNRATED_SORT_WEIGHT);
}

/// No two confidences share a weight — two that did would sort arbitrarily
/// against each other and make the list shuffle between reads.
#[test]
fn the_confidence_weights_are_distinct() {
    let mut weights: Vec<u8> = LinkConfidence::ALL
        .iter()
        .map(|c| c.sort_weight())
        .collect();
    weights.push(LinkConfidence::UNRATED_SORT_WEIGHT);
    let count = weights.len();
    weights.sort_unstable();
    weights.dedup();
    assert_eq!(weights.len(), count, "two confidence weights collide");
}

/// An unknown confidence is refused by name.
#[test]
fn an_unknown_confidence_is_refused_by_name() {
    let err = LinkConfidence::try_from("very_high").expect_err("not a confidence");
    assert_eq!(err.token, "very_high");
    assert!(err.to_string().contains("high/medium/low"));
}

/// The two vocabularies do not overlap: they sit in adjacent columns of the same
/// projection, and a shared token would make one readable as the other.
#[test]
fn the_role_and_confidence_vocabularies_are_disjoint() {
    for role in EdgeRole::ALL {
        assert!(LinkConfidence::try_from(role.code()).is_err());
    }
}
