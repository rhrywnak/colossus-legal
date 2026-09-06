// =============================================================================
// backend/src/services/rehearsal_cards.rs — the prep page, read off the cards
// =============================================================================
//
// FACT_CARD_v2 §3, in one pure module. The rehearsal page's three sections stop
// being three unrelated reads and become three views of ONE thing: the cards a
// human wrote for this scenario.
//
//   * **The Accusation** — the other side's statements, oldest first, each with
//     her Answer. A card with none shows the stored gap sentence.
//   * **Her Points** — under each point, the Proof of every card that backs it.
//     §3 calls this "the 3.9 pairing", and it is: the pairing nobody had to
//     author, because `backs_position` already says which point a card is for.
//   * **What to Watch For** — every card's `watch_out`, with its C-code, before
//     the free-form items.
//
// ## Why this is read-only, and why that is the whole design
//
// §3 keeps the read-only rule: edits happen on the working page. So this module
// SELECTS and ORDERS and never composes a judgment — every sentence it hands out
// was written by a human or by Job B on the working surface, and this page shows
// it back.
//
// ## Everything here is pure
//
// The caller reads the cards and the statements once. Keeping this a function of
// its inputs is what makes "the other side is whoever is not us" a rule with a
// test rather than a query nobody can see.

use std::collections::HashMap;

use crate::domain::text::non_blank;
use crate::repositories::pipeline_repository::scenario_fact_cards::CardRecord;
use crate::repositories::scenario_accusation_repository::RehearsalFactRow;

/// One of the other side's statements, with her answer to it (§3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AccusationCard {
    pub(crate) graph_node_id: String,
    /// The card's title — what she says about this statement in three seconds.
    /// `None` when nobody has written one.
    pub title: Option<String>,
    /// Her reply, or `None` — which the caller renders as the stored gap
    /// sentence. NEVER hidden: an unanswered accusation is the most important
    /// thing on this page.
    pub answer: Option<String>,
    /// The statement's own date, for the ordering and the line above it.
    pub when: Option<String>,
    pub who: Option<String>,
    pub quote: Option<String>,
}

/// One card's proof, under the point it backs (§3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PointProof {
    pub graph_node_id: String,
    pub title: Option<String>,
    pub quote: Option<String>,
    pub who: Option<String>,
    pub when: Option<String>,
}

/// One card's warning, with the handle a human says out loud (§3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CardWatch {
    /// `"C-116"`, or `None` for a card nothing has numbered.
    pub code: Option<String>,
    pub text: String,
}

/// Everything the three views read.
///
/// ## Rust Learning: `pub(crate)` where the fields name a crate-private type
///
/// `RehearsalFactRow` is `pub(crate)`, so a `pub` struct exposing it would be a
/// public item nobody outside the crate could construct — which the compiler
/// warns about, rightly. Everything on this page is internal, so the struct is
/// `pub(crate)` too and the two visibilities agree.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DeckInput<'a> {
    /// This scenario's cards, by node id.
    pub cards: &'a HashMap<String, CardRecord>,
    /// The statements themselves, by node id.
    pub facts: &'a HashMap<String, RehearsalFactRow>,
    /// The C-code ordinals, by node id.
    pub ordinals: &'a HashMap<String, i32>,
    /// The speakers whose statements are OURS, lowercased (the stored list).
    pub our_side: &'a [String],
}

/// The other side's statements, oldest first, each with her answer (§3).
///
/// ## Domain note: "the other side" is defined by ELIMINATION
///
/// §3 names Phillips, CFS and the court — case-specific names Rule 2 keeps out of
/// code. The stored list is the inverse, and this is where that inversion is
/// applied: a statement is the other side's unless its speaker is one of ours.
///
/// A statement with NO recorded speaker counts as the other side. That is the
/// safe direction on this page: it shows an extra card she may not have to
/// answer, rather than hiding one she does.
pub(crate) fn accusation_cards(input: DeckInput<'_>) -> Vec<AccusationCard> {
    let mut rows: Vec<AccusationCard> = input
        .cards
        .values()
        .filter(|card| !is_ours(card, input))
        .map(|card| {
            let fact = input.facts.get(&card.graph_node_id);
            AccusationCard {
                graph_node_id: card.graph_node_id.clone(),
                title: non_blank(card.title.as_deref()),
                answer: non_blank(card.answer.as_deref()),
                when: fact.and_then(|f| f.occurred_on.clone()),
                who: fact.and_then(|f| f.speaker.clone()),
                quote: fact.and_then(|f| f.quote.clone()),
            }
        })
        .collect();

    // Chronological, and TOTAL: undated statements sort last (a missing date
    // cannot be the oldest), then by node id so two reads of unchanged data
    // cannot disagree about the order of two cards from the same day.
    rows.sort_by(|a, b| {
        date_key(a.when.as_deref())
            .cmp(&date_key(b.when.as_deref()))
            .then_with(|| a.graph_node_id.cmp(&b.graph_node_id))
    });
    rows
}

