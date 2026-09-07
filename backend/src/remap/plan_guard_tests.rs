//! What the tiers refuse, which is the half that protects Roman's curated rows.
//!
//! A matcher that is only tested on the pairs it SHOULD match is not tested: the
//! whole risk of loosening tier 1 is that something now matches which should not,
//! and every case below is a shape that reads as a match to a human skimming it.
//! Companion to `plan_tier_tests.rs`.

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
fn two_near_candidates_on_one_page_are_ambiguous_and_never_auto_applied() {
    // Both candidates measure 0.958 / 0.900 against the old quote — well clear of
    // the defaults, and indistinguishable from each other. Picking either would
    // move twelve of Roman's curated rows onto a guess.
    let snap = snapshot(vec![transcript_old(
        "doc:evidence:old1",
        9,
        "The fee petition was filed in April of the year.",
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
                "doc:evidence:newC",
                9,
                "The fee petition was filed in April of this year",
            ),
        ],
    );

    match &plan.nodes[0].outcome {
        Match::Ambiguous { candidates, tier } => {
            assert_eq!(candidates.len(), 2);
            assert_eq!(*tier, MatchTier::Near);
        }
        other => panic!("expected a tier-3 ambiguity, got {other:?}"),
    }
    assert!(plan.auto_moves().is_empty());
    assert_eq!(plan.totals().curated_rows_at_risk, 12);
}

#[test]
fn a_quote_below_the_thresholds_stays_unmatched() {
    // The `44f15b2b` class: the statement genuinely has no counterpart in the new
    // extraction. Inventing one would move four of Roman's curated rows onto a
    // different sentence, which is worse than an orphan.
    let snap = snapshot(vec![transcript_old(
        "doc:evidence:old1",
        11,
        "This affirmance is a res judicata judgment from the appellate court.",
        4,
    )]);
    let plan = build(
        &snap,
        &[transcript_new(
            "doc:evidence:new1",
            11,
            "It is not appropriate to talk any more about what happened with the \
             missing cuff links.",
        )],
    );

    assert_eq!(plan.nodes[0].outcome, Match::Unmatched);
    assert_eq!(plan.totals().curated_rows_at_risk, 4);
}

#[test]
fn the_word_coverage_gate_refuses_a_new_quote_that_dropped_half_the_statement() {
    // The `67ce21a3` class: the new quote is a TRUNCATION, not a superset. It
    // reads as the same statement to a human skimming, which is exactly why the
    // coverage measure has to be the one that answers.
    let snap = snapshot(vec![transcript_old(
        "doc:evidence:old1",
        7,
        "The CFS and attorney Phillips had every right to bill for services \
         rendered in addressing the objections.",
        4,
    )]);
    let plan = build(
        &snap,
        &[transcript_new(
            "doc:evidence:new1",
            7,
            "The Court of\nAppeals said, The CFS and attorney Phillips had every \
             right",
        )],
    );

    assert_eq!(plan.nodes[0].outcome, Match::Unmatched);
}

#[test]
fn a_different_question_blocks_a_match_that_the_quote_alone_would_allow() {
    // Domain note: `question` is in the stable key because quote+page alone left
    // 119/131 distinct and quote+page+question left 129/131. Two identical
    // answers to two different questions are two statements.
    let snap = snapshot(vec![SnapshotNode {
        id: "doc:evidence:old1".to_string(),
        page: Some(4),
        verbatim_quote: "Yes.".to_string(),
        question: Some("Did you sign it?".to_string()),
        curated_rows: 5,
    }]);
    let plan = build(
        &snap,
        &[NewNode {
            id: "doc:evidence:new1".to_string(),
            page: Some(4),
            verbatim_quote: "Yes".to_string(),
            question: Some("Did you read it?".to_string()),
        }],
    );

    assert_eq!(plan.nodes[0].outcome, Match::Unmatched);
}

#[test]
fn a_question_that_appeared_or_vanished_does_not_block_a_match() {
    // 287 of 525 live Evidence nodes answer nobody. A re-extraction that starts
    // emitting a question has not changed what the document says.
    let snap = snapshot(vec![transcript_old("doc:evidence:old1", 4, "Yes", 5)]);
    let plan = build(
        &snap,
        &[NewNode {
            id: "doc:evidence:new1".to_string(),
            page: Some(4),
            verbatim_quote: "Yes.".to_string(),
            question: Some("Did you sign it?".to_string()),
        }],
    );

    assert!(matches!(
        plan.nodes[0].outcome,
        Match::Unambiguous {
            tier: MatchTier::Normalized,
            ..
        }
    ));
}

#[test]
fn two_old_nodes_near_matching_one_new_node_are_both_demoted() {
    // The cross-tier collision the per-tier twin guards cannot see. Left alone,
    // this becomes two MAP lines onto one id — which `apply` refuses outright
    // with a duplicate-id error, turning a decidable case into a dead end.
    let snap = snapshot(vec![
        transcript_old(
            "doc:evidence:oldA",
            9,
            "The fee petition was filed in April of that year",
            10,
        ),
        transcript_old(
            "doc:evidence:oldB",
            9,
            "The fee petition was filed in April of that year!",
            8,
        ),
    ]);
    let plan = build(
        &snap,
        &[transcript_new(
            "doc:evidence:new1",
            9,
            "The fee petition was filed in April of that year?",
        )],
    );

    for node in &plan.nodes {
        assert!(
            matches!(node.outcome, Match::Ambiguous { .. }),
            "{} should be ambiguous, got {:?}",
            node.old.id,
            node.outcome
        );
    }
    assert!(plan.auto_moves().is_empty());
    assert_eq!(plan.totals().curated_rows_at_risk, 18);
}

#[test]
fn a_near_match_onto_a_node_that_already_survived_is_demoted() {
    // oldA kept its id, so new1 already has an owner. oldB near-matching it would
    // put oldB's rows on oldA's node.
    let snap = snapshot(vec![
        transcript_old("doc:evidence:new1", 9, "The fee petition was filed.", 3),
        transcript_old("doc:evidence:oldB", 9, "The fee petition was filed", 7),
    ]);
    let plan = build(
        &snap,
        &[transcript_new(
            "doc:evidence:new1",
            9,
            "The fee petition was filed.",
        )],
    );

    assert_eq!(plan.nodes[0].outcome, Match::Unchanged);
    assert!(
        matches!(plan.nodes[1].outcome, Match::Ambiguous { .. }),
        "got {:?}",
        plan.nodes[1].outcome
    );
    assert!(plan.auto_moves().is_empty());
}

#[test]
fn a_near_match_on_another_page_is_never_considered() {
    // Every tier below tier 1 is page-local. The same boilerplate sentence
    // appears on many pages of a transcript.
    let snap = snapshot(vec![transcript_old(
        "doc:evidence:old1",
        8,
        "Thank you, your Honor",
        4,
    )]);
    let plan = build(
        &snap,
        &[transcript_new(
            "doc:evidence:new1",
            9,
            "Thank you, your Honor.",
        )],
    );

    assert_eq!(plan.nodes[0].outcome, Match::Unmatched);
}
