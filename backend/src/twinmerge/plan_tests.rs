//! The merge's decisions, asserted with specific inputs and specific outputs.
//!
//! Every case here mirrors a shape measured on DEV 2026-08-15: 14 clean pairs,
//! 7 pairs curated on both sides, 1 pair whose edge sets diverge (and which is
//! also one of the 7). Nothing touches a database — that is the whole point of
//! keeping the decisions in this module.

use super::*;
use crate::rekey::plan::EvidenceRow;

/// A node in the shape the twins actually take: same document, same page, same
/// quote, same question — only the id differs.
fn twin(id: &str, curated_rows: u64, relationships: &[&str]) -> TwinNode {
    TwinNode {
        row: EvidenceRow {
            current_id: id.to_string(),
            doc_slug: "doc-george-phillips-response-to-discovery".to_string(),
            page: Some(16),
            verbatim_quote: "Yes.".to_string(),
            question: Some("Do you admit the claim?".to_string()),
        },
        curated_rows,
        relationships: relationships.iter().map(|s| s.to_string()).collect(),
    }
}

/// A node that is nobody's twin — different quote, so a different key.
fn singleton(id: &str) -> TwinNode {
    TwinNode {
        row: EvidenceRow {
            current_id: id.to_string(),
            doc_slug: "doc-george-phillips-response-to-discovery".to_string(),
            page: Some(4),
            verbatim_quote: "Not that I recall.".to_string(),
            question: None,
        },
        curated_rows: 0,
        relationships: vec!["STATED_BY->person-george-phillips".to_string()],
    }
}

const EDGES: [&str; 2] = ["CONTAINED_IN->doc-x", "STATED_BY->person-george-phillips"];

#[test]
fn a_node_with_a_unique_key_is_not_a_cluster_at_all() {
    let plan = TwinPlan::build(vec![singleton("a"), twin("b", 0, &EDGES)]);
    assert_eq!(
        plan.clusters.len(),
        0,
        "two nodes with different keys are two statements, not a twin pair"
    );
    assert_eq!(plan.totals().nodes_seen, 0);
}

#[test]
fn an_uncurated_pair_with_matching_edges_merges() {
    let plan = TwinPlan::build(vec![twin("zzz", 0, &EDGES), twin("aaa", 0, &EDGES)]);
    assert_eq!(plan.clusters.len(), 1);

    match &plan.clusters[0].disposition {
        Disposition::Merge {
            survivor,
            losers,
            target_id,
        } => {
            assert_eq!(
                survivor, "aaa",
                "with neither node curated the survivor is the lexicographically \
                 smallest id — deterministic across runs and hosts"
            );
            assert_eq!(losers, &vec!["zzz".to_string()]);
            assert_eq!(
                target_id, &plan.clusters[0].key,
                "the survivor takes the cluster's stable-arm key, which is the \
                 whole point: afterwards the key has one holder"
            );
        }
        other => panic!("expected a merge, got {other:?}"),
    }

    let t = plan.totals();
    assert_eq!(t.clusters_to_merge, 1);
    assert_eq!(t.nodes_to_delete, 1);
    assert_eq!(t.nodes_seen, 2);
}

#[test]
fn the_curated_member_survives_so_no_ruling_has_to_move() {
    // "zzz" would win on id order; curation overrides that, because leaving
    // Roman's rows where they are is strictly less work than moving them.
    let plan = TwinPlan::build(vec![twin("aaa", 0, &EDGES), twin("zzz", 9, &EDGES)]);
    match &plan.clusters[0].disposition {
        Disposition::Merge {
            survivor, losers, ..
        } => {
            assert_eq!(survivor, "zzz");
            assert_eq!(losers, &vec!["aaa".to_string()]);
        }
        other => panic!("expected a merge, got {other:?}"),
    }
}

#[test]
fn a_pair_curated_on_both_sides_is_refused_with_its_rulings_named() {
    // The measured case: 7 of 21 pairs, three of them carrying conflicting
    // weights. No program gets to pick which of Roman's two rulings survives.
    let plan = TwinPlan::build(vec![
        twin("98515eda", 12, &EDGES),
        twin("f1439b2c", 9, &EDGES),
    ]);

    match &plan.clusters[0].disposition {
        Disposition::RefusedMultipleCurated { curated } => {
            assert_eq!(
                curated,
                &vec![
                    ("98515eda".to_string(), 12u64),
                    ("f1439b2c".to_string(), 9u64),
                ],
                "the queue entry must carry the row counts, so the merge session \
                 can see what is at stake without another query"
            );
        }
        other => panic!("expected a curated refusal, got {other:?}"),
    }
    assert_eq!(plan.totals().clusters_refused_curated, 1);
    assert_eq!(plan.totals().nodes_to_delete, 0);
}

