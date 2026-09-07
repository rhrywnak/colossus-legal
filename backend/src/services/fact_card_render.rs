// =============================================================================
// backend/src/services/fact_card_render.rs — a stored card, as a witness reads it
// =============================================================================
//
// FACT_CARD_v2 §2, in one pure function. A [`CardRecord`] holds five raw values;
// this turns them into the finished lines the browser renders and nothing else.
//
// ## Why this is pure, and why it is not in the repository
//
// It needs three things a repository has no business holding: the stored wording,
// the case's accusation catalogue, and the scenario's talking points. And it
// needs none of them at I/O time — the caller reads all three once for a whole
// deck. Keeping it pure is what makes every rule below testable from a literal.
//
// ## The one rule this file exists to enforce
//
// NOTHING IS HIDDEN FOR LACKING A FIELD. An absent title, an unbacked point, a
// card that names no accusation — each renders as the stored em dash, and the row
// stays. §2 says so, and it is the difference between a deck that shows Chuck
// what he still owes and one that quietly looks finished. (The mockup of record
// hides a card with no Answer; the instruction governs.)

use std::collections::HashMap;

use crate::domain::fact_card::{CardAuthor, CardField, CardStance, SUPPORTS_CAP};
use crate::domain::scenario_code::allegation_code;
use crate::domain::text::non_blank;
use crate::domain::wording_fact_card::FactCardWording;
use crate::dto::fact_card::{CardSupportRef, FactCardBlock, FactCardDrafts};
use crate::repositories::pipeline_repository::scenario_fact_cards::CardRecord;

/// One accusation, as the card needs to name it.
///
/// A narrow struct rather than the repository's own row: this module composes
/// three strings and must not be able to reach a pool count or an element list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllegationLabel {
    /// The complaint paragraph — `"21"`. `None` for an accusation the extraction
    /// numbered no paragraph for; the code is then omitted rather than invented.
    pub paragraph: Option<String>,
    /// What the accusation says, in the complaint's own words.
    pub text: Option<String>,
    /// The count it goes to, if the accusation is wired to an element yet.
    /// `None` for 25 of the case's 120, measured — a real state.
    pub count_tag: Option<String>,
}

/// Everything the composer needs beyond the card row itself.
///
/// ## Rust Learning: borrowed maps in a params struct
///
/// All three are read-only and live for the whole request, so they are borrowed
/// rather than cloned per card — a deck of forty cards would otherwise clone the
/// 120-accusation catalogue forty times. The lifetime `'a` ties this struct to
/// them, which the compiler checks.
#[derive(Debug, Clone, Copy)]
pub struct RenderContext<'a> {
    /// Accusation graph id → how to name it.
    pub allegations: &'a HashMap<String, AllegationLabel>,
    /// The scenario's talking points, by their 1-based position.
    pub talking_points: &'a HashMap<i32, String>,
    pub wording: &'a FactCardWording,
}

/// Compose one stored card into the block the browser renders.
pub fn render_card(record: &CardRecord, context: RenderContext<'_>) -> FactCardBlock {
    // A malformed `supports` column renders as NO accusations rather than
    // refusing the card: the title, the answer and the watch-out are still the
    // witness's, and withholding them over one bad list would take four good
    // sentences off the page. The repository's own error names the card, and the
    // caller logs it.
    let supports = record.supports_list().unwrap_or_default();
    let refs = support_refs(&supports, context);

    FactCardBlock {
        title: non_blank(record.title.as_deref()),
        backs: backs_line(record.backs_position, context),
        backs_position: record.backs_position,
        supports: refs.iter().map(|r| supports_line(r, context)).collect(),
        count_tags: count_tags(&refs, context),
        supports_refs: refs,
        watch_out: non_blank(record.watch_out.as_deref()),
        answer: non_blank(record.answer.as_deref()),
        drafts: drafts(record),
    }
}

/// Attach every stored card to the response, in place.
///
/// ## Why this runs AFTER assembly rather than inside it
///
/// `assemble` is per-candidate and already at the argument count
/// `clippy::too_many_arguments` refuses to widen; the stored cards are a
/// SCENARIO-wide read, keyed by node. Attaching afterwards is the same shape
/// `attach_ruled_reasons` takes, for the same reason: a fact about the whole
/// deck, laid over cards that were built one at a time.
///
/// A card with no stored row keeps `card: None` — absent, not an empty block —
/// so "nobody has drafted this" and "somebody started and stopped" stay
/// different states on the wire.
pub fn attach_fact_cards(
    response: &mut crate::dto::scenario_card::ScenarioCardsResponse,
    cards: &HashMap<String, CardRecord>,
    context: RenderContext<'_>,
) {
    for card in response
        .pool
        .iter_mut()
        .chain(response.set_aside.iter_mut())
    {
        if let Some(record) = cards.get(&card.graph_node_id) {
            card.card = Some(render_card(record, context));
        }
    }
}

