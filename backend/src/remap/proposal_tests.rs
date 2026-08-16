//! The proposal is the approval gate. If it can be executed unread, the
//! three-step is a two-step with a ceremony in the middle.

use super::*;
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
    RemapPlan::build(&snapshot, &new_nodes)
}

#[test]
fn a_generated_proposal_carries_its_approval_line_commented_out() {
    let rendered = render(&plan());
    assert!(
        rendered.contains("# APPROVED your-name-here"),
        "the approval line must be present and commented, so approving is \
         deleting a '#' on a file someone opened"
    );
    assert!(
        !rendered.contains("\nAPPROVED "),
        "a generated proposal must never be pre-approved"
    );
    assert!(rendered.contains("MAP doc:evidence:old1 doc:evidence:new1"));
    assert!(rendered.contains("DOCUMENT doc-sabrina-morris-affidavit"));
}

#[test]
fn a_generated_proposal_states_the_yield_the_gate_test_checks() {
    let rendered = render(&plan());
    assert!(rendered.contains("1 unambiguous"));
    assert!(rendered.contains("1 unmatched"));
    assert!(rendered.contains("Yield (unchanged + unambiguous): 50.0%"));
    assert!(rendered.contains("Curated rows at risk in the queue: 12"));
}

#[test]
fn a_generated_proposal_parses_but_refuses_to_apply_until_approved() {
    // The load-bearing test of the whole design.
    let parsed = parse(&render(&plan())).expect("the generated file parses");
    assert_eq!(parsed.approved_by, None);
    assert_eq!(
        parsed.approved_moves().unwrap_err(),
        ProposalError::NotApproved
    );
}

#[test]
fn uncommenting_the_approval_line_is_all_it_takes() {
    let approved = render(&plan()).replace("# APPROVED your-name-here", "APPROVED Roman");
    let parsed = parse(&approved).expect("parses");
    assert_eq!(parsed.approved_by.as_deref(), Some("Roman"));
    assert_eq!(
        parsed.approved_moves().expect("approved"),
        &[(
            "doc:evidence:old1".to_string(),
            "doc:evidence:new1".to_string()
        )]
    );
}

#[test]
fn deleting_a_map_line_un_approves_that_move() {
    let text = "DOCUMENT d\nAPPROVED Roman\nMAP a1 b1\nMAP a2 b2\n";
    let both = parse(text).expect("parses");
    assert_eq!(both.moves.len(), 2);

    let one = parse("DOCUMENT d\nAPPROVED Roman\nMAP a1 b1\n").expect("parses");
    assert_eq!(one.moves.len(), 1);
}

#[test]
fn a_proposal_with_every_map_deleted_is_refused_rather_than_run_as_a_no_op() {
    let err = parse("DOCUMENT d\nAPPROVED Roman\n").unwrap_err();
    assert_eq!(err, ProposalError::NoMoves);
}

#[test]
fn a_proposal_naming_no_document_is_refused() {
    assert_eq!(
        parse("APPROVED Roman\nMAP a1 b1\n").unwrap_err(),
        ProposalError::NoDocument
    );
}

#[test]
fn an_approval_without_a_name_is_refused() {
    match parse("DOCUMENT d\nAPPROVED\nMAP a1 b1\n").unwrap_err() {
        ProposalError::Syntax { line, message } => {
            assert_eq!(line, 2);
            assert!(message.contains("needs a name"), "got: {message}");
        }
        other => panic!("expected a syntax error, got {other:?}"),
    }
}

#[test]
fn a_map_line_with_the_wrong_number_of_ids_is_refused() {
    match parse("DOCUMENT d\nAPPROVED R\nMAP a1\n").unwrap_err() {
        ProposalError::Syntax { line, message } => {
            assert_eq!(line, 3);
            assert!(message.contains("exactly two ids"), "got: {message}");
        }
        other => panic!("expected a syntax error, got {other:?}"),
    }
}

#[test]
fn an_unknown_directive_is_refused_rather_than_ignored() {
    match parse("DOCUMENT d\nAPPROVED R\nDELETE a1\nMAP a1 b1\n").unwrap_err() {
        ProposalError::Syntax { message, .. } => {
            assert!(
                message.contains("unknown directive 'DELETE'"),
                "got: {message}"
            )
        }
        other => panic!("expected a syntax error, got {other:?}"),
    }
}

#[test]
fn the_same_old_id_mapped_twice_is_refused() {
    // Two different moves applied to one set of rows: the second wins silently.
    let err = parse("DOCUMENT d\nAPPROVED R\nMAP a1 b1\nMAP a1 b2\n").unwrap_err();
    assert!(matches!(err, ProposalError::Syntax { .. }));
}

#[test]
fn two_old_ids_mapped_onto_one_new_id_are_refused() {
    // Would collapse two statements' rulings onto one node — the exact failure
    // the twin merge exists to prevent a program from committing.
    let err = parse("DOCUMENT d\nAPPROVED R\nMAP a1 b1\nMAP a2 b1\n").unwrap_err();
    assert!(matches!(err, ProposalError::Syntax { .. }));
}

#[test]
fn the_queue_names_the_orphan_and_what_it_would_cost() {
    let queue = render_queue(&plan());
    assert!(queue.contains("doc:evidence:orphan  (12 curated row(s))"));
    assert!(queue.contains("UNMATCHED"));
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
    let queue = render_queue(&RemapPlan::build(&snapshot, &new_nodes));

    assert!(queue.contains("AMBIGUOUS"));
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
    let queue = render_queue(&RemapPlan::build(&snapshot, &new_nodes));
    assert!(queue.contains("Nothing here needs a human"));
}
