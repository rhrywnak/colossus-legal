//! Behavioural tests for `build_event_detail` — the single-event payload with
//! its notes and its history.
//!
//! Split from `chronology_read_tests.rs`, which crossed the 300-line module
//! limit. See that file's header for why the fixtures are duplicated rather
//! than shared.

use super::*;
use crate::dto::chronology::LinkResolution;
use chrono::{NaiveDate, TimeZone, Utc};

fn ts() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 25, 12, 0, 0)
        .single()
        .expect("a real instant")
}

fn phase(id: &str, order: i32) -> ChronologyPhaseRow {
    ChronologyPhaseRow {
        id: id.to_string(),
        label: id.to_uppercase(),
        date_range: "2014\u{2013}Present".to_string(),
        color: "#059669".to_string(),
        description: Some("a subtitle".to_string()),
        sort_order: order,
    }
}

fn event(id: Uuid, attributes: serde_json::Value) -> ChronologyEventRow {
    ChronologyEventRow {
        id,
        case_slug: "a_case".to_string(),
        event_date: NaiveDate::from_ymd_opt(2012, 4, 12).expect("a real date"),
        date_precision: "day".to_string(),
        approximate: false,
        phase: "appeals".to_string(),
        title: "Judge Tighe Issues Post-Appeal Order".to_string(),
        fact: Some("Judge Tighe issues Opinion and Order.".to_string()),
        attributes,
        created_by: Some("roman".to_string()),
        created_at: ts(),
        updated_by: None,
        updated_at: ts(),
    }
}

fn link(event_id: Uuid, target_type: &str, target_id: &str) -> ChronologyLinkRow {
    ChronologyLinkRow {
        event_id,
        target_type: target_type.to_string(),
        target_id: target_id.to_string(),
        label: Some("Tighe Order".to_string()),
        pinpoint: None,
        created_by: Some("roman".to_string()),
        created_at: ts(),
    }
}

fn seeded_attributes() -> serde_json::Value {
    // NOTE what is absent: `phase`. It lives in the column and nowhere else.
    serde_json::json!({"tags": ["court_action"], "source": "legacy_json", "source_id": "e016"})
}

// ─── build_event_detail ──────────────────────────────────────────────────────

#[test]
fn the_detail_payload_carries_notes_history_and_a_note_count_that_matches() {
    let id = Uuid::from_u128(9);
    let notes = vec![ChronologyNoteRow {
        id: Uuid::from_u128(90),
        event_id: id,
        note: "Check this against the docket.".to_string(),
        created_by: Some("marie".to_string()),
        created_at: ts(),
    }];
    let history = vec![ChronologyHistoryRow {
        id: Uuid::from_u128(91),
        event_id: id,
        action: "created".to_string(),
        snapshot: serde_json::json!({"title": "Judge Tighe Issues Post-Appeal Order"}),
        changed_by: Some("roman".to_string()),
        changed_at: ts(),
    }];

    let composed = build_event_detail(
        &event(id, seeded_attributes()),
        &[],
        &notes,
        &history,
        &HashSet::new(),
    );

    assert_eq!(composed.payload.notes.len(), 1);
    assert_eq!(
        composed.payload.notes[0].created_by.as_deref(),
        Some("marie")
    );
    assert_eq!(composed.payload.history.len(), 1);
    assert_eq!(composed.payload.history[0].action, "created");
    assert_eq!(
        composed.payload.event.note_count, 1,
        "the count and the list must be the same fact"
    );
}

#[test]
fn an_event_with_no_history_returns_an_empty_list_not_an_absence() {
    let id = Uuid::from_u128(10);
    let composed = build_event_detail(
        &event(id, seeded_attributes()),
        &[],
        &[],
        &[],
        &HashSet::new(),
    );

    assert!(composed.payload.history.is_empty());
    assert!(composed.payload.notes.is_empty());
    // Serialised, the field is PRESENT and empty — "no changes recorded" — not
    // missing, which a frontend could not tell from an older payload shape.
    let json = serde_json::to_value(&composed.payload).expect("serialises");
    assert_eq!(json["history"], serde_json::json!([]));
    assert_eq!(json["notes"], serde_json::json!([]));
}
