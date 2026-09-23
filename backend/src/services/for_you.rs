//! Whose list a viewer is served, and the page built out of it.
//!
//! CC_TASK_FOR_YOU_v1 L1. Two decisions live here and nowhere else: WHICH side
//! a signed-in person belongs to, and what the page says once the rows are in.
//! Neither touches a database — the reads are the route's, so both are unit-
//! testable without a pool.
//!
//! ## Domain note: PERMISSION decides the reviewer's side, not a display list
//!
//! `may_review` is the one place that answers "may this person review", and it
//! answers "listed reviewer OR administrator" (CC_TASK_REVIEW_PERMISSION_v1).
//! So an administrator who took himself off the war room's display list still
//! gets the reviewers' list here — which is the whole point of that task,
//! carried into this page. The stored list still decides whose WORK does not
//! wait, in SQL, one layer down.
//!
//! ## Domain note: somebody who is neither is not an error
//!
//! A signed-in person who is not a reviewer, not an administrator and not the
//! witness gets [`ForYouSide::None`]: an empty list and a sentence saying whose
//! page this is. A 403 would be wrong — nothing was refused — and an empty list
//! with no explanation would read as "you are up to date", which is a different
//! fact and the wrong one to tell somebody.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::domain::wording_templates::render;
use crate::dto::for_you::{ForYouPayload, ForYouRowDto, ForYouSide};
use crate::dto::for_you_wording::ForYouWordingDto;
use crate::repositories::pipeline_repository::waiting_items::{WaitingItemRow, WaitingSide};
use crate::services::for_you_rows::RowVoice;
use crate::services::practice_clock::local_stamp;
use crate::services::review_permission::may_review;

/// Which list this signed-in person is served.
///
/// The order matters: a person who is BOTH a reviewer and the witness — which
/// a settings edit can produce — is served the reviewers' list, because that is
/// the side with a review duty attached to it.
pub fn side_for(user_id: &str, is_admin: bool, reviewers: &[String], witness: &str) -> ForYouSide {
    if may_review(user_id, is_admin, reviewers) {
        ForYouSide::Reviewers
    } else if !witness.trim().is_empty() && user_id == witness {
        ForYouSide::Witness
    } else {
        ForYouSide::None
    }
}

/// The repository's side, for a page side that has one.
///
/// `None` for [`ForYouSide::None`]: there is no query to run, so the route does
/// not run one. Returning an empty list from a query nobody should ask is how a
/// page ends up costing four statements to say nothing.
pub fn query_side(side: ForYouSide) -> Option<WaitingSide> {
    match side {
        ForYouSide::Reviewers => Some(WaitingSide::Reviewers),
        ForYouSide::Witness => Some(WaitingSide::Witness),
        ForYouSide::None => None,
    }
}

/// The page, composed from the two reads.
///
/// `other_side` is the name of whoever this reader is waiting on — the reviewer
/// bench for the witness, the witness for a reviewer. It is what fills `{who}`
/// in the subtitle and in the empty state, so the page says where the next row
/// will come from rather than only that there is none.
pub fn assemble_page(
    voice: &RowVoice<'_>,
    other_side: &str,
    unread: &[WaitingItemRow],
    everything: &[WaitingItemRow],
    deck_threshold: u32,
) -> ForYouPayload {
    let w = voice.wording;
    let unread_count = unread.len() as u32;
    let subtitle = match voice.side {
        ForYouSide::Witness => render(&w.subtitle_witness, &[("who", other_side)]),
        ForYouSide::Reviewers => render(&w.subtitle_reviewer, &[("who", other_side)]),
        // Nobody's list: the page's own sentence stands in for a subtitle, and
        // the page renders it instead of the list.
        ForYouSide::None => w.not_your_list.clone(),
    };
    ForYouPayload {
        side: voice.side,
        subtitle,
        tab_unread_label: render(
            &w.tab_unread_template,
            &[("count", &unread_count.to_string())],
        ),
        tab_everything_label: render(
            &w.tab_everything_template,
            &[("count", &everything.len().to_string())],
        ),
        unread_count,
        unread: group_unread(voice, unread, deck_threshold),
        everything: everything.iter().map(|r| voice.compose(r)).collect(),
        empty_hint: render(&w.empty_hint_template, &[("who", other_side)]),
        // The newest item on this side, read or not — `everything` is ordered
        // newest first by the query. `None` when there has never been one, so
        // the line is withheld rather than rendered with an empty date.
        empty_last: everything.first().map(|row| {
            render(
                &w.empty_last_template,
                &[("when", &local_stamp(row.at, voice.timezone))],
            )
        }),
        wording: ForYouWordingDto::from_block(w),
    }
}

