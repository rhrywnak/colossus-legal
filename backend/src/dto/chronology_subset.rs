//! Wire shapes for timeline subsets — reads and writes (T1.3).
//!
//! TIMELINE_SUBSET_DESIGN_v1 §4. A subset is a named, ordered list of
//! REFERENCES to events that already live in the one case chronology.
//!
//! ## ⚑ THERE IS NO SECOND EVENT SHAPE
//!
//! [`SubsetEventDto`] does not describe an event. It WRAPS
//! [`TimelineEventDto`] — the exact type `GET /api/timeline` already returns —
//! and adds the two facts that belong to the subset rather than to the event:
//! the author's one-line note, and whether the event has since been removed from
//! the chronology. That is the design's "references, never copies" rule as a
//! Rust type: there is nowhere in this module for a copy of a title or a date to
//! live, so a subset cannot render an event differently from the timeline.
//!
//! A flattened shape would have read more tidily on the wire and would have made
//! that impossible to keep true: `#[serde(flatten)]` would let a field be added
//! here that shadowed one of the event's, and nothing would fail.
//!
//! ## Reads tolerate unknown fields; requests do not
//!
//! The same split `dto::chronology` and `dto::chronology_write` make, for the
//! same reasons. A payload is additive forever (chronology R4); a request is a
//! form this build drew, and an unknown key in one is a typo or a stale tab.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::dto::chronology::TimelineEventDto;
use crate::dto::chronology_wording::ChronologyWordingDto;

/// One subset as the home section's list renders it.
///
/// # Domain note — why the counts are two numbers and not one
///
/// `event_count` is how many events the subset references; `gap_count` is how
/// many of those have been soft-deleted on the chronology. A single total over a
/// list that shows some lines struck through is the sentence that makes a reader
/// distrust the count — which is why the window's footer wording template takes
/// both.
// serde: allows unknown fields because a chronology payload is additive by
// design R4; a field added by a newer build must not fail an older reader.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsetSummaryDto {
    pub id: Uuid,
    pub name: String,
    /// Never `Option`: the column is `NOT NULL DEFAULT ''`, because "no
    /// description" and "an empty description" are one state for a field a human
    /// types into.
    pub description: String,
    /// How many events this subset references, gaps included.
    pub event_count: i64,
    /// How many of those have been removed from the chronology.
    pub gap_count: i64,
    /// The scenario codes carrying this subset — `["S-11", "S-12"]` — in the
    /// order the scenarios attached it. Empty when nothing carries it.
    #[serde(default)]
    pub carried_by: Vec<String>,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_by: String,
    pub updated_at: DateTime<Utc>,
}

/// One event IN a subset: the timeline's own event shape, plus what the subset
/// knows about it.
// serde: allows unknown fields for the same forward-compatibility reason.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsetEventDto {
    /// The event exactly as the timeline renders it — links, tags, note count,
    /// phase. Not a copy: composed from `chronology_events` at read time.
    pub event: TimelineEventDto,
    /// The author's one line on why this event is in this story. `""` when they
    /// wrote none.
    pub subset_note: String,
    /// The event has been soft-deleted on the chronology (design R1).
    ///
    /// ## Domain note: the row is MARKED, never dropped
    ///
    /// Dropping it would silently shorten a story somebody counted, and the gap
    /// is half the value of a subset — it is the story saying "this happened and
    /// it is not on our timeline yet". The Undo is not here; it is on the
    /// timeline, on the event itself, which is what the wording row says.
    pub removed: bool,
}

/// One subset with its events, in story order.
// serde: allows unknown fields for the same forward-compatibility reason.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsetDetailDto {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    /// Ordered by `position`, then by `(event_date, date_precision, id)` for any
    /// two that somehow share one — a tie that cannot happen while the unique
    /// constraint holds, broken deterministically anyway so two reads of
    /// unchanged data never disagree.
    #[serde(default)]
    pub events: Vec<SubsetEventDto>,
    /// The scenario codes carrying this subset, in attachment order.
    #[serde(default)]
    pub carried_by: Vec<String>,
    pub event_count: i64,
    pub gap_count: i64,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_by: String,
    pub updated_at: DateTime<Utc>,
    /// When this subset was soft-deleted, if it was. Absent means live.
    ///
    /// Present for the same reason `TimelineEventDto` carries one: the DELETE
    /// endpoint answers with the subset it just deleted, so a surface draws the
    /// undo line from the server's account rather than from a status code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
}

