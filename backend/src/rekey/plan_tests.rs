// Tests for `rekey::plan`.
//
// This is the half of the re-key that DECIDES, so these tests are the ones that
// stand between Roman's 947 curated rows and a bad write. The measured shape of
// the live corpus — 525 nodes, 483 unique keys, 21 twin pairs, 0 triples — is
// reproduced in miniature rather than asserted in the abstract.

use super::*;

const DOC_A: &str = "doc-george-phillips-response-to-discovery";
const DOC_B: &str = "doc-cfs-interrogatory-response-08-08-16";

fn row(current_id: &str, doc: &str, page: i64, quote: &str, question: Option<&str>) -> EvidenceRow {
    EvidenceRow {
        current_id: current_id.to_string(),
        doc_slug: doc.to_string(),
        page: Some(page),
        verbatim_quote: quote.to_string(),
        question: question.map(str::to_string),
    }
}

/// A node whose stored id is the OLD blob-hash form gets re-keyed.
#[test]
fn a_stale_id_is_planned_for_rekey() {
    let plan = RekeyPlan::build(vec![row(
        "doc-george-phillips-response-to-discovery:evidence:deadbeef",
        DOC_A,
        16,
        "Yes.",
        Some("Did you sign it?"),
    )]);
    let node = plan.nodes().next().expect("one node");
    let target = node.rekey_target().expect("planned for re-key");
    assert!(target.starts_with(&format!("{DOC_A}:evidence:")));
    assert_ne!(target, node.row.current_id);
    assert_eq!(
        plan.totals(),
        PlanTotals {
            nodes_seen: 1,
            to_rekey: 1,
            already_current: 0,
            refused_shared_key: 0
        }
    );
}

/// IDEMPOTENCY, which the ruling requires so a partial run resumes safely.
///
/// Built by re-keying once and feeding the result back in — rather than
/// hand-writing an expected id, which would only prove the test author can run
/// the hash function.
#[test]
fn a_second_run_over_completed_work_plans_nothing() {
    let first = RekeyPlan::build(vec![row(
        "doc-a:evidence:stale111",
        DOC_A,
        16,
        "Yes.",
        Some("Did you sign it?"),
    )]);
    let new_id = first
        .nodes()
        .next()
        .and_then(PlannedNode::rekey_target)
        .expect("planned")
        .to_string();

    let second = RekeyPlan::build(vec![row(
        &new_id,
        DOC_A,
        16,
        "Yes.",
        Some("Did you sign it?"),
    )]);
    assert_eq!(
        second.totals(),
        PlanTotals {
            nodes_seen: 1,
            to_rekey: 0,
            already_current: 1,
            refused_shared_key: 0
        },
        "a completed re-key must plan no work on the second run",
    );
}

/// THE RULING, as an assertion: two nodes sharing a key are BOTH refused and
/// BOTH keep the ids they have.
///
/// Seven such pairs on DEV carry curated rows on both twins and three carry
/// conflicting weights. If either twin were re-keyed, the curated rows pointing
/// at it would move to a node the other twin's rows do not — which is how a
/// `carries` and a `backup` swap places in one scenario.
#[test]
fn twins_are_both_refused_and_neither_moves() {
    let plan = RekeyPlan::build(vec![
        row(
            "doc-a:evidence:twin0001",
            DOC_A,
            9,
            "Correspondence was received.",
            Some("Did you receive it?"),
        ),
        row(
            "doc-a:evidence:twin0002",
            DOC_A,
            9,
            "Correspondence was received.",
            Some("Did you receive it?"),
        ),
    ]);
    assert_eq!(
        plan.totals(),
        PlanTotals {
            nodes_seen: 2,
            to_rekey: 0,
            already_current: 0,
            refused_shared_key: 2
        },
    );
    for node in plan.nodes() {
        assert!(node.rekey_target().is_none(), "a twin must not be re-keyed");
        match &node.disposition {
            Disposition::RefusedSharedKey { key_holders, .. } => assert_eq!(*key_holders, 2),
            other => panic!("expected a refusal, got {other:?}"),
        }
    }
}

/// Whitespace and case differences do not save a twin from refusal — the key is
/// normalized, so "the same statement typed two ways" is still one key.
#[test]
fn normalization_does_not_let_a_twin_slip_through_as_unique() {
    let plan = RekeyPlan::build(vec![
        row(
            "doc-a:evidence:t1",
            DOC_A,
            9,
            "It  was\n received.",
            Some("Q?"),
        ),
        row(
            "doc-a:evidence:t2",
            DOC_A,
            9,
            "It was received.",
            Some("Q?"),
        ),
    ]);
    assert_eq!(plan.totals().refused_shared_key, 2);
}

/// The refusal enumeration the completion report owes: grouped by shared key,
/// with both current ids, in stable order.
#[test]
fn refused_groups_enumerate_each_pair_once() {
    let plan = RekeyPlan::build(vec![
        row("doc-a:evidence:bbb", DOC_A, 9, "Same words.", None),
        row("doc-a:evidence:aaa", DOC_A, 9, "Same words.", None),
        row("doc-a:evidence:ccc", DOC_A, 10, "Different words.", None),
    ]);
    let groups = plan.refused_groups();
    assert_eq!(groups.len(), 1, "one shared key");
    assert_eq!(
        groups[0].1,
        vec![
            "doc-a:evidence:aaa".to_string(),
            "doc-a:evidence:bbb".to_string()
        ],
        "both twins listed, sorted",
    );
}

