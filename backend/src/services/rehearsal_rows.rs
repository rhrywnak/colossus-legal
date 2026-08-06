//! One row of the rehearsal page — a statement, its source, its answer.
//!
//! Pure — no I/O. Everything here turns a stored graph row into the sentences a
//! witness reads, so every one of them is decided in a place a test can reach.
//!
//! ## Domain note: how a date is rendered, and why it is not the stored ISO
//!
//! `15 Dec 2009`, not `2009-12-15`. Ruled 2026-08-06, overturning this module's
//! first answer, and the reasoning is worth keeping because the first answer was
//! wrong in a specific and instructive way.
//!
//! The objection was that reformatting IMPLIES we parsed the value and can vouch
//! for it. That conflates two things. Formatting a value that genuinely parses as
//! a date is deterministic — if the stored value is wrong, both spellings are
//! equally wrong, and neither vouches for anything. What a month NAME does buy is
//! that it cannot be misread day-month by one reader and month-day by another,
//! which was the real half of the objection and is satisfied better by `15 Dec
//! 2009` than by an ISO string. And Marie reads this under stress: `2009-12-15`
//! is engineer-speak on a witness surface.
//!
//! The risk that survived is a MALFORMED stored string, and [`display_date`]
//! honors it: only a value that parses is formatted; anything else is rendered
//! verbatim. Never guessed, never blanked.
//!
//! Month abbreviations are date formatting, not user wording — no settings row
//! (ruled with the above).

use std::collections::HashMap;

use chrono::NaiveDate;

use crate::domain::card_language::statement_kind_label;
use crate::domain::wording_rehearsal::RehearsalWording;
use crate::domain::wording_templates::render;
use crate::dto::rehearsal::{RehearsalAnswer, RehearsalSource};
use crate::repositories::scenario_accusation_repository::RehearsalFactRow;

/// How much of a quote the collapsed row shows.
///
/// ## Rust Learning: a `const` that is GEOMETRY, not wording
///
/// Standing Rule 2 asks whether a value varies across environments or cases. This
/// one does not: it is how much text fits on one line at this page's type size,
/// which is a property of the layout and not of the case. The same reasoning that
/// keeps the context-window's SENTENCE rule in code while its character seed is a
/// setting — and the architect ruled it explicitly for this field (2026-08-06).
// STRUCTURAL: how many characters fit on ONE LINE at this page's type scale. A
// format constant, not a per-deployment size — every deployment renders the same
// page at the same scale, and a shorter or longer cut would not be a different
// configuration but a different layout. Ruled explicitly for this field
// (architect, 2026-08-06).
const FIRST_LINE_CHARS: usize = 96;

/// The opening of a quote, cut at a word boundary.
///
/// ## Why the cut is never mid-word
///
/// "…refused to divide the prop" reads as a transcription error on a page whose
/// entire claim is that its quotes are verbatim. Backing up to the last space
/// costs a few characters and never produces a word the record does not contain.
///
/// A quote already short enough is returned WHOLE, with no ellipsis — an ellipsis
/// on a complete sentence claims there is more, which is the same lie in
/// miniature.
pub(crate) fn first_line(quote: &str) -> String {
    let trimmed = quote.trim();
    if trimmed.chars().count() <= FIRST_LINE_CHARS {
        return trimmed.to_string();
    }

    // `char_indices` rather than byte slicing: a quote is arbitrary human text
    // and may hold multi-byte characters, where `&s[..n]` would panic on a
    // boundary. Taking the byte offset of the FIRST_LINE_CHARS-th character is
    // the safe equivalent.
    let cut = trimmed
        .char_indices()
        .nth(FIRST_LINE_CHARS)
        .map(|(byte, _)| byte)
        .unwrap_or(trimmed.len());
    let head = &trimmed[..cut];

    let head = match head.rfind(' ') {
        // Only back up when there IS a word boundary to back up to. A single
        // ninety-seven-character token has none, and cutting it mid-way is then
        // the only option — better than returning nothing.
        Some(space) if space > 0 => &head[..space],
        _ => head,
    };
    format!("{}…", head.trim_end())
}

/// Where a statement was made, and how to open it.
///
/// Composed from the stored template. A row whose document the record cannot name
/// still gets a source: the label falls back to the page alone rather than
/// vanishing, because a missing "Where" column reads as a rendering fault where a
/// partial one reads as an incomplete record — which is what it is.
pub(crate) fn source_of(fact: &RehearsalFactRow, wording: &RehearsalWording) -> RehearsalSource {
    let document = fact.document_title.as_deref().unwrap_or_default();
    let page = fact.page.map(|p| p.to_string()).unwrap_or_default();

    RehearsalSource {
        label: render(
            &wording.source_label_template,
            &[("document", document), ("page", &page)],
        ),
        href: viewer_href(fact),
        open_label: wording.source_open_label.clone(),
    }
}

