//! What the loader WILL write, worked out before anything is written.
//!
//! ## Why a plan and not a loop of writes
//!
//! The dry run has to print exactly what the apply would do, and the only way to
//! guarantee that is for both to consume the same value. A loop that wrote as it
//! went would make the dry run a second implementation of the first — the shape
//! that lets a "verified" plan differ from what lands.
//!
//! Everything here is pure. Resolving a short accusation id to its graph node is
//! a LOOKUP against a map the caller read; the map's own read is `main`'s.

use anyhow::{bail, Result};
use std::collections::HashMap;

use colossus_legal_backend::domain::fact_card::{CardField, MACHINE_AUTHOR};

use super::model::{CandidatesFile, DraftedCard, TalkingPointsFile};

/// One field this run will write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedField {
    pub graph_node_id: String,
    pub field: CardField,
    /// `None` clears the field — which the loader never does; it is here because
    /// the writer's shape allows it and a plan that could not express a clear
    /// would hide that the writer can.
    pub value: Option<String>,
}

/// Everything one `--input` file becomes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CardPlan {
    pub fields: Vec<PlannedField>,
    /// How many cards the file held — the number `--expect-count` is checked
    /// against.
    pub cards: usize,
    /// Accusations named by a short id nothing in the graph matches.
    ///
    /// Domain note: counted and NAMED, never silently dropped. A card that claims
    /// a link nothing answers is the stale-pointer defect of 2026-07-24, and the
    /// loader is where it is cheapest to see.
    pub unresolved_accusations: Vec<String>,
}

/// Turn Job B's drafted cards into the fields to write.
///
/// `allegations` maps a SHORT id (the graph id's hash suffix, which is what Job B
/// wrote) to the full node id. An accusation it cannot resolve is dropped from
/// that card's list and recorded in `unresolved_accusations` — the rest of the
/// card is still written, because four good sentences are worth more than a
/// refusal over one bad pointer.
///
/// # Errors
/// Returns the first card that breaks the title or accusation cap. Nothing is
/// planned when it does: the file is the record of what a model decided, and a
/// file produced against a different rule is not one to half-load.
pub fn plan_cards(
    cards: &[DraftedCard],
    allegations: &HashMap<String, String>,
    title_word_limit: usize,
) -> Result<CardPlan> {
    let mut plan = CardPlan {
        cards: cards.len(),
        ..CardPlan::default()
    };

    for card in cards {
        let node = card.card_id.clone();
        push(
            &mut plan,
            &node,
            CardField::Title,
            Some(card.checked_title(title_word_limit)?.to_string()),
        );
        push(
            &mut plan,
            &node,
            CardField::BacksPosition,
            card.backs.map(|b| b.to_string()),
        );
        push(
            &mut plan,
            &node,
            CardField::WatchOut,
            card.watch_out.clone(),
        );
        push(
            &mut plan,
            &node,
            CardField::Answer,
            card.answer_draft.clone(),
        );
        plan_supports(&mut plan, card, &node, allegations)?;
    }
    Ok(plan)
}

/// Resolve one card's Supports entries and plan the column write.
///
/// Split out of [`plan_cards`] for the function-size limit (Rule 18).
///
/// ## Domain note: an unresolved accusation is REPORTED, never dropped
///
/// A Job B file names accusations by short id (`A-17`). If the id is not in the
/// live allegation table the entry does NOT silently vanish — it lands in
/// `unresolved_accusations` so the dry run prints it, because a card that quietly
/// lost the count it answers is a card the witness would rehearse against nothing.
///
/// An entirely unresolved card writes NO Supports column at all: an empty JSON
/// array would read on the card as "this answers nothing", which is a different
/// and false claim from "we could not match what it answers".
fn plan_supports(
    plan: &mut CardPlan,
    card: &DraftedCard,
    node: &str,
    allegations: &HashMap<String, String>,
) -> Result<()> {
    let mut resolved = Vec::new();
    for entry in card.checked_supports()? {
        match allegations.get(&entry.allegation_id) {
            Some(full) => resolved.push(serde_json::json!({
                "allegation_id": full,
                "stance": entry.stance,
            })),
            None => plan
                .unresolved_accusations
                .push(format!("{} → {}", node, entry.allegation_id)),
        }
    }
    if !resolved.is_empty() {
        push(
            plan,
            node,
            CardField::Supports,
            Some(serde_json::Value::Array(resolved).to_string()),
        );
    }
    Ok(())
}

/// Add one field to the plan, unless there is nothing to write.
///
/// ## Domain note: an absent draft is NOT written as a clear
///
/// Job B leaves `watch_out` and `answer_draft` out on some cards. Writing those
/// as NULL would stamp `machine:job_b_v1` on a field the machine never wrote,
/// and the card would then show a draft mark over an em dash — a mark claiming
/// the machine had drafted an emptiness.
fn push(plan: &mut CardPlan, node: &str, field: CardField, value: Option<String>) {
    let Some(value) = value else { return };
    if value.trim().is_empty() {
        return;
    }
    plan.fields.push(PlannedField {
        graph_node_id: node.to_string(),
        field,
        value: Some(value),
    });
}

