//! Behavioural tests for the chronology read composition: the attribute
//! helpers, the phase column, and `build_timeline`.
//!
//! `build_event_detail` has its own file next door — this one crossed the
//! 300-line module limit, and the split follows the two functions rather than
//! cutting the file in half at an arbitrary point. The fixtures are duplicated
//! deliberately: sharing them would mean a third module existing only to hold
//! six constructors, and a test fixture that two files must agree on is worse
//! than two that are each obviously right.
//!
//! Pure: every case builds rows by hand, states the expected payload, and never
//! touches a database. The degradation cases matter most — they are the ones
//! that must produce a WARNING and a rendered row, never an error.

use super::*;
use crate::domain::wording_chronology::ChronologyWording;
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

/// `build_timeline` with the arguments these cases do not vary.
///
/// Phase B gave the builder a tag list, the wording block and the scroll-window
/// size. Every case here is about phases, events, links, counts and resolution —
/// so those three are supplied once, here, rather than repeated verbatim in a
/// dozen call sites where they would be noise a reader has to skip.
fn timeline_of(
    phases: &[ChronologyPhaseRow],
    events: &[ChronologyEventRow],
    links: &[ChronologyLinkRow],
    note_counts: &HashMap<Uuid, i64>,
    resolved_documents: &HashSet<String>,
) -> Composed<TimelineDto> {
    build_timeline(TimelineSources {
        phases,
        tags: &[],
        events,
        links,
        note_counts,
        resolved_documents,
        wording: &ChronologyWording::for_test(),
        phase_window_events: 4,
    })
}

fn seeded_attributes() -> serde_json::Value {
    // NOTE what is absent: `phase`. It lives in the column and nowhere else.
    serde_json::json!({"tags": ["court_action"], "source": "legacy_json", "source_id": "e016"})
}

// ─── tags_of / phase_of ──────────────────────────────────────────────────────

#[test]
fn a_well_formed_bag_yields_its_tags_with_no_warning() {
    let (tags, odd) = tags_of(&seeded_attributes());
    assert_eq!(tags, vec!["court_action".to_string()]);
    assert!(!odd);
}

#[test]
fn the_bag_carries_no_phase_and_the_column_is_the_only_home() {
    assert!(
        seeded_attributes().get("phase").is_none(),
        "a mirrored attributes.phase is the second home that goes stale"
    );

    let composed = timeline_of(
        &[],
        &[event(Uuid::from_u128(1), seeded_attributes())],
        &[],
        &HashMap::new(),
        &HashSet::new(),
    );
    assert_eq!(composed.payload.events[0].phase, "appeals");
}

#[test]
fn the_phase_on_the_wire_comes_from_the_column_even_if_a_stale_bag_disagrees() {
    // A bag written by some future caller that wrongly mirrors the phase must
    // not be able to change what the payload says. The column wins, always.
    let stale = serde_json::json!({"tags": ["court_action"], "phase": "probate"});
    let composed = timeline_of(
        &[],
        &[event(Uuid::from_u128(2), stale)],
        &[],
        &HashMap::new(),
        &HashSet::new(),
    );
    assert_eq!(
        composed.payload.events[0].phase, "appeals",
        "the column is the single source of truth for the phase"
    );
}