/// One subset attached to a scenario — the View Timeline button's data.
///
/// Deliberately smaller than [`SubsetSummaryDto`]: the button needs a name and
/// two counts, and the window fetches the whole subset when it opens. Sending
/// the full summary on every scenario view would put a subset's authorship
/// metadata on five pages that never render it.
// serde: allows unknown fields for the same forward-compatibility reason.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioSubsetDto {
    pub id: Uuid,
    pub name: String,
    pub event_count: i64,
    pub gap_count: i64,
    /// The order this scenario carries its subsets in — the window's selector.
    pub position: i32,
}

/// What `GET /cases/:slug/scenarios/:id/subsets` answers with.
///
/// ## ⚑ Why the WORDS ride this read
///
/// The dock that draws the View Timeline button and the floating window is
/// mounted on five scenario surfaces that share no header component and no
/// read between them — the T3 report carries what each of them actually calls.
/// It is self-contained by design: it takes a case slug and a scenario id and
/// nothing else, so no page has to learn about it and no page's own read
/// changes.
///
/// That leaves one question — where its WORDS come from. They ride here, on the
/// read the dock already has to make to know whether to draw anything at all.
/// One field, carrying the SAME [`ChronologyWordingDto`] that
/// `GET /api/timeline` serves: not a second shape and not a subset of one, so a
/// row edited once is edited for both surfaces and the two cannot drift.
///
/// The cost, stated rather than hidden: a practice page now carries two wording
/// vocabularies — its own, and this block inside the dock. Ruled acceptable on
/// 2026-08-30 (two components, two blocks), and recorded in the T3 report.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioSubsetsDto {
    /// The subsets this scenario carries, in attachment order.
    ///
    /// `[]` hides the button, and is deliberately NOT a 404 — that would mean
    /// "there is no such scenario". A surface collapsing the two would draw a
    /// working page for a scenario that does not exist.
    pub subsets: Vec<ScenarioSubsetDto>,
    /// Every word the dock speaks, as the timeline serves them.
    pub wording: ChronologyWordingDto,
}

/// One event's place in a subset, as a request states it.
///
/// Used by both the create body and the replace body, because they are the same
/// fact: which event, where in the story, and why. Unlike the event create/edit
/// pair, there is no field one of them may carry and the other may not.
// serde: deny_unknown_fields because this is a request from a form this build
// drew, not a stored row a future build may have widened.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubsetEventRef {
    pub event_id: Uuid,
    /// The story order. Any distinct integers will do — the picker sends
    /// 1, 2, 3, and the read orders by this column, never by the values'
    /// magnitude relative to any other subset's.
    pub position: i32,
    /// Absent and `""` are the same thing here, unlike a link's pinpoint: an
    /// unwritten note is not marked on any screen.
    #[serde(default)]
    pub note: Option<String>,
}

/// Create one subset. `POST /api/timeline/subsets`.
// serde: deny_unknown_fields — a request, not a stored row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateSubsetRequest {
    /// Required and non-blank. A story with no name cannot be picked out of a
    /// list, and the live-name uniqueness index has nothing to index.
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    /// May be absent or empty: naming a story before choosing its events is a
    /// real thing an author does, and the picker is a second screen.
    #[serde(default)]
    pub events: Option<Vec<SubsetEventRef>>,
}

/// Rename or re-describe one subset. `PUT /api/timeline/subsets/:id`.
///
/// ## Rust Learning: two `Option`s that mean "leave it alone"
///
/// Both fields are optional and an absent one means "do not touch". That is the
/// same discipline `UpdateEventRequest` applies to `tags`, and it matters here
/// because the two fields are edited from two different places — a rename from
/// the list, a description from the form — and a request that had to send both
/// would let one screen silently clear what the other wrote.
// serde: deny_unknown_fields — a request, not a stored row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateSubsetRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// REPLACE a subset's ordered event set. `PUT /api/timeline/subsets/:id/events`.
///
/// ## ⚑ THIS STRUCT IS THE FIX FOR THE 422, AND WHY IT IS A STRUCT
///
/// The handler took `Json<Vec<SubsetEventRef>>` — a BARE TOP-LEVEL ARRAY —
/// while every client sent `{"events": [...]}`. Axum's `Json` extractor refused
/// the map before any handler code ran, with
///
///   422 "Failed to deserialize the JSON body into the target type:
///        invalid type: map, expected a sequence at line 1 column 0"
///
/// which meant the endpoint had NEVER worked: no save of the picker, of any
/// shape, had ever reached the database. It surfaced as a rename bug only
/// because a rename makes the first call succeed, so the reader sees half a save.
///
/// The envelope wins rather than the array, for three reasons. It is what every
/// other request in this module already is — `CreateSubsetRequest`,
/// `UpdateSubsetRequest` and `AttachSubsetRequest` are all named structs with
/// `deny_unknown_fields`, and `CreateSubsetRequest` already carries an `events`
/// field of the same name and element type. A bare top-level array cannot gain a
/// sibling field later without breaking every client. And there was no
/// compatibility cost either way, because nothing had ever successfully called
/// this endpoint.
// serde: deny_unknown_fields — a request, not a stored row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplaceSubsetEventsRequest {
    /// The COMPLETE ordered set. T1's replace semantics: one human act, one
    /// write, one history row — never a per-row add or remove.
    pub events: Vec<SubsetEventRef>,
}

