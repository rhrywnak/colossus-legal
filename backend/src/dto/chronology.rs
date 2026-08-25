//! Wire shapes for the chronology read endpoints (task A3).
//!
//! ## THE CHANGE RULE LIVES IN THESE TYPES (design R4)
//!
//! "Additive only, absence tolerated. No chronology field is ever REQUIRED after
//! day one; old rows never need migrating to satisfy new code." In practice that
//! means three habits, all visible below:
//!
//! 1. `attributes` travels as a whole `serde_json::Value`. A key this build has
//!    never heard of reaches the frontend intact instead of being dropped by a
//!    typed struct that did not know about it.
//! 2. Everything DERIVED from `attributes` is `Option` or defaulted — never a
//!    required field. A row whose bag is `{}` deserialises fine and renders as
//!    an event with no tags and no phase, which is a real state, not an error.
//! 3. `#[serde(default)]` on every optional field, so a payload stored by an
//!    older build still parses if it is ever read back.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One phase, as `chronology_phases` holds it.
///
/// This is what replaces `timeline.json` as the label source for the five
/// non-timeline surfaces that read it today (design R15).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelinePhaseDto {
    pub id: String,
    pub label: String,
    pub date_range: String,
    pub color: String,
    /// Rendered as a muted subtitle under the phase header (design R14).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub sort_order: i32,
}

/// One link from an event to its evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineLinkDto {
    pub target_type: String,
    pub target_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// `None` is MEANINGFUL and the surface marks it "no pinpoint" (design R9).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinpoint: Option<String>,
    /// Whether the target exists in its store, computed at read time.
    ///
    /// This field is the whole reason the redesign happened: ten of the eleven
    /// links in the old JSON pointed at ids that did not exist, and the page
    /// rendered every one as a live link because nothing ever asked. A dead link
    /// is DATA the frontend renders as "no document" — never a 500, and never
    /// silently dropped from the list.
    pub resolves: bool,
}

/// One dated fact, with its links and how many notes it carries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEventDto {
    pub id: Uuid,
    pub event_date: NaiveDate,
    pub date_precision: String,
    pub approximate: bool,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fact: Option<String>,
    /// The whole bag, verbatim. Nothing is dropped on the way out.
    pub attributes: serde_json::Value,
    /// Derived from `attributes.phase`. `None` when the bag has no phase — a
    /// real state (an event nobody has filed yet), not a failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    /// Derived from `attributes.tags`. Empty when there are none, or when the
    /// key holds something that is not an array of strings — see
    /// `services::chronology_read` for why that degradation is logged, not silent.
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub links: Vec<TimelineLinkDto>,
    /// How many live notes this event carries, for the card's badge.
    #[serde(default)]
    pub note_count: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_by: Option<String>,
    pub updated_at: DateTime<Utc>,
}

/// The whole page in one read: the phases in order, the events by date.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineDto {
    pub phases: Vec<TimelinePhaseDto>,
    pub events: Vec<TimelineEventDto>,
}

/// One attributed note (design R8).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineNoteDto {
    pub id: Uuid,
    pub note: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// One history entry.
///
/// Always an empty list in Phase A — nothing writes history until the write
/// endpoints land in Phase C. Empty and absent are different observables: the
/// field is always present, and an empty list means "no changes recorded".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineHistoryDto {
    pub id: Uuid,
    pub action: String,
    pub snapshot: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changed_by: Option<String>,
    pub changed_at: DateTime<Utc>,
}

/// One event, in full: everything the event page renders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEventDetailDto {
    #[serde(flatten)]
    pub event: TimelineEventDto,
    #[serde(default)]
    pub notes: Vec<TimelineNoteDto>,
    #[serde(default)]
    pub history: Vec<TimelineHistoryDto>,
}