/// Whether a card's statement is one of ours.
///
/// Compared LOWERCASED on both sides: `token_list_of` folds case on the stored
/// list, and the graph holds "George Phillips" and "George R. Phillips" as two
/// rows — a case-sensitive test would have been a third way for one man to be two
/// people.
fn is_ours(card: &CardRecord, input: DeckInput<'_>) -> bool {
    let Some(speaker) = input
        .facts
        .get(&card.graph_node_id)
        .and_then(|f| f.speaker.as_deref())
    else {
        // No recorded speaker: documentary evidence, and not ours to claim.
        return false;
    };
    let folded = speaker.trim().to_lowercase();
    input.our_side.iter().any(|ours| ours == &folded)
}

/// Each point's backing proof, keyed by the point's 1-based position (§3).
///
/// ## Domain note: this IS the 3.9 pairing
///
/// Tracker task 3.9 was "pair each talking point with the exhibit that backs it",
/// and the rehearsal page has carried an `exhibit: None` and a named absence
/// waiting for it. The pairing turns out to need no editor at all:
/// `backs_position` already says which point a card is for, and Job B filled it
/// on the cards it drafted. This reads it back.
///
/// A card whose position names no point does not appear — the caller has the
/// points and looks up by position, so a stale pointer simply matches nothing.
/// That is the same safe outcome `fact_card_render::backs_line` takes.
pub(crate) fn point_backing(input: DeckInput<'_>) -> HashMap<i32, Vec<PointProof>> {
    let mut by_point: HashMap<i32, Vec<PointProof>> = HashMap::new();
    for card in input.cards.values() {
        let Some(position) = card.backs_position else {
            continue;
        };
        let fact = input.facts.get(&card.graph_node_id);
        by_point.entry(position).or_default().push(PointProof {
            graph_node_id: card.graph_node_id.clone(),
            title: non_blank(card.title.as_deref()),
            quote: fact.and_then(|f| f.quote.clone()),
            who: fact.and_then(|f| f.speaker.clone()),
            when: fact.and_then(|f| f.occurred_on.clone()),
        });
    }
    // Oldest first within a point, then by node id — the same total order the
    // Accusation section uses, so the page never orders two lists two ways.
    for proofs in by_point.values_mut() {
        proofs.sort_by(|a, b| {
            date_key(a.when.as_deref())
                .cmp(&date_key(b.when.as_deref()))
                .then_with(|| a.graph_node_id.cmp(&b.graph_node_id))
        });
    }
    by_point
}

/// Every card's warning, with its C-code (§3).
///
/// Ordered by code ordinal so the list reads in the same order the working page
/// numbers them; a card nothing has numbered sorts last, then by node id.
pub(crate) fn watch_outs(input: DeckInput<'_>) -> Vec<CardWatch> {
    let mut rows: Vec<(Option<i32>, String, CardWatch)> = input
        .cards
        .values()
        .filter_map(|card| {
            let text = non_blank(card.watch_out.as_deref())?;
            let ordinal = input.ordinals.get(&card.graph_node_id).copied();
            Some((
                ordinal,
                card.graph_node_id.clone(),
                CardWatch {
                    code: ordinal.map(crate::domain::scenario_code::candidate_code),
                    text,
                },
            ))
        })
        .collect();

    // `is_none()` leads the key so an un-numbered card sorts LAST (`false <
    // true`), matching the card list's own rule rather than inventing a second
    // convention for the same question.
    rows.sort_by(|a, b| (a.0.is_none(), a.0, &a.1).cmp(&(b.0.is_none(), b.0, &b.1)));
    rows.into_iter().map(|(_, _, watch)| watch).collect()
}

/// The sort key for a date: present dates first in order, absent last.
///
/// `YYYY-MM-DD` sorts lexicographically in date order, so no parsing is needed —
/// but only for values of that shape, and an EMPTY string would sort before every
/// real date. So blank is treated as absent, and absent sorts last: a missing
/// date cannot be the oldest thing in a chronology.
fn date_key(value: Option<&str>) -> (u8, &str) {
    match value {
        Some(date) if !date.trim().is_empty() => (0, date),
        _ => (1, ""),
    }
}

#[cfg(test)]
#[path = "rehearsal_cards_fixtures.rs"]
mod deck_fixtures;

#[cfg(test)]
#[path = "rehearsal_cards_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "rehearsal_cards_points_tests.rs"]
mod points_tests;
