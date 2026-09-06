// Shared fixtures for the `services::fact_card_render` tests.
//
// One card record, one supports column, one allegation table, one point table —
// built here so both halves of the split test module assert against the SAME
// starting card. A fixture that drifted between the two files would make a
// failure in one of them unreadable from the other.
//
// ## Rust Learning: `pub(super)` on a test fixture
//
// This module is a sibling of the two test modules under
// `services::fact_card_render`. `pub(super)` publishes each fixture to that
// parent and its descendants — enough for `use super::render_fixtures::*;` in
// either test file, and no further.

use super::*;
use crate::repositories::pipeline_repository::scenario_fact_cards::CardSupport;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub(super) const MACHINE: &str = crate::domain::fact_card::MACHINE_AUTHOR;

pub(super) fn at() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_757_000_000, 0).expect("a valid instant")
}

/// A card with nothing on it and nobody named as its author.
pub(super) fn bare_record() -> CardRecord {
    CardRecord {
        scenario_id: Uuid::nil(),
        graph_node_id: "doc-tighe:evidence:8a240a75".to_string(),
        title: None,
        backs_position: None,
        supports: None,
        watch_out: None,
        answer: None,
        title_authored_by: None,
        backs_position_authored_by: None,
        supports_authored_by: None,
        watch_out_authored_by: None,
        answer_authored_by: None,
        authored_by: "roman".to_string(),
        created_at: at(),
        edited_at: at(),
    }
}

pub(super) fn supports_value(entries: &[(&str, CardStance)]) -> serde_json::Value {
    serde_json::Value::Array(
        entries
            .iter()
            .map(|(id, stance)| {
                serde_json::to_value(CardSupport {
                    allegation_id: (*id).to_string(),
                    stance: *stance,
                })
                .expect("a support serializes")
            })
            .collect(),
    )
}

pub(super) fn allegations() -> HashMap<String, AllegationLabel> {
    HashMap::from([
        (
            "a-21".to_string(),
            AllegationLabel {
                paragraph: Some("21".to_string()),
                text: Some("CFS could have returned the money.".to_string()),
                count_tag: Some("Count 1 — Breach of Fiduciary Duty".to_string()),
            },
        ),
        (
            "a-44".to_string(),
            AllegationLabel {
                paragraph: Some("44".to_string()),
                text: Some("They ignored her arguments.".to_string()),
                count_tag: Some("Count 1 — Breach of Fiduciary Duty".to_string()),
            },
        ),
        (
            "a-61".to_string(),
            AllegationLabel {
                paragraph: Some("61".to_string()),
                text: Some("Sanctions were sought against her alone.".to_string()),
                count_tag: Some("Count 4 — Abuse of Process".to_string()),
            },
        ),
        (
            "a-unwired".to_string(),
            AllegationLabel {
                paragraph: Some("9".to_string()),
                text: Some("An accusation no element carries yet.".to_string()),
                count_tag: None,
            },
        ),
    ])
}

pub(super) fn points() -> HashMap<i32, String> {
    HashMap::from([
        (1, "I never refused to pay for the funeral.".to_string()),
        (2, "My sisters' claim rested on hearsay.".to_string()),
    ])
}

pub(super) fn words() -> FactCardWording {
    FactCardWording::for_test()
}

pub(super) fn render(record: &CardRecord) -> FactCardBlock {
    let allegations = allegations();
    let points = points();
    let wording = words();
    render_card(
        record,
        RenderContext {
            allegations: &allegations,
            talking_points: &points,
            wording: &wording,
        },
    )
}
