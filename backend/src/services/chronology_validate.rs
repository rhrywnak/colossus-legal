//! Every refusal the chronology's write endpoints make. Pure — no database.
//!
//! The handlers in `api::timeline_write` fetch the vocabularies (the phase rows,
//! the tag rows) and hand them here as slices; this module decides whether a
//! submitted event is writable and says exactly why it is not. Nothing here
//! touches a pool, so every rule below is reachable by a unit test without a
//! Postgres — which is the same split `services::chronology_read` makes for the
//! read side.
//!
//! ## ⚑ 400 AND 422 ARE DIFFERENT ANSWERS
//!
//! A 400 says the SHAPE is wrong: a blank title, a date that is not a date, a
//! precision token this build does not have. The form that sent it has a bug, or
//! the author left a required box empty, and the fix is in the request.
//!
//! A 422 says the shape is right and a VALUE names something this deployment
//! does not have: a phase slug with no row, a tag token that is not in the
//! vocabulary. The request is well-formed and the CHOICES it was offered are
//! stale — the fix is to reload the page, or to add the row.
//!
//! Collapsing them would send an author to look for a typo in a field they
//! filled in correctly. The instruction for Phase C names the phase case
//! explicitly: "an unknown phase is a 422 naming the value, never a 500".
//!
//! ## Why an unknown phase is refused HERE and not by the foreign key
//!
//! `chronology_events.phase` references `chronology_phases`, so an unknown slug
//! would be refused by the database too. It would come back as a foreign-key
//! violation, which this codebase turns into a 500 — an operator paged over
//! somebody's typo, with the offending value buried in a Postgres message. The
//! constraint stays as the backstop it was designed to be; this is the answer a
//! human reads.

use chrono::NaiveDate;

use crate::domain::chronology::{validate_precision, ChronologyPrecisionError};
use crate::domain::date_precision::DatePrecision;

/// The date format the wire uses, and the only one accepted.
///
/// ISO-8601's date half, which is what `<input type="date">` submits and what
/// `event_date` stores. Named rather than inlined because it appears in the
/// parse AND in the refusal message, and a refusal that named a different
/// format from the one the parser wants would send an author in a circle.
// STRUCTURAL: the ISO-8601 date half, which is the only format
// `<input type="date">` emits and the only one `event_date` stores. A wire
// contract, not a deployment setting — it cannot vary between DEV and PROD, and
// changing it would mean changing what every browser sends.
const WIRE_DATE_FORMAT: &str = "%Y-%m-%d";

/// The same format, in the words a human reads.
// STRUCTURAL: the same wire contract in the words a human reads. Bound to the
// parse format above by the refusal message that quotes both.
const WIRE_DATE_SHAPE: &str = "YYYY-MM-DD";

/// Why a submitted event could not be written.
///
/// ## Rust Learning: a typed error enum instead of a `String`
///
/// Every variant carries the VALUE that was refused, so the message the API
/// layer renders can quote it. A stringly-typed error would force the message to
/// be composed here — at which point the HTTP status would have to be guessed
/// from its text, and the 400/422 distinction above would be a matter of reading
/// English. The enum lets `api::timeline_write` map each variant to its status
/// once, by pattern match, with the compiler checking the table is complete.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ChronologyWriteRefusal {
    #[error("an event needs a title — R11 makes the date and the title the only two required fields, and this one is blank")]
    BlankTitle,

    #[error("'{supplied}' is not a date this build can read — expected {WIRE_DATE_SHAPE}")]
    UnreadableDate { supplied: String },

    #[error("{source}")]
    Precision {
        #[source]
        source: ChronologyPrecisionError,
    },

    #[error("no phase named '{supplied}' — this case's phases are: {known}")]
    UnknownPhase { supplied: String, known: String },

    #[error("no tag named '{supplied}' — this case's tags are: {known}. A new tag is a row in chronology_tags, not a code change")]
    UnknownTag { supplied: String, known: String },

    #[error("a note needs words — an empty note is a row with an author and nothing said")]
    BlankNote,

    #[error("a link needs a target type and a target id, and '{field}' is blank")]
    BlankLinkField { field: &'static str },
}

impl ChronologyWriteRefusal {
    /// Whether this refusal is about a VALUE the deployment does not have (422)
    /// rather than about the request's shape (400).
    ///
    /// The one place the distinction is decided, so the two handlers that map it
    /// cannot disagree. See the module header for what the difference means to
    /// the person reading the message.
    pub fn is_unprocessable(&self) -> bool {
        matches!(
            self,
            ChronologyWriteRefusal::UnknownPhase { .. } | ChronologyWriteRefusal::UnknownTag { .. }
        )
    }

