//! The conditional timeline — both sides in date order (task 2.11 B2, block 4).
//!
//! Pure — no I/O.
//!
//! ## The threshold is measured over what is PLACED, never over the pool
//!
//! Ruled 2026-08-06, and it is the honest-gap law rather than a new rule. The
//! timeline interleaves the marked instances and their paired answers; a
//! threshold read against the scenario's whole INCLUDED pool would let this block
//! promise a timeline the placed set has no rows for. Measured on S-2 the day
//! this was written: four distinct dates in the pool, zero placed items. Under a
//! pool-based threshold the block would have rendered an empty "timeline".
//!
//! ## Why it is conditional at all
//!
//! Measured on this case: twelve of forty-six resolvable included statements carry
//! a date, across four distinct dates, none later than 2010. Roman's model asks
//! for "2011 to 2019" and the data cannot draw it. A timeline drawn from one date
//! is a dot; drawn from two undated items it is a lie about chronology. So the
//! block renders only when the placed set can actually draw one, and says how many
//! items it is NOT showing when it cannot.
//!
//! ## Domain note: sorted on the STORED value, displayed as a month name
//!
//! The entries read `15 Dec 2009` (ruled 2026-08-06), but they are ordered and
//! counted on the raw `2009-12-15`. That separation is load-bearing: `"1 Dec
//! 2009"` sorts before `"15 Nov 2009"` as a string, so formatting before sorting
//! would put the chronology out of order — on the one block whose entire purpose
//! is the chronology. See [`drawable_entries`].
//!
//! ## Domain note: no document-date inheritance, ever
//!
//! A statement's date is its OWN. Measured on this case, the Court of Appeals
//! document is titled "…RULLING 01 12 2012" while the statement inside it carries
//! 2010-10-04, and "CFS INTERROGATORY RESPONSE 08 08 16" holds a statement dated
//! 2009-10-02. Inheriting a document's date would put three wrong years on a page
//! a witness testifies from.

use std::collections::{HashMap, HashSet};

use crate::domain::settings::Settings;
use crate::domain::wording_templates::render;
use crate::dto::rehearsal::{RehearsalTimeline, RehearsalTimelineEntry};
use crate::repositories::scenario_accusation_repository::RehearsalFactRow;
use crate::services::rehearsal_rows::{display_date, source_of, who_of};
use crate::services::scenario_accusation::AccusationState;

/// Which side of the story an entry belongs to.
///
/// Tokens rather than prose so the client can style the two sides without reading
/// a sentence — the same reason the gap kinds are tokens. "Their words" and "our
/// answer" is the whole point of interleaving them; a client that could not tell
/// them apart would draw one undifferentiated list.
pub const SIDE_THEIR_WORDS: &str = "their_words";
pub const SIDE_OUR_ANSWER: &str = "our_answer";

/// Build block 4.
///
/// Returns entries ONLY when the placed items carry at least the configured
/// number of distinct dates. Otherwise the entries are empty and `notice` carries
/// the honest-gap line naming how many placed items have no date.
pub(crate) fn build_timeline(
    state: &AccusationState,
    facts: &HashMap<String, RehearsalFactRow>,
    settings: &Settings,
) -> RehearsalTimeline {
    let w = &settings.rehearsal_wording;
    let placed = placed_rows(state, facts);

    let chrome = &settings.rehearsal_chrome_wording;
    let dated_entries = drawable_entries(&placed, w, chrome);

    // Counted on the STORED value, not the rendered one. Two spellings of one
    // date would count as two distinct dates and could push a one-date scenario
    // over the threshold — drawing a "timeline" from a single point.
    let distinct: HashSet<&str> = dated_entries.iter().map(|(raw, _)| raw.as_str()).collect();
    let entries: Vec<RehearsalTimelineEntry> = dated_entries
        .iter()
        .map(|(_, entry)| entry.clone())
        .collect();
    let people = people_in(&entries);

    if distinct.len() < settings.rehearsal_timeline_min_distinct_dates {
        let total = placed.len();
        // Counted from the PLACED set, not from the entries: an item dropped for
        // having no quote is still an item this block is not showing, and the
        // sentence promises to say how many those are.
        let undated = total - dated_entries.len();
        return RehearsalTimeline {
            entries: Vec::new(),
            notice: Some(render(
                &w.timeline_gap_template,
                &[
                    ("undated", undated.to_string().as_str()),
                    ("total", total.to_string().as_str()),
                ],
            )),
            people: Vec::new(),
            filter_prompt: w.timeline_filter_prompt.clone(),
            filter_all_label: w.timeline_filter_all_label.clone(),
        };
    }

    RehearsalTimeline {
        entries,
        notice: None,
        people,
        filter_prompt: w.timeline_filter_prompt.clone(),
        filter_all_label: w.timeline_filter_all_label.clone(),
    }
}

