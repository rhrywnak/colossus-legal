//! Which tier answers, and why the order is a preference rather than a fallback
//! chain by accident.
//!
//! Every fixture is a measured damage class from
//! `doc-transcript-post-appeal-restructure-02-28-2012-clean` — the document where
//! a re-extraction from corrected page text left **0 of 14** movable nodes
//! matchable byte-for-byte, and twenty of Roman's curated rows went to the human
//! queue for a reason that was purely typographic.
//!
//! Split three ways to stay under the 300-line rule: `plan_tests.rs` asserts the
//! plan's SHAPE (yield, queue order, round-trip), this file asserts which tier
//! answers, and `plan_guard_tests.rs` asserts what every tier still refuses.

use super::*;
use crate::remap::normalize::NearMatchSettings;

/// Build a plan at the documented default thresholds.
fn build(snap: &Snapshot, new_nodes: &[NewNode]) -> RemapPlan {
    RemapPlan::build(snap, new_nodes, NearMatchSettings::default())
}

fn snapshot(nodes: Vec<SnapshotNode>) -> Snapshot {
    Snapshot {
        document_id: "doc-transcript-post-appeal-restructure-02-28-2012-clean".to_string(),
        taken_note: "before the OCR-repair reprocess".to_string(),
        nodes,
    }
}

fn transcript_old(id: &str, page: i64, quote: &str, curated_rows: u64) -> SnapshotNode {
    SnapshotNode {
        id: id.to_string(),
        page: Some(page),
        verbatim_quote: quote.to_string(),
        question: None,
        curated_rows,
    }
}

fn transcript_new(id: &str, page: i64, quote: &str) -> NewNode {
    NewNode {
        id: id.to_string(),
        page: Some(page),
        verbatim_quote: quote.to_string(),
        question: None,
    }
}

#[test]
fn an_exact_match_still_wins_and_is_reported_as_tier_one() {
    // Tier 1 must not be weakened by the tiers below it: when the quote agrees
    // byte for byte, that is the answer and no measurement is consulted.
    let snap = snapshot(vec![transcript_old(
        "doc:evidence:old1",
        8,
        "The objections seem to have two-fold thrusts.",
        4,
    )]);
    let plan = build(
        &snap,
        &[transcript_new(
            "doc:evidence:new1",
            8,
            "The objections seem to have two-fold thrusts.",
        )],
    );

    assert_eq!(
        plan.nodes[0].outcome,
        Match::Unambiguous {
            new_id: "doc:evidence:new1".to_string(),
            tier: MatchTier::Exact,
            score: 1.0,
        }
    );
    assert_eq!(plan.totals().unambiguous_exact, 1);
}

#[test]
fn a_hyphenated_line_break_matches_at_tier_two() {
    // The corrected page text keeps the transcript's justified-text hyphenation:
    // the same sentence arrives as `reason-` + newline + `ableness`. Tier 1
    // cannot see through it; tier 2 can, and the match is exact once it does.
    let snap = snapshot(vec![transcript_old(
        "doc:evidence:old1",
        15,
        "The reasonableness of the fee was never contested.",
        4,
    )]);
    let plan = build(
        &snap,
        &[transcript_new(
            "doc:evidence:new1",
            15,
            "The reason-\nableness of the fee\nwas never contested",
        )],
    );

    assert_eq!(
        plan.nodes[0].outcome,
        Match::Unambiguous {
            new_id: "doc:evidence:new1".to_string(),
            tier: MatchTier::Normalized,
            score: 1.0,
        }
    );
    assert_eq!(plan.totals().unambiguous_normalized, 1);
}

