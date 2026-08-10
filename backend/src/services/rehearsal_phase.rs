//! Which phase of the case a statement belongs to (task R3).
//!
//! Four phases — Pre-probate, Probate, COA, Complaint — shown as a chip on every
//! instance card and used as the prep page's filter.
//!
//! ## The rule: FORUM WINS, then the date
//!
//! A Court of Appeals ruling belongs to COA even though its date (2012-01-12)
//! falls squarely inside the probate years. Containment is the wrong question;
//! which court was speaking is the right one. So a document that names its forum
//! settles the phase outright, and only a document that does not falls through to
//! the boundary dates.
//!
//! ## Why the forum map is a settings row and not a graph property
//!
//! Measured on DEV 2026-08-10: `Document` nodes carry six properties —
//! `doc_type`, `id`, `ingested_at`, `source_document_id`, `status`, `title` — and
//! none of them is a forum. `doc_type` does not stand in for one either:
//! `court_ruling` covers BOTH the Judge Tighe probate opinion (April 2012) and
//! the Court of Appeals ruling (January 2012), which are exactly the two
//! documents this rule exists to tell apart.
//!
//! Date-only assignment was considered and rejected by the architect, for a
//! reason worth keeping written down: it would have tagged the appeal "Probate"
//! on the page Marie preps from. A known-wrong chip is worse than a missing one.
//!
//! So the case's documents name their own forum in `rehearsal_phase_document_forums`,
//! which Roman edits in Settings with no build. A document absent from the map
//! falls through to the date rule rather than being guessed at.
//!
//! ## Rust Learning: `&str` in, `String` out, and why not an enum
//!
//! A phase is a stored LABEL, not a compiled vocabulary — Roman can rename
//! "COA" to "Court of Appeals" in Settings tonight, and an enum would make that a
//! rebuild. The label travels as a string and the client matches chips against it
//! by equality, so the two ends agree without either compiling the words in.

use crate::domain::settings::Settings;

/// The separator inside the phase list and the boundary list.
// CONST: a delimiter inside a stored value, not a deployment knob — changing it
// means re-writing every row that uses it, so it is structural (Rule 13 N/A).
const LIST_SEPARATOR: char = '|';
/// The separator between `document-id=Phase` pairs in the forum map.
const PAIR_SEPARATOR: char = ',';
/// What joins a document id to its forum inside one pair.
const PAIR_ASSIGN: char = '=';

/// The four phase labels, in chronological order.
///
/// Returns them borrowed from the settings snapshot — no allocation, and no
/// second copy that could disagree with the row.
pub(crate) fn phase_labels(settings: &Settings) -> Vec<&str> {
    settings
        .rehearsal_wording
        .phase_labels
        .split(LIST_SEPARATOR)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

/// Which phase one statement belongs to.
///
/// `document_id` decides when the forum map names it; otherwise `occurred_on`
/// decides against the two boundaries; and a statement with neither gets the
/// stored undated label.
///
/// ## Why an undated statement gets a LABEL rather than `None`
///
/// 57% of this case's evidence carries no date (measured 2026-08-10), so this is
/// a common state rather than an edge. A card with no chip, sitting in a list
/// where every other card has one, reads as a rendering fault; a card that says
/// "No date yet" reads as the standing prompt it is — go and add one on the
/// working page.
pub(crate) fn phase_of(
    document_id: Option<&str>,
    occurred_on: Option<&str>,
    settings: &Settings,
) -> String {
    // FORUM FIRST. This is the whole rule, and the order is the rule.
    if let Some(id) = document_id {
        if let Some(forum) = forum_for(id, settings) {
            return forum.to_string();
        }
    }

    let w = &settings.rehearsal_wording;
    let Some(date) = occurred_on.map(str::trim).filter(|d| !d.is_empty()) else {
        return w.phase_undated_label.clone();
    };

    let labels = phase_labels(settings);
    let bounds = boundaries(settings);
    // A malformed list is not worth a panic on a witness's page: fall back to the
    // undated label, which is honest ("we cannot place this") rather than wrong.
    let (Some(first), Some(second)) = (bounds.first(), bounds.get(1)) else {
        return w.phase_undated_label.clone();
    };
    let (Some(pre), Some(mid), Some(last)) = (labels.first(), labels.get(1), labels.get(3)) else {
        return w.phase_undated_label.clone();
    };

    // Compared as string prefixes, which is what makes a year-only date work:
    // "2011" < "2014-01" is true and correct, and "2011" >= "2009-06" likewise.
    // Parsing to a NaiveDate would need a day this record does not have and would
    // put one in a witness's mouth.
    if date < *first {
        (*pre).to_string()
    } else if date >= *second {
        (*last).to_string()
    } else {
        (*mid).to_string()
    }
}

/// The forum a document belongs to, if the map names one.
fn forum_for<'a>(document_id: &str, settings: &'a Settings) -> Option<&'a str> {
    settings
        .rehearsal_wording
        .phase_document_forums
        .split(PAIR_SEPARATOR)
        .filter_map(|pair| pair.split_once(PAIR_ASSIGN))
        .find(|(id, _)| id.trim() == document_id)
        .map(|(_, forum)| forum.trim())
        .filter(|forum| !forum.is_empty())
}

/// The two dates that split the timeline for documents with no named forum.
fn boundaries(settings: &Settings) -> Vec<&str> {
    settings
        .rehearsal_wording
        .phase_boundaries
        .split(LIST_SEPARATOR)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
#[path = "rehearsal_phase_tests.rs"]
mod tests;
