//! Tests for `services::chronology_subset_read`.
//!
//! The composition is pure, so every ordering rule, every count and the gap
//! marking below are reachable without a Postgres. The QUERIES that feed it are
//! proved by `tests/timeline_subsets_integration.rs`; this proves what happens
//! to the rows once they arrive.

use super::*;

use chrono::{NaiveDate, TimeZone, Utc};

use crate::repositories::pipeline_repository::chronology_subsets::ChronologySubsetRow;

fn id(n: u8) -> Uuid {
    Uuid::from_bytes([n; 16])
}

fn subset() -> ChronologySubsetRow {
    ChronologySubsetRow {
        id: id(7),
        case_slug: "awad_v_catholic_family_service".to_string(),
        name: "The $50,000".to_string(),
        description: "What the money did.".to_string(),
        created_by: "roman".to_string(),
        created_at: Utc.with_ymd_and_hms(2026, 8, 30, 9, 0, 0).unwrap(),
        updated_by: "roman".to_string(),
        updated_at: Utc.with_ymd_and_hms(2026, 8, 30, 9, 0, 0).unwrap(),
        deleted_at: None,
    }
}

fn reference(event: u8, position: i32, note: &str) -> SubsetEventRefRow {
    SubsetEventRefRow {
        subset_id: id(7),
        event_id: id(event),
        position,
        note: note.to_string(),
    }
}

fn event(n: u8, day: u32, deleted: bool) -> ChronologyEventStateRow {
    ChronologyEventStateRow {
        id: id(n),
        case_slug: "awad_v_catholic_family_service".to_string(),
        event_date: NaiveDate::from_ymd_opt(2009, 3, day).expect("a real day"),
        date_precision: "day".to_string(),
        approximate: false,
        phase: "estate".to_string(),
        title: format!("event {n}"),
        fact: None,
        attributes: serde_json::json!({ "tags": ["Money"] }),
        created_by: Some("roman".to_string()),
        created_at: Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap(),
        updated_by: Some("roman".to_string()),
        updated_at: Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap(),
        deleted_at: deleted.then(|| Utc.with_ymd_and_hms(2026, 8, 29, 0, 0, 0).unwrap()),
    }
}

fn sources<'a>(
    subset: &'a ChronologySubsetRow,
    refs: &'a [SubsetEventRefRow],
    events: &'a [ChronologyEventStateRow],
    note_counts: &'a HashMap<Uuid, i64>,
    resolved: &'a HashSet<String>,
    carried_by: &'a [String],
) -> SubsetDetailSources<'a> {
    SubsetDetailSources {
        subset,
        refs,
        events,
        links: &[],
        note_counts,
        resolved_documents: resolved,
        carried_by,
    }
}

#[test]
fn the_events_come_back_in_the_order_the_references_arrived() {
    // ⚑ The composer does NOT sort. The repository's `ORDER BY position,
    // event_id` is the one place the story order is decided, and a second sort
    // here would be a second answer to "what order is this story in" — which is
    // how one of them eventually disagrees.
    let subset = subset();
    let refs = [
        reference(2, 1, ""),
        reference(3, 2, ""),
        reference(1, 3, ""),
    ];
    let events = [
        event(1, 16, false),
        event(2, 18, false),
        event(3, 20, false),
    ];
    let counts = HashMap::new();
    let resolved = HashSet::new();

    let composed = build_subset_detail(sources(&subset, &refs, &events, &counts, &resolved, &[]));
    let order: Vec<Uuid> = composed.payload.events.iter().map(|e| e.event.id).collect();
    assert_eq!(order, vec![id(2), id(3), id(1)]);
    assert!(composed.warnings.is_empty());
}

#[test]
fn a_deleted_event_is_marked_and_counted_never_dropped() {
    // Design R1. Dropping the row would silently shorten a story somebody
    // counted; the gap is half the value of a subset.
    let subset = subset();
    let refs = [
        reference(1, 1, ""),
        reference(2, 2, "why"),
        reference(3, 3, ""),
    ];
    let events = [event(1, 16, false), event(2, 18, true), event(3, 20, false)];
    let counts = HashMap::new();
    let resolved = HashSet::new();

    let composed = build_subset_detail(sources(&subset, &refs, &events, &counts, &resolved, &[]));
    let payload = composed.payload;
    assert_eq!(
        payload.event_count, 3,
        "the removed event is still in the story"
    );
    assert_eq!(payload.gap_count, 1);
    assert!(!payload.events[0].removed);
    assert!(payload.events[1].removed);
    // `deleted_at` reaches the wire too, so a surface can say WHEN rather than
    // only THAT — and the two must agree.
    assert!(payload.events[1].event.deleted_at.is_some());
    assert!(payload.events[2].event.deleted_at.is_none());
    // The subset's note rides beside the event, not inside it.
    assert_eq!(payload.events[1].subset_note, "why");
}