/// The placed statements that can actually be drawn, in date order.
///
/// A statement with no date cannot go on a timeline, and one with no words has
/// nothing to show — both are dropped here and counted by the caller's notice.
///
/// Strictly chronological is the design's ruling and the reason the block exists:
/// the force is the repetition OVER TIME, which grouping by speaker buries. The
/// secondary sort on the quote keeps two entries sharing a date in a stable order,
/// so two renders of the same data cannot disagree.
fn drawable_entries(
    placed: &[(&'static str, &RehearsalFactRow)],
    w: &crate::domain::wording_rehearsal::RehearsalWording,
    chrome: &crate::domain::wording_rehearsal_chrome::RehearsalChromeWording,
) -> Vec<(String, RehearsalTimelineEntry)> {
    let mut entries: Vec<(String, RehearsalTimelineEntry)> = placed
        .iter()
        .filter_map(|(side, fact)| {
            let when = dated(fact)?;
            let quote = fact.quote.as_deref()?.trim();
            (!quote.is_empty()).then(|| {
                (
                    // The STORED value, kept beside the entry as the sort key and
                    // never sent. `"1 Dec 2009"` sorts before `"15 Nov 2009"` as a
                    // string; the ISO form is the only one that sorts as time.
                    when.to_string(),
                    RehearsalTimelineEntry {
                        when: display_date(when),
                        who: who_of(fact, w),
                        side: (*side).to_string(),
                        // The token decides the colour, this is what a human
                        // reads. Both travel, so a client never has to match on
                        // prose to tell the sides apart.
                        side_label: side_label(side, chrome).to_string(),
                        quote: quote.to_string(),
                        // Ruling C7: a timeline row IS an instance or an answer,
                        // so the pinpoint a witness needs to produce the document
                        // travels with it here too.
                        source: source_of(fact, w),
                    },
                )
            })
        })
        .collect();

    entries
        .sort_by(|(a_when, a), (b_when, b)| a_when.cmp(b_when).then_with(|| a.quote.cmp(&b.quote)));
    entries
}

/// Every PLACED statement, with the side it belongs to.
///
/// Placed means a human put it here: a marked instance, or the item they paired as
/// our answer to one. Nothing else is eligible — this block never reaches into the
/// included pool.
fn placed_rows<'a>(
    state: &AccusationState,
    facts: &'a HashMap<String, RehearsalFactRow>,
) -> Vec<(&'static str, &'a RehearsalFactRow)> {
    let mut out = Vec::new();
    for instance in &state.instances {
        if let Some(fact) = facts.get(&instance.anchor_graph_node_id) {
            out.push((SIDE_THEIR_WORDS, fact));
        }
        if let Some(fact) = instance
            .answers_graph_node_id
            .as_deref()
            .and_then(|id| facts.get(id))
        {
            out.push((SIDE_OUR_ANSWER, fact));
        }
    }
    out
}

/// The stored word for one side token.
///
/// ## Rust Learning: matching on a `&'static str` rather than an enum
///
/// The two side tokens are `const`s in this module, not an enum — they cross the
/// wire as strings and the DTO holds them as `String`. A `match` with a catch-all
/// would silently label an unknown token; there is no unknown token here (both
/// come from `placed_rows`, three lines away), so the fallback is the THEIRS
/// label and the branch is exhaustive in practice. Written as an `if` for exactly
/// that reason: a `match` with an unreachable arm invites a reader to wonder what
/// the third case is.
fn side_label<'a>(
    side: &str,
    chrome: &'a crate::domain::wording_rehearsal_chrome::RehearsalChromeWording,
) -> &'a str {
    if side == SIDE_OUR_ANSWER {
        &chrome.timeline_side_ours_label
    } else {
        &chrome.timeline_side_theirs_label
    }
}

/// A statement's own date, when it has one.
fn dated(fact: &RehearsalFactRow) -> Option<&str> {
    fact.occurred_on
        .as_deref()
        .map(str::trim)
        .filter(|d| !d.is_empty())
}

/// Everyone who appears, in first-appearance order.
///
/// ## Why the order is chronological rather than alphabetical
///
/// The filter's job is "show me all of George's", and a reader reaches for it
/// after meeting George in the list. Ordering by first appearance means the name
/// they just read is near the top of the control; alphabetical would put a name
/// they have not met there instead. De-duplicated by hand rather than through a
/// `HashSet` because a set has no order at all, and this one is load-bearing.
fn people_in(entries: &[RehearsalTimelineEntry]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for entry in entries {
        if !out.iter().any(|name| name == &entry.who) {
            out.push(entry.who.clone());
        }
    }
    out
}

#[cfg(test)]
#[path = "rehearsal_timeline_tests.rs"]
mod tests;
