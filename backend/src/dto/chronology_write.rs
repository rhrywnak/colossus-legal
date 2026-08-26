//! Wire shapes for the chronology's WRITE endpoints (Phase C, §C1).
//!
//! ## ⚑ THE CHANGE RULE CUTS THE OTHER WAY ON A REQUEST
//!
//! `dto::chronology` tolerates unknown fields because it READS rows an older or
//! newer build wrote — absence tolerated, additive forever (design R4). A
//! REQUEST is the opposite situation: it is a form this build drew, submitted by
//! a browser this build shipped, and a field neither of them knows about is not
//! forward compatibility, it is a typo or a stale tab. So every request struct
//! below carries `deny_unknown_fields`, and a misspelled key is a 400 naming it
//! rather than a value silently dropped on the floor.
//!
//! The one thing that stays additive is `tags`: it lands in the JSONB bag, and
//! the bag is where the change rule lives.
//!
//! ## Optional means "not supplied", never "empty"
//!
//! Every optional field is `Option<T>` with `#[serde(default)]`, and the
//! difference between an absent field and an empty one is preserved all the way
//! into `services::chronology_validate`: on an edit, an absent `fact` and a
//! `fact` of `""` are two different instructions ("leave it" versus "clear it"),
//! and collapsing them would make one of the two impossible to express.

use serde::{Deserialize, Serialize};

/// Create one event. `POST /api/timeline/events`.
///
/// # Domain note — what R11 makes required, forever
///
/// Date and title, and nothing else. "One-sentence fact encouraged but
/// optional." Everything below except `event_date`, `title` and `phase` may be
/// omitted, and omitting them all produces a real, renderable event.
///
/// `phase` is required HERE and not by R11, and the reason is the schema rather
/// than the design: `chronology_events.phase` is `NOT NULL` with a foreign key
/// onto `chronology_phases` (Phase A, ruled 2026-08-25), so there is no value
/// this build could store for an event whose author named no phase. The form
/// always sends one — the mockup's Phase select has no blank option. See the
/// Phase C report's NEEDS A RULING for the one migration that would make it
/// genuinely optional.
// serde: deny_unknown_fields because this is a request from a form this build
// drew, not a stored row a future build may have widened.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateEventRequest {
    /// `YYYY-MM-DD`. Required by R11.
    pub event_date: String,
    /// Required by R11. Blank is refused — a card with no title is a card
    /// nobody can pick out of a list.
    pub title: String,
    /// The phase slug. An unknown one is a 422 naming the value.
    pub phase: String,
    /// One plain sentence. Optional by R11.
    #[serde(default)]
    pub fact: Option<String>,
    /// `day` | `month` | `year`. Absent means `day` — the default the column
    /// itself carries, so the two cannot disagree.
    #[serde(default)]
    pub date_precision: Option<String>,
    /// Absent means `false`. Separate from precision: precision says which
    /// parts of the date are known, this says the whole thing is an estimate.
    #[serde(default)]
    pub approximate: Option<bool>,
    /// Tag tokens, which land in `attributes.tags`. Absent and empty are the
    /// same thing on a CREATE (there is nothing yet to clear).
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Links to create with the event, so "add an event and its document" is
    /// ONE guarded write and one history row rather than two.
    #[serde(default)]
    pub links: Option<Vec<CreateLinkRequest>>,
}

/// Edit one event. `PUT /api/timeline/events/:id`. The same fields.
///
/// ## Rust Learning: why this is not `CreateEventRequest` with a different name
///
/// It very nearly is, and it stays separate for one reason that will outlive the
/// resemblance: a create may carry `links` and an edit may not. Links are added
/// and removed through their own endpoints once an event exists, because an edit
/// that silently replaced an event's link set would delete a colleague's link
/// while somebody re-typed a title. A shared struct would have to document
/// "`links` is ignored on PUT", and a documented-ignored field is a field
/// somebody will one day expect to work.
// serde: deny_unknown_fields — same reason as above. It is also what makes a
// browser that POSTs `links` to the PUT endpoint get told so.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateEventRequest {
    pub event_date: String,
    pub title: String,
    pub phase: String,
    #[serde(default)]
    pub fact: Option<String>,
    #[serde(default)]
    pub date_precision: Option<String>,
    #[serde(default)]
    pub approximate: Option<bool>,
    /// Absent leaves the stored tags alone; an empty list CLEARS them. The two
    /// are different instructions and stay different all the way down.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

/// Link one event to its evidence. `POST /api/timeline/events/:id/links`.
///
/// # Domain note — R9, and why `pinpoint` being absent is data
///
/// "Pinpoint optional but its absence is visibly marked, so unlinked and
/// unpinpointed events double as the to-scan to-do list." An absent pinpoint is
/// therefore never normalised to an empty string on the way in: `None` reaches
/// the column as NULL and every surface that renders the link marks it.
// serde: deny_unknown_fields — a request, not a stored row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateLinkRequest {
    /// `document` today; the design lists ten kinds and expects more.
    pub target_type: String,
    /// The id in that target's own store.
    pub target_id: String,
    /// The link text a human reads. Absent falls back to the target id at
    /// render time rather than being invented here.
    #[serde(default)]
    pub label: Option<String>,
    /// Page, paragraph, Q-number, line. Absent is the visible "no pinpoint".
    #[serde(default)]
    pub pinpoint: Option<String>,
}

/// Which link to remove. The natural key, as query parameters.
///
/// `DELETE /api/timeline/events/:id/links?target_type=…&target_id=…`
///
/// ## Why the key is in the query and not a body
///
/// A DELETE with a body is legal and widely mishandled — proxies drop it, and
/// `fetch` implementations differ. The key is three short values a human picked
/// off a screen, so it travels where a key belongs: in the address of the thing
/// being removed. `event_id` is the path segment; these two complete it.
// serde: deny_unknown_fields so a stale caller sending `targetType` is told,
// rather than having its delete match nothing and report success.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeleteLinkQuery {
    pub target_type: String,
    pub target_id: String,
}

/// Add one attributed note. `POST /api/timeline/events/:id/notes`.
// serde: deny_unknown_fields — a request, not a stored row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateNoteRequest {
    /// The note. Blank is refused: an empty note is a row with an author and
    /// nothing said.
    pub note: String,
}

/// What a document search asks for. `GET /api/timeline/documents?q=…`
// serde: deny_unknown_fields — a request, not a stored row.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentSearchQuery {
    /// What the author typed. Blank is refused rather than answered with the
    /// whole store: a picker that dumps every document is not a picker.
    pub q: String,
}

/// One document the picker offers.
// serde: allows unknown fields because this is a payload a future build may
// widen (a date, a type chip) without breaking an older reader.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChoiceDto {
    pub id: String,
    pub title: String,
}

/// The picker's answer: the short list, and how many there really were.
///
/// ## ⚑ THE CAP IS NEVER SILENT
///
/// `total` counts every document matching the search; `matches` holds at most
/// `chronology_document_picker_max` of them. A surface that showed the list
/// without the count would let somebody link the wrong document with no idea a
/// better match had been cut off — which is a silent failure in exactly the
/// sense Standing Rule 1 forbids, and the reason this is a struct rather than a
/// bare array.
// serde: allows unknown fields for the same forward-compatibility reason as the
// choice above.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentSearchResultDto {
    pub matches: Vec<DocumentChoiceDto>,
    /// How many matched in total, before the cap.
    pub total: i64,
    /// How many the cap allowed, so the surface can say "showing N of TOTAL"
    /// without knowing what the stored parameter is.
    pub shown_limit: usize,
}