/// The unread list, with a busy deck collapsed into ONE row (ruling 2).
///
/// ## Domain note: what the threshold counts, and whose list it touches
///
/// UNREAD items, and a REVIEWER's list only. The witness answers one question
/// at a time, so her list is always per item — collapsing four notes into "S-3
/// · 4 answers waiting" would hide the one sentence she needs to read before
/// she can answer anything. The Everything tab is never grouped either: it is
/// the archive, and its whole use is finding one item again.
///
/// ## What the ORDER guarantees
///
/// `rows` arrives newest first. A deck row takes the place of its newest item,
/// so the list keeps one order whether or not a deck is grouped, and a deck
/// that has just received something stays at the top where the reader left it.
///
/// ## Rust Learning: two passes, not one
///
/// The first pass counts and finds each deck's oldest moment; the second emits.
/// One pass cannot do it — whether the FIRST row of a deck becomes a deck row
/// depends on how many more of that deck come later — and the alternative
/// (emitting, then rewriting) is the shape that eventually emits both.
pub fn group_unread(
    voice: &RowVoice<'_>,
    rows: &[WaitingItemRow],
    deck_threshold: u32,
) -> Vec<ForYouRowDto> {
    if voice.side != ForYouSide::Reviewers {
        return rows.iter().map(|r| voice.compose(r)).collect();
    }
    let decks = deck_tallies(rows);
    let mut out = Vec::with_capacity(rows.len());
    let mut grouped: Vec<Uuid> = Vec::new();
    for row in rows {
        let Some(tally) = decks.get(&row.scenario_id) else {
            // Unreachable: every row was counted above. Rendered as an item
            // rather than dropped, because a row missing from the page is the
            // one failure this page exists to prevent.
            out.push(voice.compose(row));
            continue;
        };
        if u32::try_from(tally.count).unwrap_or(u32::MAX) < deck_threshold.max(1) {
            out.push(voice.compose(row));
            continue;
        }
        if grouped.contains(&row.scenario_id) {
            continue;
        }
        grouped.push(row.scenario_id);
        out.push(voice.deck_row(row, tally.count, tally.oldest));
    }
    out
}

/// How much each deck holds, and since when.
struct DeckTally {
    count: usize,
    oldest: DateTime<Utc>,
}

/// Count the rows per deck and find each deck's oldest moment, in one pass.
fn deck_tallies(rows: &[WaitingItemRow]) -> HashMap<Uuid, DeckTally> {
    let mut decks: HashMap<Uuid, DeckTally> = HashMap::new();
    for row in rows {
        decks
            .entry(row.scenario_id)
            .and_modify(|tally| {
                tally.count += 1;
                // `min` rather than "the last one seen": the query orders by
                // `at DESC`, but a tie on the moment is broken by item id, and
                // a page must not print a different oldest date because two
                // notes landed in the same millisecond.
                tally.oldest = tally.oldest.min(row.at);
            })
            .or_insert(DeckTally {
                count: 1,
                oldest: row.at,
            });
    }
    decks
}

#[cfg(test)]
#[path = "for_you_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "for_you_group_tests.rs"]
mod group_tests;
