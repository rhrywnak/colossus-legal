// =============================================================================
// backend/src/domain/chronology.rs — the chronology's closed vocabularies
// (CASE_CHRONOLOGY_DESIGN_v2, task TIMELINE PHASE A)
// =============================================================================
//
// Two vocabularies and one predicate, all code-owned, following the
// `actor_role` (D1) / `date_precision` (P4) / `case_phase` precedent: a Rust
// list plus a versioned constant, with the migration's CHECK as a backstop for
// anything that reaches the table another way.
//
// ## What is deliberately NOT here
//
// The PHASES. They live in `chronology_phases` (design R15) and in
// `domain::case_phase` for their slugs, and nothing here repeats either. A
// fourth copy of the phase vocabulary is the copy that goes stale.
//
// The LINK TARGET TYPES are not a closed enum either — see
// `chronology_event_links.target_type` in the migration for why: the design
// lists ten kinds and expects more, and closing the set would make every new
// kind a migration.

use crate::domain::date_precision::{DatePrecision, ALL_DATE_PRECISIONS};

/// The version of the chronology TAG vocabulary THIS build defines.
///
/// Bumped whenever a tag is added or removed. Mirrors `ACTOR_ROLE_LOOKUP_V`,
/// `DATE_PRECISION_LOOKUP_V` and `CASE_PHASE_LOOKUP_V`.
pub const CHRONOLOGY_TAG_LOOKUP_V: u32 = 1;

/// Every tag the seed corpus uses, as `(stored token, display label)`.
///
/// # Domain note
///
/// These are the five categories the hand-written timeline has carried since
/// 2026 — the vocabulary design R7 rules is KEPT ("the existing tag vocabulary
/// and colors stay"). The stored token is the JSON `category` key, not the
/// label: the colour and the display name are both looked up BY the key, so
/// storing the label would break the lookup and create a second copy of the
/// display name at the same time.
///
/// The list is open in practice — design §4 says "plus whatever Roman adds", and
/// `attributes.tags` is a free JSONB array that will happily hold a sixth tag.
/// What this constant defines is the vocabulary the SEED is allowed to use, and
/// therefore what the permanent validation guard checks the seed against.
pub const CHRONOLOGY_TAGS: &[(&str, &str)] = &[
    ("financial", "Financial"),
    ("court_action", "Court Action"),
    ("filing", "Filing"),
    ("discovery", "Discovery"),
    ("personal", "Personal"),
];

/// Whether `token` is one of the seed vocabulary's tags.
///
/// ## Rust Learning: `iter().any()` over a slice of tuples
///
/// `CHRONOLOGY_TAGS` is a `&[(&str, &str)]`, so each item destructures to a pair.
/// `any` short-circuits on the first match and returns a plain `bool` — no
/// allocation, and nothing to keep in sync with the list above.
pub fn is_known_tag(token: &str) -> bool {
    CHRONOLOGY_TAGS.iter().any(|(tag, _)| *tag == token)
}

/// The display label for a tag, or `None` if this build does not know it.
///
/// `None` rather than echoing the token back: a caller rendering a chip wants to
/// decide for itself whether an unknown tag is shown raw or hidden, and that
/// decision does not belong to the vocabulary.
pub fn tag_label(token: &str) -> Option<&'static str> {
    CHRONOLOGY_TAGS
        .iter()
        .find(|(tag, _)| *tag == token)
        .map(|(_, label)| *label)
}

/// The date precisions a chronology event may carry.
///
/// # Domain note — why this is a SUBSET, not a new enum
///
/// `DatePrecision` has four members because a DOCUMENT may carry no usable date
/// at all, and `Unknown` is the answer "a human looked and there is none". A
/// chronology EVENT cannot be in that state: design R11 makes the date required
/// forever, and `chronology_events.event_date` is `NOT NULL`. So the chronology
/// accepts exactly the precisions that expect a date — which is what
/// [`DatePrecision::expects_a_date`] already says.
///
/// Deriving the subset from that predicate rather than writing `[Day, Month,
/// Year]` by hand means a fifth precision added to the shared vocabulary lands
/// here automatically, on the right side of the line, with no second edit to
/// forget.
pub fn chronology_precisions() -> Vec<DatePrecision> {
    ALL_DATE_PRECISIONS
        .iter()
        .copied()
        .filter(DatePrecision::expects_a_date)
        .collect()
}

/// Why a submitted chronology precision could not be accepted.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ChronologyPrecisionError {
    #[error("'{supplied}' is not a date precision — expected one of: {valid}")]
    Unknown { supplied: String, valid: String },

    #[error(
        "precision '{supplied}' is not usable on a chronology event, because an \
         event always has a date. Expected one of: {valid}"
    )]
    NeedsADate { supplied: String, valid: String },
}

/// Validate a precision token for a chronology event.
///
/// Two refusals, not one, because they send the caller to different places: an
/// unrecognised token is a typo or a version skew, while `unknown` is a token
/// this build understands perfectly and refuses on purpose. Collapsing them into
/// one message would tell an operator to check their spelling of a word they
/// spelled correctly.
pub fn validate_precision(supplied: &str) -> Result<DatePrecision, ChronologyPrecisionError> {
    let valid = chronology_precisions()
        .iter()
        .map(|p| p.as_str())
        .collect::<Vec<_>>()
        .join(", ");

    match DatePrecision::from_token(supplied) {
        None => Err(ChronologyPrecisionError::Unknown {
            supplied: supplied.to_string(),
            valid,
        }),
        Some(p) if !p.expects_a_date() => Err(ChronologyPrecisionError::NeedsADate {
            supplied: supplied.to_string(),
            valid,
        }),
        Some(p) => Ok(p),
    }
}

#[cfg(test)]
#[path = "chronology_tests.rs"]
mod tests;
