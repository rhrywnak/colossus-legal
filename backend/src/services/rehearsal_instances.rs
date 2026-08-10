//! Turning marked instances into the rows the prep page renders (task R3).
//!
//! Split out of `rehearsal_render` when that module crossed the 300-line limit.
//! The seam is the one the page itself has: everything here builds ONE ROW from
//! one placed statement — its chronology, its answer, its gap — and everything
//! left there composes the blocks those rows sit in.
//!
//! ## The chronology lives here
//!
//! This module owns the sort that makes the instance list the page's timeline,
//! which is why the separate TIMELINE section could be removed. See
//! [`walk_instances`] for why it sorts on the stored string rather than the
//! displayed date.

use std::collections::{HashMap, HashSet};

use crate::domain::settings::Settings;
use crate::domain::wording_rehearsal::RehearsalWording;
use crate::domain::wording_rehearsal_chrome::RehearsalChromeWording;
use crate::domain::wording_templates::render;
use crate::dto::rehearsal::{RehearsalGap, RehearsalInstance};
use crate::repositories::scenario_accusation_repository::RehearsalFactRow;
use crate::services::rehearsal_render::{
    ScenarioInput, GAP_ANSWER_REMOVED, GAP_INSTANCE_UNAVAILABLE, GAP_NO_ANSWER,
};
use crate::services::rehearsal_rows::{answer_of, first_line, kind_of, source_of, when_of, who_of};

/// `(Vec, Vec, usize)` at a call site is three positions nobody can read.
pub(crate) struct WalkedInstances {
    pub(crate) instances: Vec<RehearsalInstance>,
    pub(crate) gaps: Vec<RehearsalGap>,
    /// How many DISTINCT documents the rendered instances sit in.
    pub(crate) documents: usize,
}

