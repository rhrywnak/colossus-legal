// Shared fixtures for the `services::rehearsal_cards` tests.
//
// One card, one fact row, one deck, one list of our own speakers. Both halves of
// the split test module build from these, so "the other side" means the same
// thing in each — the definition is by ELIMINATION, and a fixture that drifted
// would quietly change which speakers count as ours.
//
// ## Rust Learning: `pub(super)` on a test fixture
//
// This module is a sibling of the two test modules under
// `services::rehearsal_cards`. `pub(super)` publishes each fixture to that
// parent and its descendants — enough for `use super::deck_fixtures::*;` in
// either test file, and no further.

use super::*;
use crate::repositories::scenario_accusation_repository::RehearsalFactRow;
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub(super) fn at() -> DateTime<Utc> {
    DateTime::<Utc>::from_timestamp(1_757_000_000, 0).expect("a valid instant")
}

pub(super) fn card(node: &str) -> CardRecord {
    CardRecord {
        scenario_id: Uuid::nil(),
        graph_node_id: node.to_string(),
        title: Some(format!("title of {node}")),
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

pub(super) fn fact(node: &str, speaker: Option<&str>, when: Option<&str>) -> RehearsalFactRow {
    RehearsalFactRow {
        graph_node_id: node.to_string(),
        quote: Some(format!("the words of {node}")),
        speaker: speaker.map(str::to_string),
        statement_type: Some("court_finding".to_string()),
        question: None,
        occurred_on: when.map(str::to_string),
        document_id: Some("doc-x".to_string()),
        document_title: Some("A Document".to_string()),
        page: Some(6),
    }
}

const OURS: &[&str] = &["marie awad"];

pub(super) fn deck<'a>(
    cards: &'a HashMap<String, CardRecord>,
    facts: &'a HashMap<String, RehearsalFactRow>,
    ordinals: &'a HashMap<String, i32>,
    ours: &'a [String],
) -> DeckInput<'a> {
    DeckInput {
        cards,
        facts,
        ordinals,
        our_side: ours,
    }
}

pub(super) fn ours_list() -> Vec<String> {
    OURS.iter().map(|s| (*s).to_string()).collect()
}

pub(super) fn index<T>(rows: Vec<(String, T)>) -> HashMap<String, T> {
    rows.into_iter().collect()
}
