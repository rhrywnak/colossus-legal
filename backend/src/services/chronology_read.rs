//! Composing the chronology read payloads. Pure — no database, no clock.
//!
//! Every sentence the timeline endpoints speak is decided here, so a test can
//! reach it without a Postgres. The handlers in `api::timeline` fetch rows and
//! hand them over; this module decides what the frontend sees.
//!
//! ## Degradation is REPORTED, never silent (Standing Rule 1)
//!
//! One thing can be "wrong" with a stored row without being an error: an
//! `attributes` bag whose `tags` key is not an array of strings. It should not
//! fail a request — a chronology that refuses to render because one row has an
//! odd bag is worse than one that renders it plainly — so it returns a WARNING
//! alongside the payload, and the handler logs it with the event's id.
//!
//! A link whose `target_type` this build has no resolver for is NOT a
//! degradation. It is a third answer, and since 2026-08-25 it has its own name
//! on the wire: `LinkResolution::Unchecked`. Reporting it as "missing" would
//! have been a claim nobody checked; reporting it as a warning would have
//! implied something was wrong. It is simply not known.
//!
//! ## The phase is read from its COLUMN
//!
//! Not from `attributes`. Ruled 2026-08-25: `chronology_events.phase` is a real
//! `NOT NULL` column with a foreign key, and there is no bag mirror to fall back
//! to or to disagree with.

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::dto::chronology::{
    LinkResolution, TimelineDto, TimelineEventDetailDto, TimelineEventDto, TimelineHistoryDto,
    TimelineLinkDto, TimelineNoteDto, TimelinePhaseDto,
};
use crate::repositories::pipeline_repository::chronology::{
    ChronologyEventRow, ChronologyHistoryRow, ChronologyLinkRow, ChronologyNoteRow,
    ChronologyPhaseRow,
};

/// The `target_type` this build knows how to check.
///
/// ## Why this is one `&str` and not a list
///
/// A list would let a target type be DECLARED checkable without a resolver
/// existing for it, and the two would drift the first time someone added a name
/// and forgot the function. Resolving a `statement` or a `paperless_document`
/// means new code — a new query against a different store — so the day a second
/// kind becomes checkable, the signature changing is a feature: it makes the
/// compiler point at every place that has to learn the new answer.
// CONST: this build's resolver CAPABILITY, which is defined by the code it
// contains and not by its configuration. There is nothing here an operator could
// usefully change without also shipping the resolver that backs it.
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

/// What this build can say about one link's target.
///
/// Three answers, never two. `Missing` means the target was looked for in its
/// store and is not there; `Unchecked` means this build has no resolver for that
/// `target_type` and did not look. Phase A only creates `document` links, so
/// `Unchecked` is unreachable today — but the day Phase C adds a `statement` or
/// a `paperless_document` target, the difference is the difference between "this
/// document is gone" and "we cannot see that store from here".
pub fn resolve_link(
    link: &ChronologyLinkRow,
    resolved_documents: &HashSet<String>,
) -> LinkResolution {
    if link.target_type != CHECKABLE_TARGET_TYPE {
        return LinkResolution::Unchecked;
    }
    if resolved_documents.contains(&link.target_id) {
        return LinkResolution::Resolves;
    }
    LinkResolution::Missing
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
    TimelineEventDto {
        id: row.id,
        event_date: row.event_date,
        date_precision: row.date_precision.clone(),
        approximate: row.approximate,
        phase: row.phase.clone(),
        title: row.title.clone(),
        fact: row.fact.clone(),
        attributes: row.attributes.clone(),
        tags,
        links,
        note_count,
        created_by: row.created_by.clone(),
        created_at: row.created_at,
        updated_by: row.updated_by.clone(),
        updated_at: row.updated_at,
    }
}

/// Turn one event's link rows into wire links.
fn link_dtos(
    rows: &[ChronologyLinkRow],
    resolved_documents: &HashSet<String>,
) -> Vec<TimelineLinkDto> {
    rows.iter()
        .map(|link| TimelineLinkDto {
            target_type: link.target_type.clone(),
            target_id: link.target_id.clone(),
            label: link.label.clone(),
            pinpoint: link.pinpoint.clone(),
            resolution: resolve_link(link, resolved_documents),
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
            let link_dtos = link_dtos(rows, resolved_documents);
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
    let link_dtos = link_dtos(links, resolved_documents);
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

#[cfg(test)]
#[path = "chronology_read_detail_tests.rs"]
mod detail_tests;