#[test]
fn a_reference_whose_event_has_no_row_is_reported_not_swallowed() {
    // Unreachable while the foreign key holds. If it is ever reached, the
    // operator gets a line naming the subset and the id — never a short list
    // that looks complete.
    let subset = subset();
    let refs = [reference(1, 1, ""), reference(9, 2, "")];
    let events = [event(1, 16, false)];
    let counts = HashMap::new();
    let resolved = HashSet::new();

    let composed = build_subset_detail(sources(&subset, &refs, &events, &counts, &resolved, &[]));
    assert_eq!(composed.payload.event_count, 1);
    assert_eq!(composed.warnings.len(), 1);
    assert!(composed.warnings[0].contains(&id(9).to_string()));
    assert!(composed.warnings[0].contains("foreign key"));
}

#[test]
fn the_event_shape_is_the_timelines_own() {
    // ⚑ "No second event shape". The tags come out of the attributes bag by the
    // SAME function the timeline uses, so a subset cannot render an event
    // differently. If this ever stops holding, the composer has started building
    // its own event.
    let subset = subset();
    let refs = [reference(1, 1, "")];
    let events = [event(1, 16, false)];
    let mut counts = HashMap::new();
    counts.insert(id(1), 4);
    let resolved = HashSet::new();

    let composed = build_subset_detail(sources(&subset, &refs, &events, &counts, &resolved, &[]));
    let dto = &composed.payload.events[0].event;
    assert_eq!(dto.tags, vec!["Money".to_string()]);
    assert_eq!(dto.phase, "estate");
    assert_eq!(dto.note_count, 4, "the badge count is carried through");
}

#[test]
fn carriers_render_as_scenario_codes_in_the_order_they_arrived() {
    // The backend spells `S-11`, never a screen. The rows arrive ordered by
    // `(position, code_ordinal)`, and the grouping preserves that.
    let rows = [
        SubsetCarrierRow {
            subset_id: id(7),
            code_ordinal: 11,
            position: 0,
        },
        SubsetCarrierRow {
            subset_id: id(7),
            code_ordinal: 12,
            position: 1,
        },
        SubsetCarrierRow {
            subset_id: id(8),
            code_ordinal: 3,
            position: 0,
        },
    ];
    let grouped = carriers_by_subset(&rows);
    assert_eq!(
        grouped[&id(7)],
        vec!["S-11".to_string(), "S-12".to_string()]
    );
    assert_eq!(grouped[&id(8)], vec!["S-3".to_string()]);
}

#[test]
fn a_subset_with_no_counts_and_no_carriers_still_appears() {
    // A story with no events and no scenario is a real state an author passes
    // through on the way to a real one. Dropping it would make it unreachable
    // from the only screen that could fix it.
    let subsets = [subset()];
    let list = build_subset_list(&subsets, &HashMap::new(), &HashMap::new());
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].event_count, 0);
    assert_eq!(list[0].gap_count, 0);
    assert!(list[0].carried_by.is_empty());
}

#[test]
fn the_list_carries_the_counts_and_carriers_it_was_given() {
    let subsets = [subset()];
    let counts = counts_by_subset(&[SubsetCountsRow {
        subset_id: id(7),
        event_count: 15,
        gap_count: 3,
    }]);
    let carriers = carriers_by_subset(&[SubsetCarrierRow {
        subset_id: id(7),
        code_ordinal: 11,
        position: 0,
    }]);

    let list = build_subset_list(&subsets, &counts, &carriers);
    assert_eq!(list[0].event_count, 15);
    assert_eq!(list[0].gap_count, 3);
    assert_eq!(list[0].carried_by, vec!["S-11".to_string()]);
    assert_eq!(list[0].name, "The $50,000");
}

#[test]
fn a_scenario_with_no_subsets_composes_an_empty_list() {
    // `[]` is what hides the View Timeline button. It is a different answer from
    // a 404, and the handler keeps them apart — see `scenario_links`.
    assert!(build_scenario_subsets(&[]).is_empty());
}

#[test]
fn a_scenarios_subsets_carry_their_counts_and_their_order() {
    let rows = [
        ScenarioSubsetRow {
            subset_id: id(7),
            name: "The $50,000".to_string(),
            position: 0,
            event_count: 15,
            gap_count: 3,
        },
        ScenarioSubsetRow {
            subset_id: id(8),
            name: "The fee engine".to_string(),
            position: 1,
            event_count: 9,
            gap_count: 0,
        },
    ];
    let dtos = build_scenario_subsets(&rows);
    assert_eq!(dtos.len(), 2);
    assert_eq!(dtos[0].position, 0);
    assert_eq!(dtos[0].gap_count, 3);
    assert_eq!(dtos[1].name, "The fee engine");
}
