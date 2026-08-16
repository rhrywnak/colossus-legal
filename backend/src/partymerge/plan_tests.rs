//! Where Roman's rulings meet the live graph. Each case here is a way the two
//! can disagree, and each has exactly one observable outcome.

use super::*;
use crate::partymerge::rulings::parse;

fn party(id: &str, label: &str, name: &str, statements: u64) -> PartyNode {
    PartyNode {
        id: id.to_string(),
        label: label.to_string(),
        display_name: name.to_string(),
        statement_count: statements,
        source_documents: vec!["doc-x".to_string()],
        aliases: Vec::new(),
    }
}

/// The measured census, trimmed to what these tests need.
fn census() -> Vec<PartyNode> {
    vec![
        party("person-karen-a-tighe", "Person", "Karen A. Tighe", 39),
        party("person-tighe", "Person", "Tighe", 62),
        party("person-jeffrey-humphrey", "Person", "Jeffrey Humphrey", 26),
        party(
            "org-catholic-family-services",
            "Organization",
            "Catholic Family Services",
            107,
        ),
    ]
}

const TIGHE_RULING: &str = "\
CLUSTER Tighe
SURVIVOR person-karen-a-tighe
MERGE person-tighe
END
";

#[test]
fn the_tighe_cluster_plans_to_conserve_one_hundred_and_one_statements() {
    // The addendum's acceptance test, stated as the plan's own arithmetic:
    // 39 + 62 → one node with 101.
    let rulings = parse(TIGHE_RULING).expect("valid rulings");
    let plan = MergePlan::build(&rulings, &census()).expect("a buildable plan");

    match &plan.clusters[0].disposition {
        Disposition::Merge {
            survivor,
            members,
            expected_statements,
        } => {
            assert_eq!(survivor.id, "person-karen-a-tighe");
            assert_eq!(members.len(), 1);
            assert_eq!(*expected_statements, 101);
        }
        other => panic!("expected a merge, got {other:?}"),
    }

    let t = plan.totals();
    assert_eq!(t.clusters_to_merge, 1);
    assert_eq!(
        t.nodes_to_merge_in, 1,
        "the People page must drop by exactly 1"
    );
    assert_eq!(
        t.statements_to_move, 62,
        "62 statements move; the survivor's own 39 stay where they are"
    );
}

#[test]
fn a_skip_plans_nothing_and_keeps_its_reason() {
    let rulings = parse("CLUSTER Humphrey\nSKIP Jeff is ambiguous\nEND\n").expect("valid");
    let plan = MergePlan::build(&rulings, &census()).expect("a buildable plan");

    assert_eq!(
        plan.clusters[0].disposition,
        Disposition::Skipped {
            reason: "Jeff is ambiguous".to_string()
        }
    );
    let t = plan.totals();
    assert_eq!(t.clusters_skipped, 1);
    assert_eq!(t.nodes_to_merge_in, 0);
}

#[test]
fn a_survivor_that_is_not_in_the_graph_refuses_the_whole_plan() {
    // Checked before execution, because a cluster refused half-way through a run
    // leaves the People page in a state nobody planned.
    let rulings = parse("CLUSTER T\nSURVIVOR person-typo\nMERGE person-tighe\nEND\n").unwrap();
    match MergePlan::build(&rulings, &census()).unwrap_err() {
        PlanError::MissingSurvivor { label, survivor } => {
            assert_eq!(label, "T");
            assert_eq!(survivor, "person-typo");
        }
        other => panic!("expected a missing-survivor error, got {other:?}"),
    }
}

#[test]
fn merging_a_person_into_an_organization_is_refused() {
    let rulings =
        parse("CLUSTER Mixed\nSURVIVOR org-catholic-family-services\nMERGE person-tighe\nEND\n")
            .unwrap();
    match MergePlan::build(&rulings, &census()).unwrap_err() {
        PlanError::LabelMismatch {
            member,
            member_label,
            survivor_label,
            ..
        } => {
            assert_eq!(member, "person-tighe");
            assert_eq!(member_label, "Person");
            assert_eq!(survivor_label, "Organization");
        }
        other => panic!("expected a label mismatch, got {other:?}"),
    }
}

#[test]
fn a_member_already_gone_is_idempotent_not_an_error() {
    // A run interrupted after three clusters must be safe to repeat. The second
    // run finds the member already merged away and plans nothing for it.
    let after_first_run: Vec<PartyNode> = census()
        .into_iter()
        .filter(|p| p.id != "person-tighe")
        .collect();
    let rulings = parse(TIGHE_RULING).expect("valid rulings");
    let plan = MergePlan::build(&rulings, &after_first_run).expect("a buildable plan");

    assert_eq!(
        plan.clusters[0].disposition,
        Disposition::AlreadyMerged {
            survivor: "person-karen-a-tighe".to_string()
        }
    );
    let t = plan.totals();
    assert_eq!(t.clusters_already_merged, 1);
    assert_eq!(t.clusters_to_merge, 0);
    assert_eq!(t.nodes_to_merge_in, 0);
}

#[test]
fn a_partly_merged_cluster_still_merges_what_remains() {
    let text = "CLUSTER Sharp\nSURVIVOR person-karen-a-tighe\nMERGE person-gone\nMERGE person-tighe\nEND\n";
    let rulings = parse(text).expect("valid rulings");
    let plan = MergePlan::build(&rulings, &census()).expect("a buildable plan");

    match &plan.clusters[0].disposition {
        Disposition::Merge {
            members,
            expected_statements,
            ..
        } => {
            assert_eq!(members.len(), 1, "the vanished member is simply not there");
            assert_eq!(*expected_statements, 101);
        }
        other => panic!("expected a merge, got {other:?}"),
    }
}

#[test]
fn totals_add_up_across_a_mixed_file() {
    let text = "\
CLUSTER Tighe
SURVIVOR person-karen-a-tighe
MERGE person-tighe
END
CLUSTER Humphrey
SKIP ambiguous
END
";
    let rulings = parse(text).expect("valid rulings");
    let plan = MergePlan::build(&rulings, &census()).expect("a buildable plan");
    let t = plan.totals();

    assert_eq!(t.clusters_ruled, 2);
    assert_eq!(t.clusters_to_merge, 1);
    assert_eq!(t.clusters_skipped, 1);
    assert_eq!(t.clusters_already_merged, 0);
    assert_eq!(plan.merges().count(), 1);
}