    /// The field the refusal is about, for the error body's `details`.
    ///
    /// A form highlights the box it names. `None` for refusals that are about
    /// the request as a whole rather than one box.
    pub fn field(&self) -> Option<&'static str> {
        match self {
            ChronologyWriteRefusal::BlankTitle => Some("title"),
            ChronologyWriteRefusal::UnreadableDate { .. } => Some("event_date"),
            ChronologyWriteRefusal::Precision { .. } => Some("date_precision"),
            ChronologyWriteRefusal::UnknownPhase { .. } => Some("phase"),
            ChronologyWriteRefusal::UnknownTag { .. } => Some("tags"),
            ChronologyWriteRefusal::BlankNote => Some("note"),
            ChronologyWriteRefusal::BlankLinkField { field } => Some(field),
        }
    }

    /// The value that was refused, when there was one.
    pub fn value(&self) -> Option<&str> {
        match self {
            ChronologyWriteRefusal::UnreadableDate { supplied }
            | ChronologyWriteRefusal::UnknownPhase { supplied, .. }
            | ChronologyWriteRefusal::UnknownTag { supplied, .. } => Some(supplied),
            _ => None,
        }
    }
}

/// A submitted event, proved writable.
///
/// Nothing in here needs checking again. The handler turns it into the
/// repository's parameter struct and nothing else — which is what keeps the
/// judgement in one place a test can reach instead of spread through two
/// handlers that must agree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidEvent {
    pub event_date: NaiveDate,
    pub date_precision: DatePrecision,
    pub approximate: bool,
    pub phase: String,
    pub title: String,
    /// `None` when the author left it empty. R11: encouraged, optional.
    pub fact: Option<String>,
    /// Trimmed, de-duplicated, in the order the author picked them.
    pub tags: Vec<String>,
}

/// Everything one submitted event carries, before it is judged.
///
/// ## Rust Learning: borrowing the request rather than owning a copy
///
/// The two request DTOs (`CreateEventRequest`, `UpdateEventRequest`) are
/// different types with the same seven fields, so neither can be passed here
/// directly without this module knowing about `dto` — which would make a pure
/// judgement depend on a wire shape. This little struct of borrows is the seam:
/// each handler fills it from its own DTO, and the rule below is written once.
#[derive(Debug, Clone, Copy)]
pub struct SubmittedEvent<'a> {
    pub event_date: &'a str,
    pub title: &'a str,
    pub phase: &'a str,
    pub fact: Option<&'a str>,
    pub date_precision: Option<&'a str>,
    pub approximate: Option<bool>,
    pub tags: Option<&'a [String]>,
}

/// The vocabularies a submitted event is judged against.
///
/// Both are read from their tables by the handler. Passing them in — rather than
/// reading them here — is what keeps this module free of I/O, and it is what
/// lets a test state "these are the four phases" in one line.
#[derive(Debug, Clone, Copy)]
pub struct Vocabularies<'a> {
    /// Every `chronology_phases.id`, in stored order.
    pub phases: &'a [String],
    /// Every `chronology_tags.id`, in stored order.
    pub tags: &'a [String],
}

/// The precision an event takes when its author named none.
///
/// `day` — the same default `chronology_events.date_precision` carries, so the
/// column and this build cannot disagree about what an unspecified precision
/// means. Not a config value: it is the meaning of an absent field, which is a
/// fact about the wire contract rather than a tunable.
// STRUCTURAL: the meaning of an ABSENT precision field on the wire, mirroring
// `chronology_events.date_precision`'s own column default. Not a tunable —
// changing it would need this line AND a migration, and the two disagreeing is
// the drift it exists to prevent.
const DEFAULT_PRECISION: &str = "day";

/// Judge one submitted event, or say precisely what is wrong with it.
///
/// # Errors
/// The FIRST thing wrong, in the order a form would want to hear about it: the
/// two required fields, then the two vocabularies. One refusal at a time
/// because each one names a box, and a list of four would still send the author
/// back to the same form.
pub fn validate_event(
    submitted: SubmittedEvent<'_>,
    vocab: Vocabularies<'_>,
) -> Result<ValidEvent, ChronologyWriteRefusal> {
    let title = submitted.title.trim();
    if title.is_empty() {
        return Err(ChronologyWriteRefusal::BlankTitle);
    }

    let event_date = NaiveDate::parse_from_str(submitted.event_date.trim(), WIRE_DATE_FORMAT)
        .map_err(|_| ChronologyWriteRefusal::UnreadableDate {
            supplied: submitted.event_date.to_string(),
        })?;

    let precision_token = submitted
        .date_precision
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .unwrap_or(DEFAULT_PRECISION);
    let date_precision = validate_precision(precision_token)
        .map_err(|source| ChronologyWriteRefusal::Precision { source })?;

    let phase = submitted.phase.trim();
    if !vocab.phases.iter().any(|known| known == phase) {
        return Err(ChronologyWriteRefusal::UnknownPhase {
            supplied: submitted.phase.to_string(),
            known: vocab.phases.join(", "),
        });
    }

    let tags = validate_tags(submitted.tags.unwrap_or(&[]), vocab.tags)?;

    Ok(ValidEvent {
        event_date,
        date_precision,
        approximate: submitted.approximate.unwrap_or(false),
        phase: phase.to_string(),
        title: title.to_string(),
        // An empty fact is `None`, not `Some("")`. The column is nullable and
        // the surface renders nothing for either — but a stored empty string is
        // a value somebody wrote, and NULL is the absence of one. Keeping them
        // apart is what stops "he cleared the fact" and "he never wrote one"
        // from reading identically in a history snapshot.
        fact: submitted
            .fact
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        tags,
    })
}

