//! The prep page's card-backed sections, gathered (FACT_CARD_v2 §3).
//!
//! Split out of `services::rehearsal_assembly`, which was already past the
//! 300-line module limit before this task. The seam is real: everything here is
//! about the scenario's CARDS — the sentences a human wrote — while the assembly
//! beside it is about its instances, its points and its notes.
//!
//! ## Why these are here and not in `rehearsal_cards`
//!
//! Two of them do I/O. `rehearsal_cards` is the pure half — it decides which
//! statements are the other side's and what order they read in, and touches no
//! database — and this module is what feeds it and dresses its answers in the
//! wire types. The same split `scenario_card_assembly` documents.

use std::collections::HashMap;

use sqlx::PgPool;

use crate::domain::scenario_code::candidate_code;
use crate::domain::settings::Settings;
use crate::dto::rehearsal::RehearsalPoint;
use crate::dto::rehearsal::{RehearsalAccusationCard, RehearsalCardWatch, RehearsalPointProof};
use crate::repositories::pipeline_repository::scenario_fact_cards::{
    list_cards_for_scenario, CardRecord,
};
use crate::repositories::pipeline_repository::{
    list_items_for_response, sole_response_for_scenario,
};
use crate::repositories::scenario_accusation_repository::{
    fetch_rehearsal_facts, RehearsalFactRow,
};
use crate::services::rehearsal_cards;
use crate::services::rehearsal_render::CardSections;

use super::rehearsal_assembly::AssemblyError;

/// Both card-driven views of one scenario, from one pair of reads.
///
/// Split out of `rehearsal_assembly::assemble_scenario` for the function-size
/// limit (Rule 18), and it earns the split: the two views MUST be built from the
/// same `DeckInput`. A future caller that read the cards twice — once for the
/// sections, once for the backing — could show an accusation on one half of the
/// page and not the other, which is the one failure this page cannot afford.
///
/// ## Rust Learning: returning a tuple of two owned values
///
/// `DeckInput` borrows the two maps read here, so it cannot outlive this
/// function. Returning the two FINISHED views instead of the deck hands the
/// caller owned data and lets the borrows end at the closing brace — no lifetime
/// parameter travels back up into the assembly.
pub(crate) async fn card_views(
    pool: &PgPool,
    graph: &neo4rs::Graph,
    scenario_id: uuid::Uuid,
    ordinals: &HashMap<String, i32>,
    settings: &Settings,
) -> Result<(CardSections, HashMap<i32, Vec<RehearsalPointProof>>), AssemblyError> {
    let cards = card_index(pool, scenario_id).await?;
    let facts = card_statements(graph, &cards).await?;
    let deck = rehearsal_cards::DeckInput {
        cards: &cards,
        facts: &facts,
        ordinals,
        our_side: &settings.rehearsal_our_side_speakers,
    };
    Ok((card_sections(deck, settings), point_proofs(deck, ordinals)))
}

/// This scenario's fact cards, by node id (FACT_CARD_v2 §3).
pub(crate) async fn card_index(
    pool: &PgPool,
    scenario_id: uuid::Uuid,
) -> Result<HashMap<String, CardRecord>, AssemblyError> {
    Ok(list_cards_for_scenario(pool, scenario_id)
        .await
        .map_err(|source| AssemblyError::Read { source })?
        .into_iter()
        .map(|card| (card.graph_node_id.clone(), card))
        .collect())
}

/// The statements behind those cards, by node id.
///
/// The SAME read the placed instances use, so one statement reads identically
/// wherever this page shows it. An empty card set skips the query entirely —
/// `IN []` is a round trip that can only return nothing.
pub(crate) async fn card_statements(
    graph: &neo4rs::Graph,
    cards: &HashMap<String, CardRecord>,
) -> Result<HashMap<String, RehearsalFactRow>, AssemblyError> {
    if cards.is_empty() {
        return Ok(HashMap::new());
    }
    let mut ids: Vec<String> = cards.keys().cloned().collect();
    ids.sort();
    fetch_rehearsal_facts(graph, &ids)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| (row.graph_node_id.clone(), row))
                .collect()
        })
        .map_err(|source| AssemblyError::Record { source })
}

/// The two scenario-level §3 views, with the stored gap sentence.
pub(crate) fn card_sections(
    deck: rehearsal_cards::DeckInput<'_>,
    settings: &Settings,
) -> CardSections {
    CardSections {
        accusation: deck_accusations(deck, settings),
        watch_for: rehearsal_cards::watch_outs(deck)
            .into_iter()
            .map(|watch| RehearsalCardWatch {
                code: watch.code,
                text: watch.text,
            })
            .collect(),
        gap: settings.fact_card_wording.no_cards_notice.clone(),
    }
}

/// The Accusation section, each card carrying the stored gap sentence for an
/// answer nobody has written.
///
/// The gap travels on EVERY card rather than once on the section: the section
/// renders a mixed list, and a client holding one notice would have to decide
/// per row whether it applied — which is the browser making a judgment about an
/// absence, the thing the honest-gap law puts on this side of the wire.
fn deck_accusations(
    deck: rehearsal_cards::DeckInput<'_>,
    settings: &Settings,
) -> Vec<RehearsalAccusationCard> {
    rehearsal_cards::accusation_cards(deck)
        .into_iter()
        .map(|row| RehearsalAccusationCard {
            code: deck
                .ordinals
                .get(&row.graph_node_id)
                .copied()
                .map(candidate_code),
            title: row.title,
            who: row.who,
            when: row.when,
            quote: row.quote,
            answer: row.answer,
            answer_gap: settings.fact_card_wording.no_answer_notice.clone(),
        })
        .collect()
}

