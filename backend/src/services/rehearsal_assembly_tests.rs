//! Tests for [`super`] — the one decode that stands between stored rows and the
//! render.
//!
//! The reads themselves need a live Postgres and a live Neo4j and are
//! DEV-verified, the house convention. What is testable here is the decision that
//! decides whether a human's judgment reaches the page at all — and a judgment
//! silently skipped is an accusation quietly said one fewer time, with a gap
//! quietly missing from the prep list.

use super::*;
use crate::repositories::pipeline_repository::ScenarioHumanFactRecord;
use chrono::{TimeZone, Utc};

fn a_scenario() -> uuid::Uuid {
    uuid::Uuid::nil()
}

fn row(kind: &str, anchor: Option<&str>, answers: Option<&str>) -> ScenarioHumanFactRecord {
    let ts = Utc
        .timestamp_opt(1_700_000_000, 0)
        .single()
        .expect("a real instant");
    ScenarioHumanFactRecord {
        id: uuid::Uuid::nil(),
        scenario_id: a_scenario(),
        // An anchored row stores no prose; a fact or a watch-list note does.
        text: if anchor.is_some() {
            String::new()
        } else {
            "He will say she cannot work with anybody".to_string()
        },
        occurred_on: None,
        date_type: None,
        person_refs: None,
        authored_by: "Roman".to_string(),
        kind: kind.to_string(),
        created_at: ts,
        updated_at: ts,
        anchor_graph_node_id: anchor.map(str::to_string),
        answers_graph_node_id: answers.map(str::to_string),
    }
}

#[test]
fn prose_rows_are_skipped_and_anchored_rows_are_kept() {
    // A watch-list note belongs to another block entirely, and its absence from
    // the judgments is the ordinary state of every scenario.
    let rows = vec![
        row("watch_list", None, None),
        row("fact", None, None),
        row("accusation_instance", Some("ev-a"), None),
        row("answer_pairing", Some("ev-a"), Some("ev-answer")),
    ];

    let judgments = judgments_in(&rows, a_scenario()).expect("all four kinds are defined");

    assert_eq!(judgments.len(), 2);
    assert_eq!(judgments[0].kind, HumanFactKind::AccusationInstance);
    assert_eq!(judgments[1].kind, HumanFactKind::AnswerPairing);
    assert_eq!(
        judgments[1].answers_graph_node_id.as_deref(),
        Some("ev-answer")
    );
}

#[test]
fn an_anchored_row_with_an_undefined_kind_refuses_rather_than_being_skipped() {
    // It is somebody's judgment about a specific statement, and this build cannot
    // say which. Rendering the page without it would drop a human's decision
    // silently — the failure Standing Rule 1 forbids by name.
    let rows = vec![row("rehearsal_highlight", Some("ev-a"), None)];

    let Err(error) = judgments_in(&rows, a_scenario()) else {
        panic!("an anchored row this build cannot classify must refuse");
    };
    assert!(matches!(error, AssemblyError::Undecodable { .. }));
}

#[test]
fn a_prose_kind_carrying_an_anchor_refuses_rather_than_being_guessed_at() {
    // The row's SHAPE says it is about a statement while its KIND says it is not.
    // Nothing here may pick a winner: read as a note it loses its anchor, read as
    // an instance it invents a judgment nobody made.
    let rows = vec![row("watch_list", Some("ev-a"), None)];

    let Err(error) = judgments_in(&rows, a_scenario()) else {
        panic!("a watch-list note with an anchor is a contradiction");
    };
    assert!(matches!(error, AssemblyError::Undecodable { .. }));
}

#[test]
fn an_undefined_prose_kind_is_skipped_and_not_refused() {
    // The asymmetry, stated: a prose row of a kind this build does not know is a
    // FUTURE block's content in the same table — B3's margin notes will be exactly
    // that. Refusing it would break the rehearsal page every time a sibling task
    // seeds a kind ahead of the code that reads it.
    let rows = vec![
        row("annotation", None, None),
        row("accusation_instance", Some("ev-a"), None),
    ];

    let judgments = judgments_in(&rows, a_scenario()).expect("the unknown row is not ours");
    assert_eq!(judgments.len(), 1);
}

#[test]
fn a_scenario_with_no_judgments_yields_an_empty_list_and_not_an_error() {
    // The state S-2 is in today, measured. It must render the page's honest
    // nothing-marked notice, never an error banner.
    assert!(judgments_in(&[], a_scenario())
        .expect("no rows is an answer")
        .is_empty());
}

#[test]
fn the_two_record_stores_report_separate_failures() {
    // A Postgres outage and a Neo4j outage have different operators and different
    // remedies. Collapsing them would produce one "failed to load" that says which
    // service is down to nobody.
    let pg = AssemblyError::Read {
        source: crate::repositories::pipeline_repository::PipelineRepoError::Database(
            "connection refused".to_string(),
        ),
    };
    let graph = AssemblyError::Record {
        source: crate::repositories::scenario_card_repository::ScenarioCardRepoError::RowDecode {
            operation: "fetch_rehearsal_facts",
            source: neo4rs::DeError::PropertyMissingButRequired,
        },
    };

    assert_ne!(pg.to_string(), graph.to_string());
    assert!(pg.to_string().contains("rehearsal content"), "{pg}");
    assert!(graph.to_string().contains("record store"), "{graph}");
}