#[test]
fn a_loser_edge_the_survivor_lacks_refuses_rather_than_dropping_it() {
    let survivor_edges = ["CONTAINED_IN->doc-x"];
    let loser_edges = ["CONTAINED_IN->doc-x", "ABOUT->person-marie-awad"];
    let plan = TwinPlan::build(vec![
        twin("aaa", 0, &survivor_edges),
        twin("bbb", 0, &loser_edges),
    ]);

    match &plan.clusters[0].disposition {
        Disposition::RefusedEdgeDivergence {
            survivor,
            extra_edges,
        } => {
            assert_eq!(survivor, "aaa");
            assert_eq!(
                extra_edges,
                &vec![(
                    "bbb".to_string(),
                    vec!["ABOUT->person-marie-awad".to_string()]
                )],
                "deleting bbb would lose the ABOUT edge; the tool names it \
                 instead of losing it"
            );
        }
        other => panic!("expected an edge-divergence refusal, got {other:?}"),
    }
    assert_eq!(plan.totals().clusters_refused_edges, 1);
}

#[test]
fn a_survivor_edge_the_loser_lacks_is_not_a_divergence() {
    // Only the LOSER's edges can be lost by a delete. The survivor knowing more
    // than the loser costs nothing, and refusing on it would refuse merges that
    // are perfectly safe.
    let plan = TwinPlan::build(vec![
        twin(
            "aaa",
            0,
            &["CONTAINED_IN->doc-x", "ABOUT->person-marie-awad"],
        ),
        twin("bbb", 0, &["CONTAINED_IN->doc-x"]),
    ]);
    assert!(plan.clusters[0].is_merge(), "expected a merge");
}

#[test]
fn curated_refusal_wins_over_edge_divergence() {
    // The measured pair 042d8287/be12ddef is both. Roman needs to see it as a
    // ruling conflict — the thing only he can settle — not as a mechanical
    // edge difference he then has to look past.
    let plan = TwinPlan::build(vec![
        twin("042d8287", 10, &["CONTAINED_IN->doc-x"]),
        twin(
            "be12ddef",
            9,
            &["CONTAINED_IN->doc-x", "ABOUT->person-marie-awad"],
        ),
    ]);
    assert!(matches!(
        plan.clusters[0].disposition,
        Disposition::RefusedMultipleCurated { .. }
    ));
}

#[test]
fn a_merged_cluster_does_not_reappear_on_the_next_run() {
    // Idempotency, stated as the ruling requires. After the merge the key has
    // one holder, and a single node is not a cluster — so a second run plans
    // nothing, which is how an operator can tell from a dry run alone that the
    // work has already happened.
    let after_merge = TwinPlan::build(vec![twin("aaa", 0, &EDGES)]);
    assert_eq!(after_merge.clusters.len(), 0);
    assert_eq!(after_merge.totals().clusters_to_merge, 0);
}

#[test]
fn a_target_already_held_by_an_outsider_is_an_unsafe_plan() {
    let plan = TwinPlan::build(vec![twin("aaa", 0, &EDGES), twin("bbb", 0, &EDGES)]);
    let target = plan.clusters[0].key.clone();

    let safe = plan.target_conflicts(&["aaa".to_string(), "bbb".to_string()]);
    assert!(safe.is_empty(), "members holding the target is normal");

    let all_ids = vec!["aaa".to_string(), "bbb".to_string(), target.clone()];
    let conflicts = plan.target_conflicts(&all_ids);
    assert_eq!(
        conflicts,
        vec![(target.clone(), target)],
        "an unrelated node already carrying the target id would be welded to \
         this statement — refuse before writing anything"
    );
}

#[test]
fn clusters_are_ordered_the_same_way_on_every_run() {
    // Two dry runs of unchanged data must diff cleanly, so ordering cannot come
    // from the order the graph happened to return rows in.
    let forward = TwinPlan::build(vec![twin("aaa", 0, &EDGES), twin("bbb", 0, &EDGES)]);
    let reverse = TwinPlan::build(vec![twin("bbb", 0, &EDGES), twin("aaa", 0, &EDGES)]);

    let ids = |p: &TwinPlan| -> Vec<String> {
        p.clusters
            .iter()
            .flat_map(|c| c.members.iter().map(|m| m.id().to_string()))
            .collect()
    };
    assert_eq!(ids(&forward), ids(&reverse));
    assert_eq!(
        forward.clusters[0].disposition,
        reverse.clusters[0].disposition
    );
}
