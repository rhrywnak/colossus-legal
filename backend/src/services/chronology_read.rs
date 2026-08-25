//! Composing the chronology read payloads. Pure — no database, no clock.
//!
//! Every sentence the timeline endpoints speak is decided here, so a test can
//! reach it without a Postgres. The handlers in `api::timeline` fetch rows and
//! hand them over; this module decides what the frontend sees.
//!
//! ## Degradation is REPORTED, never silent (Standing Rule 1)
//!
//! Two things can be "wrong" with a stored row without being an error: an
//! `attributes` bag whose `tags` key is not an array of strings, and a link
//! whose `target_type` this build cannot check. Neither should fail a request —
//! a chronology that refuses to render because one row has an odd bag is worse
//! than one that renders it plainly. So each returns a WARNING alongside the
//! payload, and the handler logs every warning with the event's id. The reader
//! of the logs can always tell what degraded and which row did it.

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::dto::chronology::{
    TimelineDto, TimelineEventDetailDto, TimelineEventDto, TimelineHistoryDto, TimelineLinkDto,
    TimelineNoteDto, TimelinePhaseDto,
};
use crate::repositories::pipeline_repository::chronology::{
    ChronologyEventRow, ChronologyHistoryRow, ChronologyLinkRow, ChronologyNoteRow,
    ChronologyPhaseRow,
};

/// The `target_type` this build knows how to check.
pub const CHECKABLE_TARGET_TYPE: &str = "document";

/// A payload plus everything that degraded while composing it.
#[derive(Debug, Clone)]
pub struct Composed<T> {
    pub payload: T,
    /// One line per degradation, each naming the row it came from. Logged by the
    /// handler; never swallowed, never returned to the browser as an error.
    pub warnings: Vec<String>,
}

/// The tags in a bag, and whether the key was present but unusable.
///
/// ## Rust Learning: matching on `serde_json::Value` instead of deserialising
///
/// Deserialising into `Vec<String>` would turn a malformed bag into an `Err`
/// that takes the whole event with it. Walking the `Value` by hand lets the
/// three cases stay distinct: absent (fine, no tags), an array of strings (the
/// tags), and anything else (no tags AND a warning).
pub fn tags_of(attributes: &serde_json::Value) -> (Vec<String>, bool) {
    match attributes.get("tags") {
        None | Some(serde_json::Value::Null) => (Vec::new(), false),
        Some(serde_json::Value::Array(items)) => {
            let tags: Vec<String> = items
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect();
            // An array holding non-strings is partially usable and still odd.
            (tags.clone(), tags.len() != items.len())
        }
        Some(_) => (Vec::new(), true),
    }
}

/// The phase in a bag, and whether the key was present but unusable.
pub fn phase_of(attributes: &serde_json::Value) -> (Option<String>, bool) {
    match attributes.get("phase") {
        None | Some(serde_json::Value::Null) => (None, false),
        Some(serde_json::Value::String(s)) => (Some(s.clone()), false),
        Some(_) => (None, true),
    }
}

/// Whether one link's target exists, and a warning when this build cannot tell.
///
/// A `target_type` other than `document` returns `false` — the frontend renders
/// "no document", which is the honest thing to show for a target nobody can
/// confirm — and a warning, so an operator sees that a whole class of link is
/// being reported unresolved rather than actually checked. Phase A never creates
/// one; Phase C's other target kinds will need their own resolvers.
fn resolve_link(
    link: &ChronologyLinkRow,
    resolved_documents: &HashSet<String>,
) -> (bool, Option<String>) {
    if link.target_type == CHECKABLE_TARGET_TYPE {
        return (resolved_documents.contains(&link.target_id), None);
    }
    (
        false,
        Some(format!(
            "event {}: link target_type '{}' cannot be checked by this build; \
             reported as unresolved",
            link.event_id, link.target_type
        )),
    )
}

/// One phase row on the wire.
pub fn phase_dto(row: &ChronologyPhaseRow) -> TimelinePhaseDto {
    TimelinePhaseDto {
        id: row.id.clone(),
        label: row.label.clone(),
        date_range: row.date_range.clone(),
        color: row.color.clone(),
        description: row.description.clone(),
        sort_order: row.sort_order,
    }
}