#[test]
fn an_empty_bag_is_a_real_state_not_a_failure() {
    let empty = serde_json::json!({});
    assert_eq!(tags_of(&empty), (Vec::new(), false));
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

// ─── build_timeline ──────────────────────────────────────────────────────────

#[test]
fn a_link_to_a_document_that_exists_resolves() {
    let id = Uuid::from_u128(1);
    let links = vec![link(id, "document", "doc-real")];
    let resolved: HashSet<String> = ["doc-real".to_string()].into_iter().collect();

    let composed = timeline_of(
        &[phase("appeals", 3)],
        &[event(id, seeded_attributes())],
        &links,
        &HashMap::new(),
        &resolved,
    );

    let link_dto = &composed.payload.events[0].links[0];
    assert_eq!(link_dto.resolution, LinkResolution::Resolves);
    assert!(composed.warnings.is_empty());
}

#[test]
fn a_link_to_a_document_that_does_not_exist_is_data_not_an_error() {
    let id = Uuid::from_u128(2);
    let links = vec![link(id, "document", "doc-vanished")];

    let composed = timeline_of(
        &[],
        &[event(id, seeded_attributes())],
        &links,
        &HashMap::new(),
        &HashSet::new(),
    );

    let link_dto = &composed.payload.events[0].links[0];
    assert_eq!(
        link_dto.resolution,
        LinkResolution::Missing,
        "looked for and not there is an ANSWER, not an absence of one"
    );
    assert_eq!(link_dto.target_id, "doc-vanished", "and it keeps its id");
    assert_eq!(
        composed.payload.events[0].links.len(),
        1,
        "a dead link is never silently removed from the list"
    );
}

#[test]
fn a_target_type_this_build_cannot_check_is_unchecked_not_missing() {
    let id = Uuid::from_u128(3);
    let links = vec![link(id, "paperless_document", "42")];

    let composed = timeline_of(
        &[],
        &[event(id, seeded_attributes())],
        &links,
        &HashMap::new(),
        &HashSet::new(),
    );

    assert_eq!(
        composed.payload.events[0].links[0].resolution,
        LinkResolution::Unchecked,
        "reporting it as Missing would be a claim nobody checked"
    );
    assert!(
        composed.warnings.is_empty(),
        "not knowing is not a degradation; it has its own name on the wire"
    );
}

#[test]
fn the_three_resolutions_serialise_to_the_agreed_wire_tokens() {
    let tokens: Vec<serde_json::Value> = [
        LinkResolution::Resolves,
        LinkResolution::Missing,
        LinkResolution::Unchecked,
    ]
    .iter()
    .map(|r| serde_json::to_value(r).expect("serialises"))
    .collect();

    assert_eq!(
        tokens,
        vec![
            serde_json::json!("resolves"),
            serde_json::json!("missing"),
            serde_json::json!("unchecked")
        ]
    );
}

#[test]
fn note_counts_default_to_zero_for_an_event_with_no_notes() {
    let with_notes = Uuid::from_u128(4);
    let without = Uuid::from_u128(5);
    let counts: HashMap<Uuid, i64> = [(with_notes, 3)].into_iter().collect();

    let composed = timeline_of(
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

    let composed = timeline_of(
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

    let composed = timeline_of(
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
    let composed = timeline_of(
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

// ─── what Phase B added to the payload ───────────────────────────────────────

fn tag(id: &str, label: &str, order: i32) -> ChronologyTagRow {
    ChronologyTagRow {
        id: id.to_string(),
        label: label.to_string(),
        color: "#059669".to_string(),
        sort_order: order,
    }
}

#[test]
fn the_tag_vocabulary_travels_in_its_stored_order() {
    // The filter chips ARE the stored vocabulary (ruling R-F): a sixth tag is a
    // row, not a build, and the order is the one Roman wrote — not alphabetical.
    let composed = build_timeline(TimelineSources {
        phases: &[],
        tags: &[
            tag("financial", "Financial", 1),
            tag("court_action", "Court Action", 2),
        ],
        events: &[],
        links: &[],
        note_counts: &HashMap::new(),
        resolved_documents: &HashSet::new(),
        wording: &ChronologyWording::for_test(),
        phase_window_events: 4,
    });

    let ids: Vec<&str> = composed
        .payload
        .tags
        .iter()
        .map(|t| t.id.as_str())
        .collect();
    assert_eq!(ids, vec!["financial", "court_action"]);
    assert_eq!(composed.payload.tags[1].label, "Court Action");
    assert_eq!(composed.payload.tags[0].color, "#059669");
}

#[test]
fn the_payload_carries_the_words_and_the_window_size() {
    // One fetch serves the page: the rows, the vocabulary, every sentence, and
    // the one number the scroll window reads.
    let composed = build_timeline(TimelineSources {
        phases: &[],
        tags: &[],
        events: &[],
        links: &[],
        note_counts: &HashMap::new(),
        resolved_documents: &HashSet::new(),
        wording: &ChronologyWording::for_test(),
        phase_window_events: 7,
    });

    assert_eq!(composed.payload.phase_window_events, 7);
    assert_eq!(composed.payload.wording.page_title, "Case Timeline");
    assert!(composed
        .payload
        .wording
        .no_document_label
        .contains("no document"));
    assert_ne!(
        composed.payload.wording.no_document_label,
        composed.payload.wording.link_unchecked_label
    );
}

#[test]
fn an_empty_vocabulary_is_served_as_an_empty_list_not_an_absence() {
    let composed = timeline_of(&[], &[], &[], &HashMap::new(), &HashSet::new());
    let json = serde_json::to_value(&composed.payload).expect("serialises");
    assert_eq!(json["tags"], serde_json::json!([]));
    assert!(
        json.get("wording").is_some(),
        "the words are always present, even when there is nothing to say them about"
    );
}