/// Which of two statements comes first on the page.
///
/// Oldest first; undated LAST; two undated keep the order a human placed them in.
///
/// ## Why the STORED string is the key and not the displayed date
///
/// This record's dates are variable precision — measured on DEV, they run from
/// `"2005"` through `"2009-12"` to a full day — and ISO-shaped prefixes compare
/// as time. The DISPLAYED date does not: `"1 Dec 2009"` sorts before
/// `"15 Nov 2009"` as a string, which is the defect `rehearsal_timeline`
/// documents and the reason formatting must happen after ordering.
///
/// ## Why undated sorts last rather than first
///
/// 57% of this case's evidence carries no date. A block of undated statements at
/// the TOP would bury the chronology this section exists to show; at the bottom
/// they read as what they are — the ones still needing a date, which the working
/// page is where to add.
pub(crate) fn chronologically(a: Option<&str>, b: Option<&str>) -> std::cmp::Ordering {
    match (a, b) {
        (Some(x), Some(y)) => x.cmp(y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

/// instance the record cannot produce is named but never counted.
pub(crate) fn walk_instances(input: &ScenarioInput<'_>, w: &RehearsalWording) -> WalkedInstances {
    let mut instances = Vec::new();
    let mut gaps = Vec::new();
    // The documents the RENDERED instances sit in — gathered as the rows are
    // built, because the DTO deliberately carries no node id for a later pass to
    // look one up with. A statement whose document the record cannot name adds
    // nothing rather than inventing a source, so the count can only ever
    // undercount, which is the one direction the honest-gap law permits.
    let mut documents: HashSet<&str> = HashSet::new();

    // CHRONOLOGY (task R3): oldest first, undated LAST.
    //
    // This section IS the timeline now — the separate TIMELINE block is gone — so
    // the order a witness reads these in has to be the order they happened. The
    // marked list arrives in the order a human clicked, which is not that.
    //
    // ## Domain note: why sorting on the STORED string is correct here
    //
    // Dates in this record are variable precision — measured on DEV, 228 of 525
    // evidence nodes carry one and they range from `"2005"` to `"2015-10"` to a
    // full day. ISO-shaped prefixes sort as time (`"2009-12" < "2011-01"`), which
    // is the same property `rehearsal_timeline` already relied on. Formatting
    // first and sorting after would order them alphabetically — `"1 Dec 2009"`
    // before `"15 Nov 2009"` — which is the defect that module documents.
    //
    // Undated instances sort last rather than first: 57% of this case's evidence
    // has no date, and a block of undated statements at the TOP would bury the
    // chronology the section exists to show. At the bottom they read as what they
    // are — the ones still needing a date, which the working page is where to add.
    let mut marked_in_order: Vec<_> = input.state.instances.iter().collect();
    marked_in_order.sort_by(|a, b| {
        let key = |m: &&crate::services::scenario_accusation::MarkedInstance| {
            usable_fact(&m.anchor_graph_node_id, input.facts)
                .and_then(|f| f.occurred_on.clone())
                .filter(|d| !d.trim().is_empty())
        };
        chronologically(key(a).as_deref(), key(b).as_deref())
    });

    for marked in marked_in_order {
        let Some(fact) = usable_fact(&marked.anchor_graph_node_id, input.facts) else {
            // A marked instance the record store can no longer produce. It is NOT
            // rendered — the page shows only what it can show — and it is NOT
            // counted, but it is always named. Six of these were measured on S-2.
            gaps.push(RehearsalGap {
                kind: GAP_INSTANCE_UNAVAILABLE.to_string(),
                message: w.gap_instance_unavailable.clone(),
                // Not rendered, so there is nothing to jump to. A position here
                // would scroll a reader to somebody else's statement.
                position: None,
            });
            continue;
        };

        let (row, gap) = answered_row(
            instances.len() + 1,
            marked,
            fact,
            input.facts,
            w,
            &input.settings.rehearsal_chrome_wording,
            input.settings,
        );
        if let Some(gap) = gap {
            gaps.push(gap);
        }
        if let Some(document_id) = fact.document_id.as_deref() {
            documents.insert(document_id);
        }
        instances.push(row);
    }

    WalkedInstances {
        instances,
        gaps,
        documents: documents.len(),
    }
}

/// One instance row, and the gap it leaves if nobody has answered it.
///
/// ## Why the SENTENCE lands in one place and a SHORT form in the other
///
/// Until beta.381 the composed gap sentence went onto the row AND into the gap
/// list — the same "NO ANSWER PREPARED — who, when, where" twice on one screen,
/// filed as a defect and killed by ruling C5. The sentence now lands only in the
/// list, which is what the header counts and what stays visible when the section
/// is folded. The row carries three words that cannot grow back into it.
fn answered_row(
    position: usize,
    marked: &crate::services::scenario_accusation::MarkedInstance,
    fact: &RehearsalFactRow,
    facts: &HashMap<String, RehearsalFactRow>,
    w: &RehearsalWording,
    chrome: &RehearsalChromeWording,
    settings: &Settings,
) -> (RehearsalInstance, Option<RehearsalGap>) {
    let answer = marked
        .answers_graph_node_id
        .as_deref()
        .and_then(|id| answer_of(id, facts, w));

    let gap = answer.is_none().then(|| {
        let mut gap = unanswered_gap(fact, marked.answers_graph_node_id.is_some(), w);
        // The row this entry is about, so the prep list can carry a link to it.
        gap.position = Some(position);
        gap
    });

    let row = instance_row(position, fact, answer, w, chrome, settings);
    (row, gap)
}

/// A placed statement the page can actually render, or `None`.
///
/// "Present in the map" is not enough: a node carrying no quote would render as an
/// empty pair of quotation marks attributed to a named person, which is worse than
/// saying the statement could not be loaded.
fn usable_fact<'a>(
    graph_node_id: &str,
    facts: &'a HashMap<String, RehearsalFactRow>,
) -> Option<&'a RehearsalFactRow> {
    let fact = facts.get(graph_node_id)?;
    let has_words = fact.quote.as_deref().is_some_and(|q| !q.trim().is_empty());
    has_words.then_some(fact)
}

/// The gap for an instance with no usable answer.
///
/// Two different absences, two different sentences: nobody paired anything (the
/// prep list), or somebody did and the item has since left (the Remove law). A
/// shared message would send a human to the wrong remedy.
fn unanswered_gap(fact: &RehearsalFactRow, was_paired: bool, w: &RehearsalWording) -> RehearsalGap {
    let (when, _) = when_of(fact, w);
    let when = when.unwrap_or_else(|| w.instance_when_gap.clone());
    let who = who_of(fact, w);

    if was_paired {
        return RehearsalGap {
            kind: GAP_ANSWER_REMOVED.to_string(),
            message: render(
                &w.gap_answer_removed,
                &[("who", who.as_str()), ("when", when.as_str())],
            ),
            // Filled by the caller, which is the only place that knows the row
            // number. Never left as `None` for these two kinds — see the caller.
            position: None,
        };
    }

    RehearsalGap {
        kind: GAP_NO_ANSWER.to_string(),
        message: render(
            &w.gap_no_answer,
            &[
                ("who", who.as_str()),
                ("when", when.as_str()),
                ("where", source_of(fact, w).label.as_str()),
            ],
        ),
        position: None,
    }
}

/// One instance row.
fn instance_row(
    position: usize,
    fact: &RehearsalFactRow,
    answer: Option<crate::dto::rehearsal::RehearsalAnswer>,
    w: &RehearsalWording,
    chrome: &RehearsalChromeWording,
    settings: &Settings,
) -> RehearsalInstance {
    // Safe by construction: `usable_fact` refused a quote that is absent or blank,
    // and this is only reached through it.
    let quote = fact.quote.as_deref().unwrap_or_default().trim().to_string();
    let (when, when_gap) = when_of(fact, w);
    let answered = answer.is_some();

    RehearsalInstance {
        position,
        // Forum first, then the date — see `rehearsal_phase`. Always a label:
        // a card with no chip in a filtered list reads as a rendering fault.
        phase: crate::services::rehearsal_phase::phase_of(
            fact.document_id.as_deref(),
            fact.occurred_on.as_deref(),
            settings,
        ),
        who: who_of(fact, w),
        when,
        when_gap,
        source: source_of(fact, w),
        kind_label: kind_of(fact),
        quote_first_line: first_line(&quote),
        quote,
        // Chosen HERE rather than in the browser: two labels and a boolean is a
        // choice a client can get backwards, and backwards means a green
        // ANSWERED over a row nobody has answered.
        answer_tag: if answered {
            chrome.answered_tag.clone()
        } else {
            chrome.no_answer_tag.clone()
        },
        answer_banner: (!answered).then(|| chrome.no_answer_banner.clone()),
        answer,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sort a list of date keys through the REAL comparator.
    fn ordered(keys: Vec<Option<&str>>) -> Vec<Option<&str>> {
        let mut keys = keys;
        keys.sort_by(|a, b| chronologically(*a, *b));
        keys
    }

    /// Handed in the wrong order, they come back in the right one.
    ///
    /// This is the assertion the removal of the separate TIMELINE section rests
    /// on: if this list is not chronological the page has no chronology at all,
    /// and nothing else on it would look wrong.
    #[test]
    fn the_stored_date_decides_the_order_not_the_click_order() {
        assert_eq!(
            ordered(vec![Some("2012-01-12"), Some("2009-12-15")]),
            vec![Some("2009-12-15"), Some("2012-01-12")]
        );
    }

    /// Variable precision sorts as TIME, which is why the stored string is the key.
    #[test]
    fn a_year_only_date_sorts_correctly_against_a_full_one() {
        assert_eq!(
            ordered(vec![
                Some("2011-03-01"),
                Some("2005"),
                Some("2009-12"),
                Some("2015-10")
            ]),
            vec![
                Some("2005"),
                Some("2009-12"),
                Some("2011-03-01"),
                Some("2015-10")
            ]
        );
    }

    /// Undated goes LAST, whichever end it was handed in from.
    #[test]
    fn undated_statements_sort_after_every_dated_one() {
        assert_eq!(
            ordered(vec![None, Some("2009-12"), None, Some("2011-03")]),
            vec![Some("2009-12"), Some("2011-03"), None, None]
        );
    }
}