/// The author every field this loader writes carries.
pub const LOADER_AUTHOR: &str = MACHINE_AUTHOR;

/// One scenario's talking points, ready to insert.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedPoints {
    pub scenario_code: String,
    pub scenario_id: uuid::Uuid,
    /// `(item_index, text)` — 0-BASED, as `response_items` stores it. The file's
    /// `position` is 1-based, and this is where that translation happens.
    pub items: Vec<(i32, String)>,
}

/// Turn a talking-points file into the rows to insert.
///
/// # Errors
/// Returns the scenario whose positions are not `1..=n` — a gap or a repeat would
/// make a card's `backs_position` point at the wrong sentence, and every card in
/// that scenario names one by number.
pub fn plan_points(files: &[TalkingPointsFile]) -> Result<Vec<PlannedPoints>> {
    files
        .iter()
        .map(|file| {
            let mut positions: Vec<i32> = file.talking_points.iter().map(|p| p.position).collect();
            positions.sort_unstable();
            let expected: Vec<i32> = (1..=positions.len() as i32).collect();
            if positions != expected {
                bail!(
                    "{} has talking-point positions {positions:?}, and a card's \
                     backs_position names one by number — they must be 1..={}",
                    file.scenario_code,
                    positions.len()
                );
            }
            Ok(PlannedPoints {
                scenario_code: file.scenario_code.clone(),
                scenario_id: file.scenario_id,
                items: file
                    .talking_points
                    .iter()
                    .map(|p| (p.position - 1, p.text.clone()))
                    .collect(),
            })
        })
        .collect()
}

/// One picked candidate, ready to include.
///
/// A named struct rather than the `(String, i32, String)` tuple this was: three
/// of its four fields are string-shaped, and a tuple made it possible to write the
/// ranker's reason into the title slot with no compile error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedPick {
    pub graph_node_id: String,
    pub sort_ordinal: i32,
    pub title: String,
    /// Job D's sentence saying why it picked this card (integration ruling R2).
    ///
    /// ## Domain note: stored on the EVENT, never on the card
    ///
    /// §1's card carries the witness's own five sentences. "Why the ranker picked
    /// this" is a claim about the CHOICE, and on a card it would read to Marie as
    /// something she is meant to say — mapping it onto `watch_out` would put a
    /// ranking note where she expects a warning about the other side. It rides
    /// the include event's `note` in `scenario_fact_card_events` instead, which
    /// is where the account of an act belongs.
    pub reason: Option<String>,
}

/// One scenario's picks, ready to include.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedPicks {
    pub scenario_code: String,
    pub scenario_id: uuid::Uuid,
    /// In display order — the pick number IS the order.
    pub picks: Vec<PlannedPick>,
    /// How many of those picks carry a reason for the ledger. Counted here so the
    /// dry run can say what the apply will record before it records it.
    pub stored_reasons: usize,
}

/// The ordinal step the picks are spaced by.
//
// STRUCTURAL: the same branching factor `services::scenario_fact_order` uses, and
// it must be the same one — a pick placed at a number outside that scheme's
// spacing would leave no room for a human to drag a card between two picks.
const ORDINAL_STEP: i32 = colossus_legal_backend::services::scenario_fact_order::ORDINAL_STEP;

/// Turn a candidates file into the facts to include, in display order.
///
/// # Errors
/// Returns the scenario whose picks are not `1..=n`: the pick number IS the
/// display order, and a gap would leave two facts claiming one seat.
pub fn plan_picks(files: &[CandidatesFile]) -> Result<Vec<PlannedPicks>> {
    files
        .iter()
        .map(|file| {
            let mut ordered = file.picks.clone();
            ordered.sort_by_key(|p| p.pick);
            let numbers: Vec<i32> = ordered.iter().map(|p| p.pick).collect();
            let expected: Vec<i32> = (1..=numbers.len() as i32).collect();
            if numbers != expected {
                bail!(
                    "{} has pick numbers {numbers:?}, and the pick number IS the \
                     display order — they must be 1..={}",
                    file.scenario_code,
                    numbers.len()
                );
            }
            Ok(PlannedPicks {
                scenario_code: file.scenario_code.clone(),
                scenario_id: file.scenario_id,
                stored_reasons: ordered.iter().filter(|p| p.reason.is_some()).count(),
                picks: ordered
                    .iter()
                    .map(|p| PlannedPick {
                        graph_node_id: p.graph_node_id.clone(),
                        sort_ordinal: p.pick.saturating_mul(ORDINAL_STEP),
                        title: p.title.clone(),
                        reason: p.reason.clone(),
                    })
                    .collect(),
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "plan_tests.rs"]
mod tests;
