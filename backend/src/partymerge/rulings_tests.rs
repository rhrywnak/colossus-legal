//! The rulings file is the only authority this tool obeys, so every way of
//! misreading it is tested. A lenient parser here would merge people.

use super::*;

const TIGHE: &str = "\
# Roman's merge session, 2026-08-16
CLUSTER Tighe — the judge, split across transcript and opinion
SURVIVOR person-karen-a-tighe
MERGE person-tighe
END

CLUSTER Humphrey
SKIP \"Jeff\" could equally be Jeff Sharp
END
";

#[test]
fn a_well_formed_file_parses_into_its_two_kinds_of_ruling() {
    let file = parse(TIGHE).expect("valid rulings");
    assert_eq!(file.clusters.len(), 2);

    assert_eq!(
        file.clusters[0].ruling,
        Ruling::Merge {
            survivor: "person-karen-a-tighe".to_string(),
            members: vec!["person-tighe".to_string()],
        }
    );
    assert_eq!(
        file.clusters[1].ruling,
        Ruling::Skip {
            reason: "\"Jeff\" could equally be Jeff Sharp".to_string(),
        }
    );
    assert_eq!(file.merges().count(), 1);
    assert_eq!(file.skips().count(), 1);
    assert_eq!(
        file.all_nodes(),
        vec![
            "person-karen-a-tighe".to_string(),
            "person-tighe".to_string()
        ],
        "a skipped cluster names no nodes — nothing about it is touched"
    );
}

#[test]
fn the_label_roman_wrote_is_kept_verbatim_for_the_report() {
    let file = parse(TIGHE).expect("valid rulings");
    assert_eq!(
        file.clusters[0].label,
        "Tighe — the judge, split across transcript and opinion"
    );
}

#[test]
fn a_multi_member_cluster_merges_every_member() {
    let text = "CLUSTER Sharp\nSURVIVOR person-jeffrey-sharp\nMERGE person-sharp\nMERGE person-shaw\nEND\n";
    let file = parse(text).expect("valid rulings");
    assert_eq!(
        file.clusters[0].ruling,
        Ruling::Merge {
            survivor: "person-jeffrey-sharp".to_string(),
            members: vec!["person-sharp".to_string(), "person-shaw".to_string()],
        }
    );
}

#[test]
fn comments_and_blank_lines_are_ignored_including_trailing_ones() {
    let text = "# header\n\nCLUSTER A  # this is the judge\nSURVIVOR p-a # keep this one\nMERGE p-b\nEND\n";
    let file = parse(text).expect("valid rulings");
    assert_eq!(
        file.clusters[0].ruling,
        Ruling::Merge {
            survivor: "p-a".to_string(),
            members: vec!["p-b".to_string()],
        }
    );
    assert_eq!(file.clusters[0].label, "A");
}

#[test]
fn an_empty_file_is_refused_rather_than_read_as_merge_nothing() {
    // An unedited template and a deliberate "merge nothing" must not look the
    // same to the tool.
    assert_eq!(parse("").unwrap_err(), RulingsError::Empty);
    assert_eq!(
        parse("# only comments\n\n").unwrap_err(),
        RulingsError::Empty
    );
}

#[test]
fn merge_lines_without_a_survivor_are_refused() {
    let err = parse("CLUSTER A\nMERGE p-b\nEND\n").unwrap_err();
    match err {
        RulingsError::Syntax { line, message } => {
            assert_eq!(line, 3);
            assert!(message.contains("no SURVIVOR"), "got: {message}");
        }
        other => panic!("expected a syntax error, got {other:?}"),
    }
}

#[test]
fn a_survivor_with_nothing_to_merge_is_refused() {
    // Ambiguous between "I meant to add members" and "I meant SKIP", and the
    // tool does not guess which.
    let err = parse("CLUSTER A\nSURVIVOR p-a\nEND\n").unwrap_err();
    assert!(matches!(err, RulingsError::Syntax { .. }));
}

#[test]
fn skip_mixed_with_a_merge_instruction_is_refused() {
    let err = parse("CLUSTER A\nSKIP unsure\nSURVIVOR p-a\nMERGE p-b\nEND\n").unwrap_err();
    match err {
        RulingsError::Syntax { message, .. } => {
            assert!(message.contains("mixes SKIP"), "got: {message}")
        }
        other => panic!("expected a syntax error, got {other:?}"),
    }
}

