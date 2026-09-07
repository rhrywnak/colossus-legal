//! The proposal is the approval gate. If it can be executed unread, the
//! three-step is a two-step with a ceremony in the middle.

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
fn every_map_line_is_preceded_by_the_evidence_behind_it() {
    // This comment line is what the human reads to decide whether to trust the
    // MAP under it. A mislabeled tier or a wrong score would be approved as
    // readily as a right one, so the exact rendering is pinned.
    let rendered = render(&plan());
    assert!(
        rendered.contains(
            "# tier1-exact · score 1.000 · page 4 · 9 curated row(s)\nMAP doc:evidence:old1 doc:evidence:new1"
        ),
        "got:\n{rendered}"
    );
}

#[test]
fn a_move_with_no_page_renders_a_dash_rather_than_an_empty_field() {
    // All 525 live Evidence nodes carry a page, but a blank column between two
    // separators would read as a rendering bug rather than as missing data.
    let snapshot = Snapshot {
        document_id: "d".to_string(),
        taken_note: "n".to_string(),
        nodes: vec![SnapshotNode {
            id: "old1".to_string(),
            page: None,
            verbatim_quote: "Yes.".to_string(),
            question: None,
            curated_rows: 2,
        }],
    };
    let new_nodes = vec![NewNode {
        id: "new1".to_string(),
        page: None,
        verbatim_quote: "Yes.".to_string(),
        question: None,
    }];
    let rendered = render(&RemapPlan::build(
        &snapshot,
        &new_nodes,
        NearMatchSettings::default(),
    ));

    assert!(
        rendered.contains("# tier1-exact · score 1.000 · page — · 2 curated row(s)"),
        "got:\n{rendered}"
    );
}

#[test]
fn a_generated_proposal_states_the_yield_the_gate_test_checks() {
    let rendered = render(&plan());
    assert!(rendered.contains("1 unambiguous"));
    assert!(rendered.contains("1 unmatched"));
    assert!(rendered.contains("Yield (unchanged + unambiguous): 50.0%"));
    assert!(rendered.contains("Curated rows at risk in the queue: 12"));
    assert!(
        rendered
            .contains("# Of the 1 unambiguous: 1 tier1-exact · 0 tier2-normalized · 0 tier3-near"),
        "a yield that rests on tier 3 is a different run from one that rests on \
         tier 1, and the header has to say which; got:\n{rendered}"
    );
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
