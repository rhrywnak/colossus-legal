//! The human queue — the list of decisions this tool refused to make.
//!
//! It is never executed, which is exactly why its wording matters: it is the
//! only place a node that could not be matched explains itself, and Roman
//! decides from it by hand. Split from `proposal_tests.rs` to keep both under
//! the 300-line rule.

use super::*;
use crate::remap::normalize::NearMatchSettings;
use crate::remap::plan::{NewNode, RemapPlan, Snapshot, SnapshotNode};

fn plan() -> RemapPlan {
    let snapshot = Snapshot {
        document_id: "doc-sabrina-morris-affidavit".to_string(),
        taken_note: "before the Morris gate test".to_string(),
        nodes: vec![
            SnapshotNode {
                id: "doc:evidence:old1".to_string(),
                page: Some(4),
                verbatim_quote: "Yes.".to_string(),
                question: Some("Do you admit?".to_string()),
                curated_rows: 9,
            },
            SnapshotNode {
                id: "doc:evidence:orphan".to_string(),
                page: Some(7),
                verbatim_quote: "Gone from the new extraction.".to_string(),
                question: None,
                curated_rows: 12,
            },
        ],
    };
    let new_nodes = vec![NewNode {
        id: "doc:evidence:new1".to_string(),
        page: Some(4),
        verbatim_quote: "Yes.".to_string(),
        question: Some("Do you admit?".to_string()),
    }];
    RemapPlan::build(&snapshot, &new_nodes, NearMatchSettings::default())
}

#[test]
fn the_queue_names_the_orphan_and_what_it_would_cost() {
    let queue = render_queue(&plan());
    assert!(queue.contains("doc:evidence:orphan  (12 curated row(s))"));
    assert!(queue.contains("UNMATCHED"));
    assert!(
        queue.contains("UNMATCHED — no new node on this page matched at any tier"),
        "the reason is the only thing telling Roman whether to look for a \
         counterpart or accept the orphan; got:\n{queue}"
    );
    assert!(queue.contains("Gone from the new extraction."));
}

#[test]
fn the_queue_lists_every_candidate_for_an_ambiguous_node() {
    let snapshot = Snapshot {
        document_id: "d".to_string(),
        taken_note: "n".to_string(),
        nodes: vec![SnapshotNode {
            id: "old1".to_string(),
            page: Some(4),
            verbatim_quote: "Yes.".to_string(),
            question: None,
            curated_rows: 3,
        }],
    };
    let new_nodes = vec![
        NewNode {
            id: "newA".to_string(),
            page: Some(4),
            verbatim_quote: "Yes.".to_string(),
            question: None,
        },
        NewNode {
            id: "newB".to_string(),
            page: Some(4),
            verbatim_quote: "Yes.".to_string(),
            question: None,
        },
    ];
    let queue = render_queue(&RemapPlan::build(
        &snapshot,
        &new_nodes,
        NearMatchSettings::default(),
    ));

    assert!(
        queue.contains("AMBIGUOUS at tier1-exact — candidates:"),
        "the queue has to say WHICH tier found the ambiguity; got:\n{queue}"
    );
    assert!(queue.contains("newA"));
    assert!(queue.contains("newB"));
}

#[test]
fn an_empty_queue_still_says_so_in_writing() {
    let snapshot = Snapshot {
        document_id: "d".to_string(),
        taken_note: "n".to_string(),
        nodes: vec![SnapshotNode {
            id: "kept".to_string(),
            page: Some(1),
            verbatim_quote: "A.".to_string(),
            question: None,
            curated_rows: 0,
        }],
    };
    let new_nodes = vec![NewNode {
        id: "kept".to_string(),
        page: Some(1),
        verbatim_quote: "A.".to_string(),
        question: None,
    }];
    let queue = render_queue(&RemapPlan::build(
        &snapshot,
        &new_nodes,
        NearMatchSettings::default(),
    ));
    assert!(queue.contains("Nothing here needs a human"));
}

#[test]
fn a_single_candidate_ambiguity_says_the_doubt_is_on_the_old_side() {
    // Two old twins, one new node. "Pick one of these" and "two of your nodes
    // want this one" are different decisions and the queue must not blur them.
    let snapshot = Snapshot {
        document_id: "d".to_string(),
        taken_note: "n".to_string(),
        nodes: vec![
            SnapshotNode {
                id: "twinA".to_string(),
                page: Some(4),
                verbatim_quote: "Yes.".to_string(),
                question: None,
                curated_rows: 5,
            },
            SnapshotNode {
                id: "twinB".to_string(),
                page: Some(4),
                verbatim_quote: "Yes.".to_string(),
                question: None,
                curated_rows: 3,
            },
        ],
    };
    let new_nodes = vec![NewNode {
        id: "new1".to_string(),
        page: Some(4),
        verbatim_quote: "Yes.".to_string(),
        question: None,
    }];
    let queue = render_queue(&RemapPlan::build(
        &snapshot,
        &new_nodes,
        NearMatchSettings::default(),
    ));

    assert!(
        queue.contains(
            "AMBIGUOUS at tier1-exact — one candidate, but another old node also claims it:"
        ),
        "got:\n{queue}"
    );
}