#[test]
fn a_skip_without_a_reason_is_refused() {
    // The reason is the record of WHY a cluster stayed split. Losing it means
    // the next session re-derives that "Jeff" is ambiguous.
    let err = parse("CLUSTER A\nSKIP\nEND\n").unwrap_err();
    match err {
        RulingsError::Syntax { line, message } => {
            assert_eq!(line, 2);
            assert!(message.contains("SKIP needs a value"), "got: {message}");
        }
        other => panic!("expected a syntax error, got {other:?}"),
    }
}

#[test]
fn two_survivors_in_one_cluster_are_refused() {
    let err = parse("CLUSTER A\nSURVIVOR p-a\nSURVIVOR p-c\nMERGE p-b\nEND\n").unwrap_err();
    match err {
        RulingsError::Syntax { line, message } => {
            assert_eq!(line, 3);
            assert!(message.contains("a second SURVIVOR"), "got: {message}");
        }
        other => panic!("expected a syntax error, got {other:?}"),
    }
}

#[test]
fn a_survivor_listed_as_its_own_member_is_refused() {
    // Would delete the survivor out from under itself.
    let err = parse("CLUSTER A\nSURVIVOR p-a\nMERGE p-a\nEND\n").unwrap_err();
    match err {
        RulingsError::Syntax { message, .. } => {
            assert!(message.contains("survivor"), "got: {message}")
        }
        other => panic!("expected a syntax error, got {other:?}"),
    }
}

#[test]
fn a_node_named_in_two_clusters_is_refused_with_both_line_numbers() {
    let text = "CLUSTER A\nSURVIVOR p-a\nMERGE p-x\nEND\nCLUSTER B\nSURVIVOR p-b\nMERGE p-x\nEND\n";
    match parse(text).unwrap_err() {
        RulingsError::DuplicateNode {
            node,
            first,
            second,
        } => {
            assert_eq!(node, "p-x");
            assert_eq!((first, second), (1, 5));
        }
        other => panic!("expected a duplicate-node error, got {other:?}"),
    }
}

#[test]
fn a_survivor_that_is_another_clusters_member_is_refused() {
    let text = "CLUSTER A\nSURVIVOR p-a\nMERGE p-x\nEND\nCLUSTER B\nSURVIVOR p-x\nMERGE p-y\nEND\n";
    assert!(matches!(
        parse(text).unwrap_err(),
        RulingsError::DuplicateNode { .. }
    ));
}

#[test]
fn an_unclosed_cluster_is_refused_and_names_its_opening_line() {
    let err = parse("CLUSTER A\nSURVIVOR p-a\nMERGE p-b\n").unwrap_err();
    match err {
        RulingsError::Syntax { line, message } => {
            assert_eq!(line, 1);
            assert!(message.contains("never closed"), "got: {message}");
        }
        other => panic!("expected a syntax error, got {other:?}"),
    }
}

#[test]
fn a_nested_cluster_is_refused() {
    let err = parse("CLUSTER A\nSURVIVOR p-a\nCLUSTER B\n").unwrap_err();
    match err {
        RulingsError::Syntax { line, message } => {
            assert_eq!(line, 3);
            assert!(message.contains("still-open cluster"), "got: {message}");
        }
        other => panic!("expected a syntax error, got {other:?}"),
    }
}

#[test]
fn an_unknown_directive_is_refused_rather_than_ignored() {
    // A silently ignored line is a decision that did not happen.
    let err = parse("CLUSTER A\nSURVIVOR p-a\nDELETE p-b\nEND\n").unwrap_err();
    match err {
        RulingsError::Syntax { line, message } => {
            assert_eq!(line, 3);
            assert!(
                message.contains("unknown directive 'DELETE'"),
                "got: {message}"
            );
        }
        other => panic!("expected a syntax error, got {other:?}"),
    }
}

#[test]
fn a_directive_outside_any_block_is_refused() {
    let err = parse("SURVIVOR p-a\n").unwrap_err();
    match err {
        RulingsError::Syntax { message, .. } => {
            assert!(message.contains("outside any CLUSTER"), "got: {message}")
        }
        other => panic!("expected a syntax error, got {other:?}"),
    }
}

#[test]
fn an_end_without_a_cluster_is_refused() {
    let err = parse("END\n").unwrap_err();
    match err {
        RulingsError::Syntax { line, message } => {
            assert_eq!(line, 1);
            assert!(message.contains("END without"), "got: {message}");
        }
        other => panic!("expected a syntax error, got {other:?}"),
    }
}

#[test]
fn the_summary_counts_what_the_report_header_claims() {
    let file = parse(TIGHE).expect("valid rulings");
    let summary = file.summary();
    assert!(summary.contains("2 cluster(s) ruled"));
    assert!(summary.contains("1 to merge"));
    assert!(summary.contains("1 node(s) merging in"));
    assert!(summary.contains("1 skipped"));
}