/// Attach one subset to a scenario.
/// `POST /api/cases/:slug/scenarios/:scenario_id/subsets`.
// serde: deny_unknown_fields — a request, not a stored row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachSubsetRequest {
    pub subset_id: Uuid,
}

#[cfg(test)]
mod wire_contract_tests {
    use super::*;

    /// ⚑ THE GUARD FOR THE DEFECT THAT COST T6 ITS FIRST HOUR.
    ///
    /// `PUT /api/timeline/subsets/:id/events` took a bare `Vec<SubsetEventRef>`
    /// while every client sent `{"events": [...]}`. Axum refused the map with a
    /// 422 before the handler ran, so the endpoint had NEVER worked — no save of
    /// the picker had ever reached the database.
    ///
    /// It survived because BOTH SIDES WERE TESTED AND NEITHER TEST CROSSED THE
    /// WIRE. `caseTimelineSubsets.test.ts` stubs `fetch` and asserts the body it
    /// sent has an `events` array; the Rust tests construct the handler's input
    /// as a Rust value. Each was green about a different contract.
    ///
    /// So this test does the one thing neither did: it deserializes the EXACT
    /// BYTES the frontend sends. If `replaceSubsetEvents` changes shape, or this
    /// struct does, one of them fails here rather than in front of Roman.
    #[test]
    fn the_events_request_parses_the_exact_body_the_frontend_sends() {
        // Copied verbatim from the harness capture on 2026-08-31, trimmed to two
        // refs. The full 15-ref body differs only in length.
        let body = r#"{"events":[{"event_id":"526cb85f-6c05-4b5a-a4cc-37ee278f5b02","position":1},{"event_id":"bdd7c5ba-b1aa-43d2-99f1-46f819d396db","position":2}]}"#;

        let parsed: ReplaceSubsetEventsRequest =
            serde_json::from_str(body).expect("the body the frontend actually sends must parse");

        assert_eq!(parsed.events.len(), 2);
        assert_eq!(parsed.events[0].position, 1);
        assert_eq!(
            parsed.events[0].event_id.to_string(),
            "526cb85f-6c05-4b5a-a4cc-37ee278f5b02"
        );
    }

    /// A note rides through, because the picker sends one when an author wrote one.
    #[test]
    fn a_ref_carries_its_note() {
        let body = r#"{"events":[{"event_id":"526cb85f-6c05-4b5a-a4cc-37ee278f5b02","position":1,"note":"the handoff"}]}"#;
        let parsed: ReplaceSubsetEventsRequest = serde_json::from_str(body).expect("parses");
        assert_eq!(parsed.events[0].note.as_deref(), Some("the handoff"));
    }

    /// The BARE ARRAY is refused now, which is the shape that used to be required.
    ///
    /// Asserted so nobody "restores" it: a client sending the old shape gets a
    /// clean refusal rather than silently writing through a second accepted form.
    #[test]
    fn a_bare_array_is_no_longer_accepted() {
        let body = r#"[{"event_id":"526cb85f-6c05-4b5a-a4cc-37ee278f5b02","position":1}]"#;
        assert!(serde_json::from_str::<ReplaceSubsetEventsRequest>(body).is_err());
    }

    /// An empty set is legal: clearing a story is a thing an author does.
    #[test]
    fn an_empty_set_is_legal() {
        let parsed: ReplaceSubsetEventsRequest =
            serde_json::from_str(r#"{"events":[]}"#).expect("parses");
        assert!(parsed.events.is_empty());
    }

    /// `deny_unknown_fields` holds, so a typo'd field is a refusal and not a
    /// silently ignored half-request.
    #[test]
    fn an_unknown_field_is_refused() {
        let body = r#"{"events":[],"evnets":[]}"#;
        assert!(serde_json::from_str::<ReplaceSubsetEventsRequest>(body).is_err());
    }
}