/// The viewer address for a statement, already at its page.
///
/// Empty when the record cannot say WHICH document — the client renders no
/// control rather than a link to nowhere, which is the same rule the working
/// view's pinpoints follow.
///
/// The URL itself is built by `card_language::viewer_href`, the one place that
/// spells this route. Two functions producing the same address is one too many:
/// if the viewer's tab parameter ever changes, a missed site is a dead link with
/// no compiler error behind it.
fn viewer_href(fact: &RehearsalFactRow) -> String {
    match fact.document_id.as_deref() {
        Some(document_id) => crate::domain::card_language::viewer_href(document_id, fact.page),
        None => String::new(),
    }
}

/// Who said it — or the stored sentence saying the record does not say.
///
/// A blank would read as a rendering fault. "Speaker not recorded" says the record
/// is silent, which is a different and checkable thing. Measured on S-2: one
/// statement of forty-six.
pub(crate) fn who_of(fact: &RehearsalFactRow, wording: &RehearsalWording) -> String {
    fact.speaker
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| wording.instance_who_gap.clone())
}

/// How a stored date is spelled on screen.
///
/// ## Rust Learning: `%-d %b %Y`, and why the hyphen matters
///
/// chrono's `%d` zero-pads — "05 Dec 2009". The `-` flag suppresses the padding,
/// giving "5 Dec 2009", which is how a person says it out loud. `%b` is the
/// three-letter month; chrono renders it in English regardless of the machine's
/// locale, which is what a shared case file needs.
///
/// ## The fallback is verbatim, and it is the whole point of the function
///
/// A value that does not parse as `YYYY-MM-DD` is rendered exactly as stored. Not
/// blanked — that would hide a date the record does have; not guessed — a partial
/// or malformed string is a data-integrity question for task 2.5, and inventing a
/// day for it would put a date in a witness's mouth. Rendering it raw shows the
/// human precisely what the record holds, which is the only honest answer while
/// the extraction's date handling is still being rebuilt.
// STRUCTURAL: the display format for a date. Not a per-deployment preference —
// this is one case file read by three people, and two spellings of one date on
// one surface would be worse than either alone.
const DATE_FORMAT: &str = "%-d %b %Y";

pub(crate) fn display_date(stored: &str) -> String {
    let stored = stored.trim();
    match NaiveDate::parse_from_str(stored, "%Y-%m-%d") {
        Ok(date) => date.format(DATE_FORMAT).to_string(),
        Err(_) => stored.to_string(),
    }
}

/// The date, and its named gap.
///
/// Returns `(when, when_gap)`, of which exactly one is `Some`. Never both — that
/// would say two things about one date — and never neither, which would leave a
/// blank column a reader cannot interpret.
pub(crate) fn when_of(
    fact: &RehearsalFactRow,
    wording: &RehearsalWording,
) -> (Option<String>, Option<String>) {
    match fact
        .occurred_on
        .as_deref()
        .map(str::trim)
        .filter(|d| !d.is_empty())
    {
        Some(date) => (Some(display_date(date)), None),
        None => (None, Some(wording.instance_when_gap.clone())),
    }
}

/// The kind of statement, in plain words.
///
/// `statement_kind_label` humanizes rather than translating, and the architect
/// confirmed that rule for this surface (2026-08-06): the extraction's set is
/// OPEN — nineteen values measured on this case — so a fixed table of plain names
/// would silently miss the twentieth, and the miss would be invisible.
pub(crate) fn kind_of(fact: &RehearsalFactRow) -> String {
    fact.statement_type
        .as_deref()
        .map(statement_kind_label)
        .unwrap_or_default()
}

/// The paired answer, composed — or `None` when the item cannot be produced.
///
/// `None` for an answer the record no longer holds is deliberate: the caller then
/// renders the Remove law's named gap instead. Returning a half-built answer with
/// an empty quote would put an empty pair of quotation marks under "Our answer",
/// which reads as us having said nothing.
pub(crate) fn answer_of(
    answer_id: &str,
    facts: &HashMap<String, RehearsalFactRow>,
    wording: &RehearsalWording,
) -> Option<RehearsalAnswer> {
    let fact = facts.get(answer_id)?;
    let quote = fact.quote.as_deref()?.trim().to_string();
    if quote.is_empty() {
        return None;
    }

    let (when, when_gap) = when_of(fact, wording);
    Some(RehearsalAnswer {
        who: who_of(fact, wording),
        when,
        when_gap,
        source: source_of(fact, wording),
        quote,
    })
}

#[cfg(test)]
#[path = "rehearsal_rows_tests.rs"]
mod tests;