/// Trim, drop blanks, de-duplicate, and refuse anything not in the vocabulary.
///
/// ## Why an unknown tag is refused rather than stored
///
/// `attributes.tags` is a free JSONB array that would happily hold a sixth tag,
/// and design §4 says the list is open — "plus whatever Roman adds". Open means
/// open BY ADDING A ROW: the filter chips, the form's picker and the card's
/// colours all read `chronology_tags`, so a token with no row would render as a
/// grey chip nobody can filter by and nobody can see the name of anywhere else.
/// Refusing it here is what makes "a new tag is a row, not a build" true rather
/// than aspirational.
///
/// Order is the author's, not the vocabulary's: the first tag decides the
/// event's dot colour, so re-ordering here would silently recolour the card.
fn validate_tags(
    submitted: &[String],
    known: &[String],
) -> Result<Vec<String>, ChronologyWriteRefusal> {
    let mut out: Vec<String> = Vec::new();
    for raw in submitted {
        let tag = raw.trim();
        if tag.is_empty() {
            continue;
        }
        if !known.iter().any(|k| k == tag) {
            return Err(ChronologyWriteRefusal::UnknownTag {
                supplied: raw.to_string(),
                known: known.join(", "),
            });
        }
        if !out.iter().any(|seen| seen == tag) {
            out.push(tag.to_string());
        }
    }
    Ok(out)
}

/// The `attributes` bag an event should be written with.
///
/// ## ⚑ EVERY OTHER KEY SURVIVES
///
/// This is the change rule (design R4) at the moment it matters most. An edit
/// submitted by today's form knows about `tags` and nothing else — but a row may
/// carry `people`, `spine`, `source: legacy_json`, or a key some future task
/// added. Rebuilding the bag from the request would delete all of them, quietly,
/// on the first edit of every seeded event.
///
/// So the stored bag is CLONED and only `tags` is replaced. `existing` is
/// whatever the row holds, including a non-object (which a hand-edited row could
/// be); anything that is not an object is replaced with one, because the column
/// is documented as an object and a bag that is an array has no keys to keep.
///
/// `tags` of `None` means the author's form did not mention tags at all, and the
/// stored ones are left exactly as they are.
pub fn merged_attributes(
    existing: &serde_json::Value,
    tags: Option<&[String]>,
) -> serde_json::Value {
    let mut bag = match existing {
        serde_json::Value::Object(map) => map.clone(),
        _ => serde_json::Map::new(),
    };
    if let Some(tags) = tags {
        bag.insert(
            "tags".to_string(),
            serde_json::Value::Array(
                tags.iter()
                    .map(|tag| serde_json::Value::String(tag.clone()))
                    .collect(),
            ),
        );
    }
    serde_json::Value::Object(bag)
}

/// Judge one submitted note.
///
/// # Errors
/// [`ChronologyWriteRefusal::BlankNote`] when there are no words in it.
pub fn validate_note(note: &str) -> Result<String, ChronologyWriteRefusal> {
    let trimmed = note.trim();
    if trimmed.is_empty() {
        return Err(ChronologyWriteRefusal::BlankNote);
    }
    Ok(trimmed.to_string())
}

/// One link's identity and label, trimmed and proved non-blank.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidLink {
    pub target_type: String,
    pub target_id: String,
    pub label: Option<String>,
    /// `None` is MEANINGFUL and stays `None` — the absence is what marks the
    /// link "no pinpoint" on every surface that draws it (design R9).
    pub pinpoint: Option<String>,
}

/// Judge one submitted link.
///
/// Existence of the TARGET is not decided here — that needs the store, and it is
/// the handler's job (§C1: "for target_type=document, creation VALIDATES the
/// target exists"). This proves only that the caller named one.
///
/// # Errors
/// [`ChronologyWriteRefusal::BlankLinkField`] naming whichever of the two
/// identifying fields is empty.
pub fn validate_link(
    target_type: &str,
    target_id: &str,
    label: Option<&str>,
    pinpoint: Option<&str>,
) -> Result<ValidLink, ChronologyWriteRefusal> {
    let target_type = target_type.trim();
    if target_type.is_empty() {
        return Err(ChronologyWriteRefusal::BlankLinkField {
            field: "target_type",
        });
    }
    let target_id = target_id.trim();
    if target_id.is_empty() {
        return Err(ChronologyWriteRefusal::BlankLinkField { field: "target_id" });
    }
    Ok(ValidLink {
        target_type: target_type.to_string(),
        target_id: target_id.to_string(),
        label: label
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string),
        pinpoint: pinpoint
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_string),
    })
}

#[cfg(test)]
#[path = "chronology_validate_tests.rs"]
mod tests;
