//! Behavioural tests for the chronology read composition.
//!
//! Pure: every case builds rows by hand, states the expected payload, and never
//! touches a database. The degradation cases matter most — they are the ones
//! that must produce a WARNING and a rendered row, never an error.

use super::*;
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
        case_id: "a_case".to_string(),
        event_date: NaiveDate::from_ymd_opt(2012, 4, 12).expect("a real date"),
        date_precision: "day".to_string(),
        approximate: false,
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
    serde_json::json!({"tags": ["court_action"], "phase": "appeals", "source": "legacy_json"})
}

// ─── tags_of / phase_of ──────────────────────────────────────────────────────

#[test]
fn a_well_formed_bag_yields_its_tags_and_phase_with_no_warning() {
    let (tags, odd) = tags_of(&seeded_attributes());
    assert_eq!(tags, vec!["court_action".to_string()]);
    assert!(!odd);

    let (phase, odd) = phase_of(&seeded_attributes());
    assert_eq!(phase.as_deref(), Some("appeals"));
    assert!(!odd);
}

#[test]
fn an_empty_bag_is_a_real_state_not_a_failure() {
    let empty = serde_json::json!({});
    assert_eq!(tags_of(&empty), (Vec::new(), false));
    assert_eq!(phase_of(&empty), (None, false));
}

#[test]
fn a_tags_key_that_is_not_an_array_degrades_and_says_so() {
    let bad = serde_json::json!({"tags": "court_action"});
    let (tags, odd) = tags_of(&bad);
    assert!(tags.is_empty(), "a string is not a tag list");
    assert!(odd, "the degradation must be reported, not swallowed");
}

#[test]
fn an_array_holding_non_strings_keeps_the_strings_and_still_reports() {
    let mixed = serde_json::json!({"tags": ["filing", 7]});
    let (tags, odd) = tags_of(&mixed);
    assert_eq!(tags, vec!["filing".to_string()], "the usable half survives");
    assert!(odd, "the unusable half is still reported");
}

#[test]
fn a_phase_key_that_is_not_a_string_degrades_and_says_so() {
    let bad = serde_json::json!({"phase": ["appeals"]});
    let (phase, odd) = phase_of(&bad);
    assert_eq!(phase, None);
    assert!(odd);
}

// ─── build_timeline ──────────────────────────────────────────────────────────

#[test]
fn a_link_to_a_document_that_exists_resolves() {
    let id = Uuid::from_u128(1);
    let links = vec![link(id, "document", "doc-real")];
    let resolved: HashSet<String> = ["doc-real".to_string()].into_iter().collect();

    let composed = build_timeline(
        &[phase("appeals", 3)],
        &[event(id, seeded_attributes())],
        &links,
        &HashMap::new(),
        &resolved,
    );

    let link_dto = &composed.payload.events[0].links[0];
    assert!(link_dto.resolves);
    assert!(composed.warnings.is_empty());
}

#[test]
fn a_link_to_a_document_that_does_not_exist_is_data_not_an_error() {
    let id = Uuid::from_u128(2);
    let links = vec![link(id, "document", "doc-vanished")];

    let composed = build_timeline(
        &[],
        &[event(id, seeded_attributes())],
        &links,
        &HashMap::new(),
        &HashSet::new(),
    );

    let link_dto = &composed.payload.events[0].links[0];
    assert!(!link_dto.resolves, "the dead link is reported, not dropped");
    assert_eq!(link_dto.target_id, "doc-vanished", "and it keeps its id");
    assert_eq!(
        composed.payload.events[0].links.len(),
        1,
        "a dead link is never silently removed from the list"
    );
}

#[test]
fn a_target_type_this_build_cannot_check_warns_rather_than_claiming_to_know() {
    let id = Uuid::from_u128(3);
    let links = vec![link(id, "paperless_document", "42")];

    let composed = build_timeline(
        &[],
        &[event(id, seeded_attributes())],
        &links,
        &HashMap::new(),
        &HashSet::new(),
    );

    assert!(!composed.payload.events[0].links[0].resolves);
    assert_eq!(composed.warnings.len(), 1);
    assert!(
        composed.warnings[0].contains("paperless_document"),
        "the warning must name the type, got: {}",
        composed.warnings[0]
    );
}

#[test]
fn note_counts_default_to_zero_for_an_event_with_no_notes() {
    let with_notes = Uuid::from_u128(4);
    let without = Uuid::from_u128(5);
    let counts: HashMap<Uuid, i64> = [(with_notes, 3)].into_iter().collect();

    let composed = build_timeline(
        &[],
        &[
            event(with_notes, seeded_attributes()),
            event(without, seeded_attributes()),
        ],
        &[],
        &counts,
        &HashSet::new(),
    );

    assert_eq!(composed.payload.events[0].note_count, 3);
    assert_eq!(composed.payload.events[1].note_count, 0);
}

#[test]
fn links_are_attached_to_their_own_event_and_no_other() {
    let a = Uuid::from_u128(6);
    let b = Uuid::from_u128(7);
    let links = vec![link(a, "document", "doc-a"), link(b, "document", "doc-b")];

    let composed = build_timeline(
        &[],
        &[event(a, seeded_attributes()), event(b, seeded_attributes())],
        &links,
        &HashMap::new(),
        &HashSet::new(),
    );

    assert_eq!(composed.payload.events[0].links[0].target_id, "doc-a");
    assert_eq!(composed.payload.events[1].links[0].target_id, "doc-b");
}

#[test]
fn the_whole_attributes_bag_survives_the_trip() {
    let id = Uuid::from_u128(8);
    // A key no build has ever heard of. The change rule says it must arrive.
    let bag =
        serde_json::json!({"tags": ["filing"], "phase": "probate", "invented_later": {"x": 1}});

    let composed = build_timeline(
        &[],
        &[event(id, bag.clone())],
        &[],
        &HashMap::new(),
        &HashSet::new(),
    );

    assert_eq!(composed.payload.events[0].attributes, bag);
    assert_eq!(
        composed.payload.events[0].attributes["invented_later"]["x"],
        1
    );
}

#[test]
fn phases_travel_in_their_stored_order_with_their_subtitle() {
    let composed = build_timeline(
        &[phase("estate", 1), phase("civil_lawsuit", 4)],
        &[],
        &[],
        &HashMap::new(),
        &HashSet::new(),
    );

    let ids: Vec<&str> = composed
        .payload
        .phases
        .iter()
        .map(|p| p.id.as_str())
        .collect();
    assert_eq!(ids, vec!["estate", "civil_lawsuit"]);
    assert_eq!(
        composed.payload.phases[0].description.as_deref(),
        Some("a subtitle")
    );
    assert_eq!(composed.payload.phases[0].date_range, "2014\u{2013}Present");
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
