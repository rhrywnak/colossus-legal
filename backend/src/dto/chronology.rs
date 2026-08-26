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

use crate::dto::chronology_wording::ChronologyWordingDto;
use uuid::Uuid;

/// One phase, as `chronology_phases` holds it.
///
/// This is what replaces `timeline.json` as the label source for the five
/// non-timeline surfaces that read it today (design R15).
// serde: allows unknown fields because a chronology payload is additive by design R4; a
// field added by a newer build must not fail an older reader.
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

/// What this build was able to say about a link's target.
///
/// ## Why three states and not a `bool`
///
/// Ruled 2026-08-25 (report v1, R-E). `resolves: bool` could only say yes or no,
/// and for a target kind this build has no resolver for — nine of the design's
/// ten — "no" would have been a claim nobody had checked. Three states keep the
/// two kinds of negative apart: `Missing` is an answer, `Unchecked` is the
/// absence of one, and a surface can render them differently ("no document" vs
/// "not checked"). Phase A only ever creates `document` links, so `Unchecked`
/// is unreachable today; the wire shape is settled now because it is cheaper to
/// widen before a frontend reads it than after.
///
/// ## Rust Learning: `#[serde(rename_all = "snake_case")]` on a fieldless enum
///
/// Each variant becomes a lower-case string on the wire — `"resolves"`,
/// `"missing"`, `"unchecked"` — so the TypeScript union is spelled the same as
/// the Rust enum with no per-variant attributes and no hand-written mapping. An
/// unknown token fails to deserialise rather than defaulting, which is the loud
/// boundary Standing Rule 1 asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkResolution {
    /// The target exists in its store.
    Resolves,
    /// The target was looked for and is not there. A real answer: the surface
    /// renders "no document", and the event stays in the list.
    Missing,
    /// This build has no resolver for that `target_type`, so nothing was
    /// checked. Never presented as though it were `Missing`.
    Unchecked,
}

/// One tag of the case's vocabulary, as the filter bar renders it.
// serde: allows unknown fields because a chronology payload is additive by
// design R4; a field added by a newer build must not fail an older reader.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineTagDto {
    pub id: String,
    pub label: String,
    pub color: String,
    pub sort_order: i32,
}

/// One link from an event to its evidence.
// serde: allows unknown fields because a chronology payload is additive by design R4; a
// field added by a newer build must not fail an older reader.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineLinkDto {
    pub target_type: String,
    pub target_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// `None` is MEANINGFUL and the surface marks it "no pinpoint" (design R9).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinpoint: Option<String>,
    /// What this build could say about the target, computed at read time.
    ///
    /// This field is the whole reason the redesign happened: ten of the eleven
    /// links in the old JSON pointed at ids that did not exist, and the page
    /// rendered every one as a live link because nothing ever asked. A dead link
    /// is DATA the frontend renders as "no document" — never a 500, and never
    /// silently dropped from the list.
    pub resolution: LinkResolution,
}

/// One dated fact, with its links and how many notes it carries.
// serde: allows unknown fields because THE CHANGE RULE (design R4) is this type's whole
// point — a field added by a newer build must not fail an older reader.
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
    /// The phase slug, straight from the column.
    ///
    /// Not an `Option` and not derived from `attributes`: ruled 2026-08-25, the
    /// phase is a real `NOT NULL` column with a foreign key to
    /// `chronology_phases`, and there is no bag mirror. Every event has exactly
    /// one phase, and the database guarantees it is a real one.
    pub phase: String,
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
    /// When this event was soft-deleted, if it was (design R10).
    ///
    /// ## Why a READ shape carries a field the reads never set
    ///
    /// The list and the event page never return a deleted event, so this is
    /// always absent there. It is present because the WRITE endpoints return
    /// this same shape, and a DELETE's response is the event it just deleted —
    /// which is what lets the surface replace the card IN PLACE with the undo
    /// line rather than guessing from an HTTP status that the row is now gone.
    ///
    /// Absent and null are the same here (`skip_serializing_if`), and both mean
    /// live. That is safe in a way it would not be for `fact`, because "deleted
    /// at no time" has exactly one meaning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
}

/// The whole page in one read: the phases in order, the events by date.
// serde: allows unknown fields because a chronology payload is additive by design R4; a
// field added by a newer build must not fail an older reader.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineDto {
    pub phases: Vec<TimelinePhaseDto>,
    /// The tag vocabulary, from `chronology_tags` (ruling R-F).
    ///
    /// Served rather than hardcoded so the filter chips ARE the stored
    /// vocabulary: adding a sixth tag is a row, not a build.
    #[serde(default)]
    pub tags: Vec<TimelineTagDto>,
    pub events: Vec<TimelineEventDto>,
    /// Every string these surfaces speak, from the settings store.
    ///
    /// Rides this payload because the page cannot render a row without the read
    /// anyway — a second request for twenty-nine strings fired at the same
    /// instant would buy nothing.
    pub wording: ChronologyWordingDto,
    /// How many events a phase's scroll window shows before it scrolls (R6).
    #[serde(default)]
    pub phase_window_events: usize,
}

/// One attributed note (design R8).
// serde: allows unknown fields because a chronology payload is additive by design R4; a
// field added by a newer build must not fail an older reader.
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
// serde: allows unknown fields because a chronology payload is additive by design R4; a
// field added by a newer build must not fail an older reader.
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
// serde: allows unknown fields because `#[serde(flatten)]` below is documented by serde
// as incompatible with deny_unknown_fields — the attribute cannot be applied here
// even if the change rule did not already forbid it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEventDetailDto {
    #[serde(flatten)]
    pub event: TimelineEventDto,
    #[serde(default)]
    pub notes: Vec<TimelineNoteDto>,
    #[serde(default)]
    pub history: Vec<TimelineHistoryDto>,
}