/// The unit of work is a document, and the grouping must actually group.
#[test]
fn nodes_are_grouped_by_document() {
    let plan = RekeyPlan::build(vec![
        row("id-1", DOC_A, 1, "A.", None),
        row("id-2", DOC_B, 1, "B.", None),
        row("id-3", DOC_A, 2, "C.", None),
    ]);
    assert_eq!(plan.by_document.len(), 2);
    assert_eq!(plan.by_document[DOC_A].len(), 2);
    assert_eq!(plan.by_document[DOC_B].len(), 1);
}

/// Two dry-runs of identical data must produce identical reports, or an operator
/// cannot diff them. Ordering is by document then current id, both stable.
#[test]
fn the_plan_order_is_stable_across_input_order() {
    let a = RekeyPlan::build(vec![
        row("id-3", DOC_B, 1, "C.", None),
        row("id-1", DOC_A, 1, "A.", None),
        row("id-2", DOC_A, 2, "B.", None),
    ]);
    let b = RekeyPlan::build(vec![
        row("id-2", DOC_A, 2, "B.", None),
        row("id-3", DOC_B, 1, "C.", None),
        row("id-1", DOC_A, 1, "A.", None),
    ]);
    let ids =
        |p: &RekeyPlan| -> Vec<String> { p.nodes().map(|n| n.row.current_id.clone()).collect() };
    assert_eq!(
        ids(&a),
        ids(&b),
        "the plan order must not depend on input order"
    );
    // Documents come out in slug order — `doc-cfs…` before `doc-george…` — and
    // within a document, by current id. So DOC_B's single node leads. Spelled out
    // rather than left implicit, because "stable" and "the order I happened to
    // pass in" are easy to confuse when reading a failure.
    assert_eq!(ids(&a), vec!["id-3", "id-1", "id-2"]);
}

/// A collision that spans two DOCUMENTS is still a collision.
///
/// Every measured pair is within one document, but nothing guarantees that, and
/// planning per document would miss this. It cannot happen while `doc_slug` is
/// in the key — which is exactly why this test exists: it fails the day somebody
/// takes the document out of the key.
#[test]
fn a_cross_document_collision_would_be_caught() {
    // Same page, quote and question in two documents — distinct because the doc
    // slug is part of the key.
    let plan = RekeyPlan::build(vec![
        row("id-a", DOC_A, 3, "Identical text.", None),
        row("id-b", DOC_B, 3, "Identical text.", None),
    ]);
    assert_eq!(
        plan.totals().refused_shared_key,
        0,
        "the document slug must keep these apart",
    );
    assert_eq!(plan.totals().to_rekey, 2);
}

/// The safety net: no node may end the run wearing an id another node also wears.
#[test]
fn a_clean_plan_reports_no_target_conflicts() {
    let plan = RekeyPlan::build(vec![
        row("id-1", DOC_A, 1, "A.", None),
        row("id-2", DOC_A, 2, "B.", None),
    ]);
    assert!(plan.target_conflicts().is_empty());
}

/// A node NOT being re-keyed still holds its id, and a re-key target must not
/// land on it.
///
/// This is the digest-collision guard. Constructed by pointing a stale node at a
/// target that a refused twin already occupies — the only way to build the case
/// without brute-forcing an 8-hex collision.
#[test]
fn a_target_landing_on_a_retained_id_is_reported_as_a_conflict() {
    // Two twins share a key and are refused, keeping ids T1 and T2.
    let twin_a = row("doc-a:evidence:t1", DOC_A, 9, "Shared.", None);
    let twin_b = row("doc-a:evidence:t2", DOC_A, 9, "Shared.", None);
    // A third node is re-keyed. Compute where it lands, then re-run with a twin
    // already sitting on that id.
    let probe = RekeyPlan::build(vec![row("stale", DOC_A, 4, "Unique.", None)]);
    let landing = probe
        .nodes()
        .next()
        .and_then(PlannedNode::rekey_target)
        .expect("planned")
        .to_string();

    let mut squatter = twin_a.clone();
    squatter.current_id = landing.clone();
    let plan = RekeyPlan::build(vec![
        squatter,
        twin_b,
        row("stale", DOC_A, 4, "Unique.", None),
    ]);

    let conflicts = plan.target_conflicts();
    assert_eq!(conflicts.len(), 1, "the landing collision must be reported");
    assert_eq!(conflicts[0].0, landing);
    assert_eq!(conflicts[0].1.len(), 2, "two nodes claim one id");
}

#[test]
fn an_empty_graph_plans_nothing_and_does_not_panic() {
    let plan = RekeyPlan::build(vec![]);
    assert_eq!(plan.totals(), PlanTotals::default());
    assert!(plan.refused_groups().is_empty());
    assert!(plan.target_conflicts().is_empty());
}
