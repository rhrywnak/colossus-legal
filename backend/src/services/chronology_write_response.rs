//! What a chronology WRITE answers with (Phase C, §C3).
//!
//! ## Why every write returns the whole event
//!
//! §C3: "After any write, the list/page reflects the server's response — no
//! optimistic divergence." A surface that applied its own guess of the new state
//! and only asked the server for a status code would drift the first time the
//! server normalised something — a trimmed title, a cleared fact, a tag order —
//! and the drift would show up as a page that disagreed with itself after a
//! reload. Returning the composed event closes that by construction: there is
//! nothing for the surface to guess.
//!
//! ## Why this is not in `chronology_read`
//!
//! That module composes what a READ returns and takes `ChronologyEventRow`,
//! whose whole promise is that it never carries a deleted row. A write's
//! response must be able to carry one — the answer to a DELETE is the event it
//! just deleted, which is what lets the card be replaced in place by the undo
//! line. Rather than widen the read type and weaken its promise, this module
//! adapts the write path's row and hands the rest to `build_event_detail`, so
//! the two responses are composed by the same code and cannot disagree about
//! how a link or a note reads.

use std::collections::HashSet;

use crate::dto::chronology::TimelineEventDetailDto;
use crate::repositories::pipeline_repository::chronology::{
    ChronologyEventRow, ChronologyHistoryRow, ChronologyLinkRow, ChronologyNoteRow,
};
use crate::repositories::pipeline_repository::chronology_write::ChronologyEventStateRow;
use crate::services::chronology_read::{build_event_detail, Composed};

/// The read module's row shape, from the write module's.
///
/// ## Rust Learning: a conversion function rather than a `From` impl
///
/// `From<&ChronologyEventStateRow> for ChronologyEventRow` would be tidier to
/// call and worse to read: it would make it look as though a write row simply IS
/// a read row, when the difference between them — `deleted_at` — is the whole
/// reason both exist. A named function that a caller has to reach for keeps the
/// lossy step visible at the one place it happens.
pub(crate) fn as_read_row(row: &ChronologyEventStateRow) -> ChronologyEventRow {
    ChronologyEventRow {
        id: row.id,
        case_slug: row.case_slug.clone(),
        event_date: row.event_date,
        date_precision: row.date_precision.clone(),
        approximate: row.approximate,
        phase: row.phase.clone(),
        title: row.title.clone(),
        fact: row.fact.clone(),
        attributes: row.attributes.clone(),
        created_by: row.created_by.clone(),
        created_at: row.created_at,
        updated_by: row.updated_by.clone(),
        updated_at: row.updated_at,
    }
}

/// Everything one write response composes from.
///
/// A parameter struct for the same reason `TimelineSources` is one: five inputs,
/// three of them slices of different row types and one a set of strings, is more
/// positions than a reader tracks at a call site.
#[derive(Debug, Clone, Copy)]
pub struct WriteResponseSources<'a> {
    pub event: &'a ChronologyEventStateRow,
    pub links: &'a [ChronologyLinkRow],
    pub notes: &'a [ChronologyNoteRow],
    pub history: &'a [ChronologyHistoryRow],
    /// Which linked document ids exist, from one query.
    pub resolved_documents: &'a HashSet<String>,
}

/// The event a write just wrote, in the shape the event page already reads.
///
/// The only thing this adds to `build_event_detail` is `deleted_at`, and it adds
/// it AFTER composition rather than threading it through: the read path has no
/// use for the field, and giving it one would mean every read had to remember to
/// set it to `None`.
pub fn build_write_response(sources: WriteResponseSources<'_>) -> Composed<TimelineEventDetailDto> {
    let read_row = as_read_row(sources.event);
    let mut composed = build_event_detail(
        &read_row,
        sources.links,
        sources.notes,
        sources.history,
        sources.resolved_documents,
    );
    composed.payload.event.deleted_at = sources.event.deleted_at;
    composed
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, NaiveDate, Utc};

    fn state_row(deleted: bool) -> ChronologyEventStateRow {
        let at: DateTime<Utc> = "2026-08-26T10:00:00Z".parse().expect("a real timestamp");
        ChronologyEventStateRow {
            id: uuid::uuid!("11111111-2222-3333-4444-555555555555"),
            case_slug: "awad".to_string(),
            event_date: NaiveDate::from_ymd_opt(2012, 4, 12).expect("a real date"),
            date_precision: "day".to_string(),
            approximate: false,
            phase: "appeals".to_string(),
            title: "Judge Tighe Issues Post-Appeal Order".to_string(),
            fact: None,
            attributes: serde_json::json!({ "tags": ["court_action"] }),
            created_by: Some("roman".to_string()),
            created_at: at,
            updated_by: None,
            updated_at: at,
            deleted_at: if deleted { Some(at) } else { None },
        }
    }

    fn sources<'a>(
        event: &'a ChronologyEventStateRow,
        empty: &'a HashSet<String>,
    ) -> WriteResponseSources<'a> {
        WriteResponseSources {
            event,
            links: &[],
            notes: &[],
            history: &[],
            resolved_documents: empty,
        }
    }

    #[test]
    fn a_live_events_response_carries_no_deleted_at() {
        let row = state_row(false);
        let empty = HashSet::new();
        let composed = build_write_response(sources(&row, &empty));
        assert_eq!(composed.payload.event.deleted_at, None);
        assert_eq!(
            composed.payload.event.title,
            "Judge Tighe Issues Post-Appeal Order"
        );
        // Composed by the same code as a read: tags come out of the bag.
        assert_eq!(
            composed.payload.event.tags,
            vec!["court_action".to_string()]
        );
    }

    #[test]
    fn a_deleted_events_response_carries_the_deletion_so_the_undo_line_can_draw() {
        // ⚑ Without this the surface would have to infer "it is gone now" from a
        // 200, which is the optimistic divergence §C3 forbids.
        let row = state_row(true);
        let empty = HashSet::new();
        let composed = build_write_response(sources(&row, &empty));
        assert!(
            composed.payload.event.deleted_at.is_some(),
            "a delete's response must say the event is deleted"
        );
    }

    #[test]
    fn the_deleted_field_survives_serialization_only_when_it_is_set() {
        // `skip_serializing_if` means a live event's payload has no key at all,
        // which is what keeps the read payload byte-identical to Phase B's.
        let live = build_write_response(sources(&state_row(false), &HashSet::new())).payload;
        let value = serde_json::to_value(&live).expect("serializes");
        assert!(
            value.get("deleted_at").is_none(),
            "a live event must not carry the key: {value}"
        );

        let gone = build_write_response(sources(&state_row(true), &HashSet::new())).payload;
        let value = serde_json::to_value(&gone).expect("serializes");
        assert!(value.get("deleted_at").is_some(), "got: {value}");
    }
}