/// Each point's backing proof, keyed by its 1-based position.
pub(crate) fn point_proofs(
    deck: rehearsal_cards::DeckInput<'_>,
    ordinals: &HashMap<String, i32>,
) -> HashMap<i32, Vec<RehearsalPointProof>> {
    rehearsal_cards::point_backing(deck)
        .into_iter()
        .map(|(position, proofs)| {
            (
                position,
                proofs
                    .into_iter()
                    .map(|proof| RehearsalPointProof {
                        code: ordinals
                            .get(&proof.graph_node_id)
                            .copied()
                            .map(candidate_code),
                        title: proof.title,
                        who: proof.who,
                        when: proof.when,
                        quote: proof.quote,
                    })
                    .collect(),
            )
        })
        .collect()
}

/// One scenario's talking points, ordered, capped, and each with its backing.
///
/// ## Why this moved here (FACT_CARD_v2 §3)
///
/// It read points and nothing else until §3 gave each one the Proof of every card
/// that backs it — which makes it a CARD-BACKED section, the third one this
/// module gathers. Keeping it beside `point_proofs`, which builds what it
/// consumes, also took `rehearsal_assembly` back under the 300-line limit.
///
/// The original doc follows.
///
/// One scenario's talking points, ordered and capped.
///
/// ## Why `scenario_responses.status` is NOT consulted — read before changing
///
/// Ruled 2026-08-01 and carried forward verbatim from the code this replaces: the
/// SCENARIO's readiness is the only gate. Every 1.4 write path sets that column to
/// `'draft'`, so filtering on it would show an empty block forever with no error —
/// the silent-empty failure this codebase keeps removing.
pub(crate) async fn talking_points_of(
    pool: &PgPool,
    scenario_id: uuid::Uuid,
    settings: &Settings,
    // FACT_CARD_v2 §3: each point's backing proof, by its 1-based position. The
    // 3.9 pairing, read off `backs_position` rather than authored.
    backing: &HashMap<i32, Vec<crate::dto::rehearsal::RehearsalPointProof>>,
) -> Result<Vec<RehearsalPoint>, AssemblyError> {
    // The guarded read (task R1 Piece 6). This site had no multi-row warning at
    // all until .390, and it is the one that feeds a witness: a second response
    // row would have silently rehearsed the older row's points.
    let response = sole_response_for_scenario(pool, scenario_id)
        .await
        .map_err(|source| AssemblyError::Read { source })?;

    let Some(response) = response else {
        return Ok(Vec::new());
    };

    let items = list_items_for_response(pool, response.id)
        .await
        .map_err(|source| AssemblyError::Read { source })?;

    Ok(items
        .into_iter()
        // The cap is a display law as well as a write law: a list that grew past
        // it — through a direct write, or a lowered cap — must still rehearse as
        // the few points a witness can hold.
        .take(settings.talking_points_cap)
        .map(|item| RehearsalPoint {
            // The STORED index, not the position in this iteration: the two agree
            // today, and would stop agreeing the moment the cap trimmed from the
            // front or a row went missing. Editing point 3 must address the row
            // the store calls 3, or the edit lands on the wrong sentence.
            //
            // ## Rust Learning: `usize::try_from` on an `i32`
            //
            // `item_index` is `i32` because Postgres `int` is signed. A negative
            // value cannot exist (the writer only ever inserts from `enumerate`),
            // but the compiler does not know that, so the conversion is fallible.
            // Falling back to 0 rather than panicking keeps a corrupted row from
            // taking down the page — and 0 is a position no route matches, so the
            // edit is refused rather than mis-applied.
            position: usize::try_from(item.item_index).unwrap_or(0) + 1,
            text: item.text,
            // The AUTHORED exhibit label is still task 3.9's and still does not
            // exist; measured on DEV, `response_item_fact_refs` holds zero rows.
            // Deriving one from the record would put words in the witness's
            // mouth, so it stays `None` and the named absence stays with it.
            exhibit: None,
            exhibit_notice: settings
                .rehearsal_chrome_wording
                .point_no_exhibit_notice
                .clone(),
            // FACT_CARD_v2 §3: the PROOF that backs this point, which needed no
            // editor after all — `backs_position` on the card already says which
            // point it is for. `position` is the 1-based number the file and the
            // card both use, which is why the map is keyed by it and not by
            // `item_index`.
            //
            // `item_index` is already an `i32` and `saturating_add` cannot
            // overflow into a wrong point — at `i32::MAX` it stays at `i32::MAX`,
            // which matches no stored position and yields an empty backing. No
            // conversion, so no error to discard.
            backing: backing
                .get(&item.item_index.saturating_add(1))
                .cloned()
                .unwrap_or_default(),
        })
        .collect())
}

#[cfg(test)]
#[path = "rehearsal_card_sections_tests.rs"]
mod tests;
