//! Every refusal a subset write makes. Pure — no database.
//!
//! The handlers in `api::timeline_subsets` fetch what a judgement needs (the
//! live names, which event ids exist) and hand it here as values; this module
//! decides whether a submitted subset is writable and says exactly why it is
//! not. Nothing here touches a pool, so every rule below is reachable by a unit
//! test without a Postgres — the same split `chronology_validate` makes for
//! events.
//!
//! ## ⚑ 400, 409 AND 422 ARE THREE DIFFERENT ANSWERS
//!
//! A 400 says the SHAPE is wrong: a blank name, two events claiming the same
//! position. The form that sent it has a bug, or the author left a box empty,
//! and the fix is in the request.
//!
//! A 409 says the request is fine and the STORE already holds something that
//! conflicts: a live subset with this name, a scenario that already carries this
//! subset. The fix is to pick a different name, or to reload and see what is
//! already there.
//!
//! A 422 says the shape is right and a VALUE names something this deployment
//! does not have: an event id that is not in this case. The request is
//! well-formed and the CHOICES it was offered are stale.
//!
//! Collapsing any two of them sends an author to look in the wrong place.

use std::collections::HashSet;

use uuid::Uuid;

/// Why a submitted subset could not be written.
///
/// ## Rust Learning: a typed error enum instead of a `String`
///
/// Every variant carries the VALUE that was refused, so the message the API
/// layer renders can quote it. A stringly-typed error would force the message to
/// be composed here — at which point the HTTP status would have to be guessed
/// from its text, and the three-way distinction above would be a matter of
/// reading English. The enum lets the API layer map each variant to its status
/// once, by pattern match, with the compiler checking the table is complete.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SubsetWriteRefusal {
    #[error(
        "a subset needs a name — it is how a story is picked out of a list, and this one is blank"
    )]
    BlankName,

    #[error("this case already has a live subset named '{supplied}'. Two stories with one name is two things nobody can tell apart — rename one, or edit the one that is already there")]
    NameTaken { supplied: String },

    #[error("no chronology event {supplied} in this case — the picker only offers events that exist, so this id came from a stale page")]
    UnknownEvent { supplied: String },

    #[error("two events were given the same story position ({supplied}) — a story order with a tie has no order")]
    DuplicatePosition { supplied: String },

    #[error(
        "the same event was listed twice ({supplied}) — an event is in a story once or not at all"
    )]
    DuplicateEvent { supplied: String },
}

impl SubsetWriteRefusal {
    /// Whether this refusal is about a VALUE the deployment does not have (422)
    /// rather than about the request's shape (400) or the store's state (409).
    pub fn is_unprocessable(&self) -> bool {
        matches!(self, SubsetWriteRefusal::UnknownEvent { .. })
    }

    /// Whether this refusal is about something the STORE already holds (409).
    pub fn is_conflict(&self) -> bool {
        matches!(self, SubsetWriteRefusal::NameTaken { .. })
    }

    /// The field the refusal is about, for the error body's `details`.
    ///
    /// A form highlights the box it names.
    pub fn field(&self) -> Option<&'static str> {
        match self {
            SubsetWriteRefusal::BlankName | SubsetWriteRefusal::NameTaken { .. } => Some("name"),
            SubsetWriteRefusal::UnknownEvent { .. } | SubsetWriteRefusal::DuplicateEvent { .. } => {
                Some("event_id")
            }
            SubsetWriteRefusal::DuplicatePosition { .. } => Some("position"),
        }
    }

    /// The value that was refused.
    pub fn value(&self) -> Option<&str> {
        match self {
            SubsetWriteRefusal::NameTaken { supplied }
            | SubsetWriteRefusal::UnknownEvent { supplied }
            | SubsetWriteRefusal::DuplicatePosition { supplied }
            | SubsetWriteRefusal::DuplicateEvent { supplied } => Some(supplied),
            SubsetWriteRefusal::BlankName => None,
        }
    }
}

/// One event's place in a story, proved writable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidSubsetEvent {
    pub event_id: Uuid,
    pub position: i32,
    /// Trimmed. `""` when the author wrote none — the column is `NOT NULL
    /// DEFAULT ''`, so there is one empty value rather than two.
    pub note: String,
}

/// What a submitted event reference carries, before it is judged.
///
/// A little struct of borrowed values rather than the DTO itself, so this module
/// stays free of `dto` — a pure judgement that depended on a wire shape would
/// have to change every time the wire did.
#[derive(Debug, Clone, Copy)]
pub struct SubmittedSubsetEvent<'a> {
    pub event_id: Uuid,
    pub position: i32,
    pub note: Option<&'a str>,
}

/// Judge a subset's NAME.
///
/// # Errors
/// [`SubsetWriteRefusal::BlankName`] for a name that is empty once trimmed;
/// [`SubsetWriteRefusal::NameTaken`] when `clash` says a live subset in this
/// case already has it.
pub fn validate_name(supplied: &str, clash: bool) -> Result<String, SubsetWriteRefusal> {
    let name = supplied.trim();
    if name.is_empty() {
        return Err(SubsetWriteRefusal::BlankName);
    }
    if clash {
        return Err(SubsetWriteRefusal::NameTaken {
            supplied: name.to_string(),
        });
    }
    Ok(name.to_string())
}

/// Judge a submitted ordered set of event references.
///
/// Three rules, in the order a reader would apply them: no event twice, no
/// position twice, and every id known. The FIRST failure is reported and the
/// rest is not checked — a list of five complaints about one paste is worse to
/// read than the one that has to be fixed first.
///
/// `known` is which of the submitted ids actually exist in this case, read by
/// the caller in one query. Passing it in — rather than reading it here — is
/// what keeps this function pure and lets a test state the world in one line.
///
/// # Errors
/// [`SubsetWriteRefusal::DuplicateEvent`], [`SubsetWriteRefusal::DuplicatePosition`]
/// or [`SubsetWriteRefusal::UnknownEvent`], whichever is met first.
pub fn validate_events(
    submitted: &[SubmittedSubsetEvent<'_>],
    known: &HashSet<Uuid>,
) -> Result<Vec<ValidSubsetEvent>, SubsetWriteRefusal> {
    let mut seen_events: HashSet<Uuid> = HashSet::new();
    let mut seen_positions: HashSet<i32> = HashSet::new();
    let mut out = Vec::with_capacity(submitted.len());

    for item in submitted {
        if !seen_events.insert(item.event_id) {
            return Err(SubsetWriteRefusal::DuplicateEvent {
                supplied: item.event_id.to_string(),
            });
        }
        if !seen_positions.insert(item.position) {
            return Err(SubsetWriteRefusal::DuplicatePosition {
                supplied: item.position.to_string(),
            });
        }
        if !known.contains(&item.event_id) {
            return Err(SubsetWriteRefusal::UnknownEvent {
                supplied: item.event_id.to_string(),
            });
        }
        out.push(ValidSubsetEvent {
            event_id: item.event_id,
            position: item.position,
            // Trimmed on the way in, deliberately: a note of three spaces and a
            // note of nothing are the same thing to a reader, and storing the
            // first would make an empty line render with height.
            note: item.note.unwrap_or_default().trim().to_string(),
        });
    }
    Ok(out)
}

#[cfg(test)]
#[path = "chronology_subset_validate_tests.rs"]
mod tests;