/// One event row on the wire, with the links and note count it was given.
fn event_dto(
    row: &ChronologyEventRow,
    links: Vec<TimelineLinkDto>,
    note_count: i64,
    warnings: &mut Vec<String>,
) -> TimelineEventDto {
    let (tags, tags_odd) = tags_of(&row.attributes);
    if tags_odd {
        warnings.push(format!(
            "event {}: attributes.tags is not an array of strings; no tags shown",
            row.id
        ));
    }
    let (phase, phase_odd) = phase_of(&row.attributes);
    if phase_odd {
        warnings.push(format!(
            "event {}: attributes.phase is not a string; no phase shown",
            row.id
        ));
    }

    TimelineEventDto {
        id: row.id,
        event_date: row.event_date,
        date_precision: row.date_precision.clone(),
        approximate: row.approximate,
        title: row.title.clone(),
        fact: row.fact.clone(),
        attributes: row.attributes.clone(),
        phase,
        tags,
        links,
        note_count,
        created_by: row.created_by.clone(),
        created_at: row.created_at,
        updated_by: row.updated_by.clone(),
        updated_at: row.updated_at,
    }
}

/// Turn one event's link rows into wire links, collecting warnings.
fn link_dtos(
    rows: &[ChronologyLinkRow],
    resolved_documents: &HashSet<String>,
    warnings: &mut Vec<String>,
) -> Vec<TimelineLinkDto> {
    rows.iter()
        .map(|link| {
            let (resolves, warning) = resolve_link(link, resolved_documents);
            if let Some(w) = warning {
                warnings.push(w);
            }
            TimelineLinkDto {
                target_type: link.target_type.clone(),
                target_id: link.target_id.clone(),
                label: link.label.clone(),
                pinpoint: link.pinpoint.clone(),
                resolves,
            }
        })
        .collect()
}

/// The whole `GET /api/timeline` payload.
///
/// `links` is every link for the case in one flat list, and `note_counts` holds
/// only the events that HAVE notes — an event missing from the map has zero,
/// which is the same fact with fewer rows.
pub fn build_timeline(
    phases: &[ChronologyPhaseRow],
    events: &[ChronologyEventRow],
    links: &[ChronologyLinkRow],
    note_counts: &HashMap<Uuid, i64>,
    resolved_documents: &HashSet<String>,
) -> Composed<TimelineDto> {
    let mut warnings = Vec::new();
    let mut by_event: HashMap<Uuid, Vec<ChronologyLinkRow>> = HashMap::new();
    for link in links {
        by_event
            .entry(link.event_id)
            .or_default()
            .push(link.clone());
    }

    let events = events
        .iter()
        .map(|row| {
            let rows = by_event.get(&row.id).map(Vec::as_slice).unwrap_or(&[]);
            let link_dtos = link_dtos(rows, resolved_documents, &mut warnings);
            let count = note_counts.get(&row.id).copied().unwrap_or(0);
            event_dto(row, link_dtos, count, &mut warnings)
        })
        .collect();

    Composed {
        payload: TimelineDto {
            phases: phases.iter().map(phase_dto).collect(),
            events,
        },
        warnings,
    }
}

/// The whole `GET /api/timeline/events/{id}` payload.
pub fn build_event_detail(
    event: &ChronologyEventRow,
    links: &[ChronologyLinkRow],
    notes: &[ChronologyNoteRow],
    history: &[ChronologyHistoryRow],
    resolved_documents: &HashSet<String>,
) -> Composed<TimelineEventDetailDto> {
    let mut warnings = Vec::new();
    let link_dtos = link_dtos(links, resolved_documents, &mut warnings);
    let dto = event_dto(event, link_dtos, notes.len() as i64, &mut warnings);

    Composed {
        payload: TimelineEventDetailDto {
            event: dto,
            notes: notes
                .iter()
                .map(|n| TimelineNoteDto {
                    id: n.id,
                    note: n.note.clone(),
                    created_by: n.created_by.clone(),
                    created_at: n.created_at,
                })
                .collect(),
            history: history
                .iter()
                .map(|h| TimelineHistoryDto {
                    id: h.id,
                    action: h.action.clone(),
                    snapshot: h.snapshot.clone(),
                    changed_by: h.changed_by.clone(),
                    changed_at: h.changed_at,
                })
                .collect(),
        },
        warnings,
    }
}

#[cfg(test)]
#[path = "chronology_read_tests.rs"]
mod tests;
