//! Unit tests for [`super`] — the two decodes that stand between stored rows and
//! the derivation.
//!
//! Both are the same kind of guard and both matter for the same reason: a row this
//! build cannot classify must never be silently skipped. A skipped judgment is an
//! accusation quietly said one fewer time, with a gap quietly missing from the prep
//! list, and nothing on screen to say either happened.
//!
//! The handler itself is five reads and a call; it is DEV-verified, the house
//! convention for a live pool.

use super::*;
use crate::repositories::pipeline_repository::{ScenarioFactRefRecord, ScenarioHumanFactRecord};
use chrono::{TimeZone, Utc};

fn a_scenario() -> uuid::Uuid {
    uuid::Uuid::nil()
}

fn fact_ref(graph_node_id: &str, status: &str) -> ScenarioFactRefRecord {
    ScenarioFactRefRecord {
        scenario_id: a_scenario(),
        graph_node_id: graph_node_id.to_string(),
        role_in_this_scenario: None,
        status: status.to_string(),
        note: None,
        confidence: None,
        source_run_id: None,
        tagged_at: Utc
            .timestamp_opt(1_700_000_000, 0)
            .single()
            .expect("a real instant"),
        defer_reason: None,
        tier: "backup".to_string(),
        sort_ordinal: None,
        tier_updated_by: None,
        tier_updated_at: None,
        order_updated_by: None,
        order_updated_at: None,
    }
}

fn human_row(kind: &str, anchor: Option<&str>, answers: Option<&str>) -> ScenarioHumanFactRecord {
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
            "Switching counsel meant emailing attachments".to_string()
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

// ── The included set ─────────────────────────────────────────────────────────

#[test]
fn only_included_refs_reach_the_derivation() {
    // The set is what turns the Remove law into a rendered gap. An undecided or
    // dropped candidate is not IN the scenario, so a judgment pointing at one is a
    // gap rather than a tally mark.
    let refs = vec![
        fact_ref("ev-in", "included"),
        fact_ref("ev-undecided", "undecided"),
        fact_ref("ev-dropped", "dropped"),
    ];

    let included = included_ids(&refs, a_scenario()).expect("every token is defined");

    assert_eq!(included.len(), 1);
    assert!(included.contains("ev-in"));
}

#[test]
fn an_unreadable_status_refuses_rather_than_reading_as_not_included() {
    // The silent-drop this guard exists for: treating an unknown token as "not
    // included" would take a marked instance out of the count AND out of the gap
    // list at once — the accusation said one fewer time, with nothing to notice.
    let refs = vec![
        fact_ref("ev-a", "included"),
        fact_ref("ev-b", "shortlisted"),
    ];

    let Err(error) = included_ids(&refs, a_scenario()) else {
        panic!("a status this build does not define must refuse");
    };
    assert!(matches!(error, AppError::Internal { .. }));
}

#[test]
fn a_scenario_with_no_refs_yields_an_empty_set_and_not_an_error() {
    // A fresh scenario is the ordinary state, and it must render the "nothing
    // marked, N waiting" notice rather than an error banner.
    let included = included_ids(&[], a_scenario()).expect("no rows is a valid answer");
    assert!(included.is_empty());
}

// ── The judgments ────────────────────────────────────────────────────────────

#[test]
fn prose_rows_are_skipped_and_anchored_rows_are_kept() {
    // A fact and a watch-list note are about nothing but themselves; they belong
    // to another section entirely and their absence here is the ordinary state.
    let records = vec![
        human_row("fact", None, None),
        human_row("watch_list", None, None),
        human_row("accusation_instance", Some("ev-a"), None),
        human_row("answer_pairing", Some("ev-a"), Some("ev-answer")),
    ];

    let judgments = judgments_in(&records, a_scenario()).expect("all four are defined kinds");

    assert_eq!(judgments.len(), 2);
    assert_eq!(judgments[0].kind, HumanFactKind::AccusationInstance);
    assert_eq!(judgments[0].anchor_graph_node_id, "ev-a");
    assert_eq!(judgments[1].kind, HumanFactKind::AnswerPairing);
    assert_eq!(
        judgments[1].answers_graph_node_id.as_deref(),
        Some("ev-answer")
    );
}

#[test]
fn an_anchored_row_with_an_undefined_kind_refuses() {
    // It is somebody's judgment about a specific statement, and this build cannot
    // say which judgment. Rendering the panel without it would drop a human's
    // decision silently — the failure Standing Rule 1 forbids by name.
    let records = vec![human_row("rebuttal", Some("ev-a"), None)];

    let Err(error) = judgments_in(&records, a_scenario()) else {
        panic!("an anchored row this build cannot classify must refuse");
    };
    assert!(matches!(error, AppError::Internal { .. }));
}

#[test]
fn a_prose_kind_carrying_an_anchor_refuses_rather_than_being_guessed_at() {
    // The row's SHAPE says it is about a statement while its KIND says it is not.
    // Nothing here may pick a winner: read as a fact it loses its anchor, read as
    // an instance it invents a judgment nobody made.
    let records = vec![human_row("fact", Some("ev-a"), None)];

    let Err(error) = judgments_in(&records, a_scenario()) else {
        panic!("a fact with an anchor is a contradiction and must refuse");
    };
    assert!(matches!(error, AppError::Internal { .. }));
}

#[test]
fn an_undefined_kind_with_no_anchor_is_skipped_and_not_refused() {
    // The asymmetry, stated: a prose row of a kind this build does not know is a
    // FUTURE section's content sitting in the same table (§2d's ratified
    // annotation is exactly that). It is not this surface's business, and refusing
    // it would break the accusation section every time a sibling task seeds a kind
    // ahead of the code that reads it.
    let records = vec![
        human_row("annotation", None, None),
        human_row("accusation_instance", Some("ev-a"), None),
    ];

    let judgments = judgments_in(&records, a_scenario()).expect("the unknown row is not ours");
    assert_eq!(judgments.len(), 1);
}

#[test]
fn the_order_rows_arrive_in_does_not_decide_the_prep_list() {
    // The repository reads oldest-first. The derivation re-sorts, and the panel
    // preserves the derivation's order — so two humans marking in different orders
    // see the same list. Pinned here because this decode is the one place that
    // could quietly make the read order load-bearing.
    let forwards = vec![
        human_row("accusation_instance", Some("ev-z"), None),
        human_row("accusation_instance", Some("ev-a"), None),
    ];
    let backwards = vec![
        human_row("accusation_instance", Some("ev-a"), None),
        human_row("accusation_instance", Some("ev-z"), None),
    ];

    let one = derive(
        &judgments_in(&forwards, a_scenario()).expect("defined kinds"),
        &["ev-a".to_string(), "ev-z".to_string()]
            .into_iter()
            .collect(),
    );
    let other = derive(
        &judgments_in(&backwards, a_scenario()).expect("defined kinds"),
        &["ev-a".to_string(), "ev-z".to_string()]
            .into_iter()
            .collect(),
    );

    assert_eq!(one.instances, other.instances);
    assert_eq!(one.gaps, other.gaps);
}