/// The accusations this card names, capped and labelled.
///
/// ## Why the cap is applied on READ and not only on write
///
/// The loader refuses a third accusation, and so does the edit route. This is the
/// third place, and it is the one that matters if either of the first two is ever
/// bypassed: a row edited around the API with five accusations would otherwise
/// render five lines in a slot laid out for one, which is a broken card rather
/// than a refused write. Truncating is the honest degradation — the extras are
/// still in the column, and the count is visible to an operator.
fn support_refs(
    supports: &[crate::repositories::pipeline_repository::scenario_fact_cards::CardSupport],
    context: RenderContext<'_>,
) -> Vec<CardSupportRef> {
    supports
        .iter()
        .take(SUPPORTS_CAP)
        .map(|s| CardSupportRef {
            allegation_id: s.allegation_id.clone(),
            stance: s.stance,
            // The handle a lawyer cites. An accusation with no paragraph number
            // gets an EMPTY code rather than "A-": half a citation is worse than
            // none, and the line still carries the accusation's own words.
            code: context
                .allegations
                .get(&s.allegation_id)
                .and_then(|a| a.paragraph.as_deref())
                .map(allegation_code)
                .unwrap_or_default(),
        })
        .collect()
}

/// One Supports line — "Supports A-21 — CFS could have returned the money."
///
/// The verb comes from the stance and the STORE, never from a literal: a reader
/// skimming five cards must not have to parse a negation, and Roman can rename
/// either word without a rebuild.
fn supports_line(reference: &CardSupportRef, context: RenderContext<'_>) -> String {
    let verb = match reference.stance {
        CardStance::Supports => &context.wording.stance_supports_verb,
        CardStance::Rebuts => &context.wording.stance_rebuts_verb,
    };
    let text = context
        .allegations
        .get(&reference.allegation_id)
        .and_then(|a| a.text.as_deref())
        .unwrap_or_default();
    context
        .wording
        .supports_template
        .replace("{verb}", verb)
        .replace("{code}", &reference.code)
        .replace("{text}", text)
}

/// The Backs line — "Point 2 — My sisters' claim rested on hearsay."
///
/// ## Domain note: a stale position renders NOTHING, not a half-line
///
/// `backs_position` points at a talking point by its 1-based position, and
/// `set_talking_points` deletes and re-inserts the whole list on an ordinary
/// edit — so a card can name position 3 of a list that now has two. "Point 3 — "
/// with nothing after it reads as a rendering fault; the em dash reads as "this
/// card backs no point", which is what is true of the list as it stands. The
/// number survives on `backs_position` so the editor can show what was stored.
fn backs_line(position: Option<i32>, context: RenderContext<'_>) -> Option<String> {
    let position = position?;
    let text = context.talking_points.get(&position)?;
    Some(
        context
            .wording
            .backs_template
            .replace("{position}", &position.to_string())
            .replace("{text}", text),
    )
}

/// The count tag(s) for the title bar, deduplicated, in the order named.
///
/// Two accusations of the same count produce ONE tag: the bar names which count
/// this card is FOR, and printing "Count 1" twice says nothing the first did not.
fn count_tags(refs: &[CardSupportRef], context: RenderContext<'_>) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for reference in refs {
        let Some(tag) = context
            .allegations
            .get(&reference.allegation_id)
            .and_then(|a| a.count_tag.clone())
        else {
            continue;
        };
        if !tags.contains(&tag) {
            tags.push(tag);
        }
    }
    tags
}

/// Which fields still carry the machine's mark.
///
/// Read per field from its OWN authorship column. A row-level answer would clear
/// every mark the moment one field was edited, and the mockup's own card shows an
/// edited Answer beside a still-drafted Watch out.
fn drafts(record: &CardRecord) -> FactCardDrafts {
    let is_draft = |field: CardField| {
        record
            .field_author(field)
            .is_some_and(|author| CardAuthor(author).is_draft())
    };
    FactCardDrafts {
        title: is_draft(CardField::Title),
        backs: is_draft(CardField::BacksPosition),
        supports: is_draft(CardField::Supports),
        watch_out: is_draft(CardField::WatchOut),
        answer: is_draft(CardField::Answer),
    }
}

#[cfg(test)]
#[path = "fact_card_render_fixtures.rs"]
mod render_fixtures;

#[cfg(test)]
#[path = "fact_card_render_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "fact_card_render_tags_tests.rs"]
mod tags_tests;

#[cfg(test)]
#[path = "fact_card_render_attach_tests.rs"]
mod attach_tests;