#[test]
fn a_leading_clause_superset_matches_at_tier_three_with_its_score() {
    // The `630cbb22` class, verbatim from the transcript: the re-extraction kept
    // the whole statement and put a clause in front of it. Measured similarity
    // 0.696 / coverage 1.000 — BELOW the default similarity threshold, so the
    // fixture runs at the coverage-led setting, which is the point of the
    // thresholds being dialable at all.
    let snap = snapshot(vec![transcript_old(
        "doc:evidence:old1",
        15,
        "that--that you may want to take this under advisement, and you may \
         want to issue a written opinion. But that's just a suggestion.",
        4,
    )]);
    let new_nodes = [transcript_new(
        "doc:evidence:new1",
        15,
        "that there's a distinct possibility of a second appeal, that--that you \
         may\nwant to take this under advisement, and you may want to\nissue a \
         written opinion. But that's just a suggestion.",
    )];

    let settings = NearMatchSettings::new(Some(0.60), Some(0.95)).expect("in range");
    let plan = RemapPlan::build(&snap, &new_nodes, settings);

    match &plan.nodes[0].outcome {
        Match::Unambiguous {
            new_id,
            tier,
            score,
        } => {
            assert_eq!(new_id, "doc:evidence:new1");
            assert_eq!(*tier, MatchTier::Near);
            assert!(
                (0.60..0.80).contains(score),
                "the measured score for this pair is 0.696; got {score}"
            );
        }
        other => panic!("expected a tier-3 match, got {other:?}"),
    }
    assert_eq!(plan.totals().unambiguous_near, 1);
    assert_eq!(plan.auto_moves()[0].tier, MatchTier::Near);

    // And at the DEFAULT thresholds the same pair is refused — the finding this
    // task measured, pinned so it cannot change silently.
    assert_eq!(build(&snap, &new_nodes).nodes[0].outcome, Match::Unmatched);
}

#[test]
fn small_drift_inside_one_sentence_matches_at_tier_three_by_default() {
    // The `e8e68f48` class, measured at similarity 0.955 / coverage 1.00: the new
    // quote is the old one with one extra word at the front. This one clears the
    // DEFAULT thresholds, and must keep doing so.
    let snap = snapshot(vec![transcript_old(
        "doc:evidence:old1",
        13,
        "they should be weighed by the Court because in the event that there is \
         another appeal.",
        4,
    )]);
    let plan = build(
        &snap,
        &[transcript_new(
            "doc:evidence:new1",
            13,
            "but\nthey should be weighed by the Court because in the event\nthat \
             there is another appeal.",
        )],
    );

    assert!(
        matches!(
            plan.nodes[0].outcome,
            Match::Unambiguous {
                tier: MatchTier::Near,
                ..
            }
        ),
        "got {:?}",
        plan.nodes[0].outcome
    );
}

#[test]
fn an_exact_candidate_beats_a_loose_one_on_the_same_page() {
    // Tier order is a preference, not an accident: newB agrees byte for byte, so
    // newA — which would match at tier 2 — never gets a hearing.
    let snap = snapshot(vec![transcript_old(
        "doc:evidence:old1",
        9,
        "The fee petition was filed in April of that year.",
        12,
    )]);
    let plan = build(
        &snap,
        &[
            transcript_new(
                "doc:evidence:newA",
                9,
                "The fee petition was filed in April of that year",
            ),
            transcript_new(
                "doc:evidence:newB",
                9,
                "The fee petition was filed in April of that year.",
            ),
        ],
    );

    assert_eq!(
        plan.nodes[0].outcome,
        Match::Unambiguous {
            new_id: "doc:evidence:newB".to_string(),
            tier: MatchTier::Exact,
            score: 1.0,
        }
    );
}

#[test]
fn the_tier_breakdown_adds_up_to_the_unambiguous_count() {
    // Standing Rule 1: a yield that rests on tier 3 is a different run from one
    // that rests on tier 1, and the totals must be able to say which.
    let snap = snapshot(vec![
        transcript_old("doc:evidence:exact", 1, "A stated fact.", 1),
        transcript_old("doc:evidence:loose", 2, "The reasonableness of it.", 1),
    ]);
    let plan = build(
        &snap,
        &[
            transcript_new("doc:evidence:n1", 1, "A stated fact."),
            transcript_new("doc:evidence:n2", 2, "The reason-\nableness of it"),
        ],
    );

    let t = plan.totals();
    assert_eq!(
        (
            t.unambiguous,
            t.unambiguous_exact,
            t.unambiguous_normalized,
            t.unambiguous_near
        ),
        (2, 1, 1, 0)
    );
    assert_eq!(t.yield_percent(), 100.0);
}

#[test]
fn the_tier_labels_are_the_names_the_proposal_prints() {
    // These three strings reach Roman on every MAP line and every queue entry.
    // Pinned here so a rename cannot happen quietly in one place and not the
    // others.
    assert_eq!(MatchTier::Exact.label(), "tier1-exact");
    assert_eq!(MatchTier::Normalized.label(), "tier2-normalized");
    assert_eq!(MatchTier::Near.label(), "tier3-near");
}
