//! Matching decisions, asserted with specific inputs and specific outputs. What
//! is auto-applied here moves Roman's rulings onto a different node, so every
//! shape that could be ambiguous is tested as ambiguous.

use super::*;

fn old(id: &str, page: i64, quote: &str, curated_rows: u64) -> SnapshotNode {
    SnapshotNode {
        id: id.to_string(),
        page: Some(page),
        verbatim_quote: quote.to_string(),
        question: Some("Do you admit the claim?".to_string()),
        curated_rows,
    }
}

fn new(id: &str, page: i64, quote: &str) -> NewNode {
    NewNode {
        id: id.to_string(),
        page: Some(page),
        verbatim_quote: quote.to_string(),
        question: Some("Do you admit the claim?".to_string()),
    }
}

fn snapshot(nodes: Vec<SnapshotNode>) -> Snapshot {
    Snapshot {
        document_id: "doc-sabrina-morris-affidavit".to_string(),
        taken_note: "before the Morris gate test".to_string(),
        nodes,
    }
}

#[test]
fn an_id_that_survived_the_reprocess_needs_no_remap() {
    // The stable-id arm's whole purpose. This must read as the best outcome, not
    // as an unmatched node.
    let snap = snapshot(vec![old("doc:evidence:aaaa", 4, "Yes.", 9)]);
    let plan = RemapPlan::build(&snap, &[new("doc:evidence:aaaa", 4, "Yes.")]);

    assert_eq!(plan.nodes[0].outcome, Match::Unchanged);
    assert_eq!(plan.auto_moves(), Vec::new());
    assert_eq!(plan.totals().unchanged, 1);
    assert_eq!(plan.totals().yield_percent(), 100.0);
}

#[test]
fn one_old_and_one_new_sharing_a_key_is_an_unambiguous_move() {
    let snap = snapshot(vec![old("doc:evidence:old1", 4, "Yes.", 9)]);
    let plan = RemapPlan::build(&snap, &[new("doc:evidence:new1", 4, "Yes.")]);

    assert_eq!(
        plan.nodes[0].outcome,
        Match::Unambiguous {
            new_id: "doc:evidence:new1".to_string()
        }
    );
    assert_eq!(
        plan.auto_moves(),
        vec![(
            "doc:evidence:old1".to_string(),
            "doc:evidence:new1".to_string()
        )]
    );
}

#[test]
fn whitespace_and_line_breaks_in_a_quote_do_not_break_a_match() {
    // OCR line wrapping is layout, not content — and the normalization is the
    // same function the stable-id arm uses, so the two cannot disagree about
    // what "the same quote" means.
    let snap = snapshot(vec![old(
        "doc:evidence:old1",
        4,
        "The claims  filed\nin probate.",
        3,
    )]);
    let plan = RemapPlan::build(
        &snap,
        &[new("doc:evidence:new1", 4, "The claims filed in probate.")],
    );

    assert!(matches!(plan.nodes[0].outcome, Match::Unambiguous { .. }));
}

#[test]
fn a_different_page_is_a_different_statement() {
    let snap = snapshot(vec![old("doc:evidence:old1", 4, "Yes.", 3)]);
    let plan = RemapPlan::build(&snap, &[new("doc:evidence:new1", 5, "Yes.")]);

    assert_eq!(plan.nodes[0].outcome, Match::Unmatched);
}

#[test]
fn two_new_nodes_on_one_key_are_ambiguous_and_never_auto_applied() {
    let snap = snapshot(vec![old("doc:evidence:old1", 4, "Yes.", 12)]);
    let plan = RemapPlan::build(
        &snap,
        &[
            new("doc:evidence:newA", 4, "Yes."),
            new("doc:evidence:newB", 4, "Yes."),
        ],
    );

    assert_eq!(
        plan.nodes[0].outcome,
        Match::Ambiguous {
            candidates: vec![
                "doc:evidence:newA".to_string(),
                "doc:evidence:newB".to_string()
            ]
        }
    );
    assert!(
        plan.auto_moves().is_empty(),
        "picking one of two candidates would move 12 curated rows onto a guess"
    );
}

#[test]
fn two_old_twins_on_one_key_are_ambiguous_even_with_a_single_candidate() {
    // The measured twin class: two old nodes, identical on every stable field.
    // One new node cannot receive both, and choosing which old node it inherits
    // from is the exact decision the twin merge refuses to make.
    let snap = snapshot(vec![
        old("doc:evidence:twinA", 4, "Yes.", 10),
        old("doc:evidence:twinB", 4, "Yes.", 8),
    ]);
    let plan = RemapPlan::build(&snap, &[new("doc:evidence:new1", 4, "Yes.")]);

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
fn an_unmatched_node_reports_the_curated_rows_it_would_orphan() {
    let snap = snapshot(vec![
        old("doc:evidence:old1", 4, "Gone from the new extraction.", 12),
        old("doc:evidence:old2", 5, "Also gone.", 0),
    ]);
    let plan = RemapPlan::build(&snap, &[]);

    let t = plan.totals();
    assert_eq!(t.unmatched, 2);
    assert_eq!(
        t.curated_rows_at_risk, 12,
        "an orphan with no curated rows costs nothing; the number that matters \
         is rows, not nodes"
    );
}

#[test]
fn the_queue_puts_the_most_curated_orphan_first() {
    let snap = snapshot(vec![
        old("doc:evidence:cheap", 4, "Nobody ruled this.", 0),
        old(
            "doc:evidence:costly",
            5,
            "Roman ruled this twelve times.",
            12,
        ),
    ]);
    let plan = RemapPlan::build(&snap, &[]);
    let queue = plan.queue();

    assert_eq!(queue.len(), 2);
    assert_eq!(queue[0].old.id, "doc:evidence:costly");
}

#[test]
fn the_yield_counts_survivors_and_clean_matches_together() {
    // The Morris gate test compares this against the measured 87.8% floor.
    let snap = snapshot(vec![
        old("doc:evidence:kept", 1, "A.", 1),
        old("doc:evidence:moved", 2, "B.", 1),
        old("doc:evidence:lost", 3, "C.", 1),
        old("doc:evidence:muddy1", 4, "D.", 1),
    ]);
    let plan = RemapPlan::build(
        &snap,
        &[
            new("doc:evidence:kept", 1, "A."),
            new("doc:evidence:newB", 2, "B."),
            new("doc:evidence:newD1", 4, "D."),
            new("doc:evidence:newD2", 4, "D."),
        ],
    );

    let t = plan.totals();
    assert_eq!(
        (t.unchanged, t.unambiguous, t.ambiguous, t.unmatched),
        (1, 1, 1, 1)
    );
    assert_eq!(t.yield_percent(), 50.0);
    assert_eq!(t.curated_rows_at_risk, 2);
}

#[test]
fn an_empty_document_yields_one_hundred_percent_rather_than_dividing_by_zero() {
    let plan = RemapPlan::build(&snapshot(Vec::new()), &[]);
    assert_eq!(plan.totals().yield_percent(), 100.0);
    assert!(plan.auto_moves().is_empty());
}

#[test]
fn a_snapshot_round_trips_through_json_unchanged() {
    // The snapshot is written before a reprocess and read after it, possibly on
    // a different day. If it does not round-trip, the remap is comparing against
    // something that is not what was there.
    let snap = snapshot(vec![old("doc:evidence:old1", 4, "Yes, \"quoted\".\n", 9)]);
    let json = serde_json::to_string(&snap).expect("serialises");
    let back: Snapshot = serde_json::from_str(&json).expect("deserialises");
    assert_eq!(back, snap);
    assert_eq!(back.curated_nodes(), 1);
}
