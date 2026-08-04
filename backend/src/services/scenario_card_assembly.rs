//! Pool-level assembly for the candidate cards (task 1.2, v2 §7).
//!
//! `scenario_card` builds ONE card; this builds the response around them — the
//! working-pool / set-aside partition, the stable C-code ordering, and the key
//! that joins a candidate to its page text.
//!
//! ## Why this is a separate module from `scenario_card`
//!
//! Two reasons, and the second is the real one. The size rule forced a split, and
//! this is where the seam actually is: everything here reasons about the POOL (how
//! candidates relate to each other), while `scenario_card` reasons about one
//! candidate's content. A future change to the ordering or the partition touches
//! only this file; a change to what a card SAYS touches only that one.
//!
//! Everything here is pure — no I/O — which is why it lives in `services` rather
//! than beside the handler that performs the four reads.

use std::collections::HashMap;

use crate::bias::dto::BiasInstance;
use crate::domain::fact_status::FactStatus;
use crate::dto::scenario_card::{CardHumanLink, ScenarioCard, ScenarioCardsResponse};

use super::scenario_card::{build_card, CardRefState, CollapsedExtras};
use super::scenario_human_links::{link_progress, HumanTouches};
use crate::domain::settings::Settings;
use crate::repositories::pipeline_repository::EvidenceSummaryOverrideRecord;

/// Everything humans have done to a POOL, indexed by node.
///
/// ## Why the two maps travel as one argument
///
/// `assemble` was at seven arguments before task 2.10 and an eighth trips
/// `clippy::too_many_arguments`. The lint is right here rather than merely noisy:
/// these two are the same KIND of thing — per-node records of human acts, each
/// read once for the whole pool — and grouping them is what lets `build_card`
/// take one [`HumanTouches`] instead of two more `Option`s. A third human act
/// lands in both structs and changes neither signature.
#[derive(Debug, Clone, Copy)]
pub(crate) struct HumanTouchIndex<'a> {
    /// Corrections of the machine's questions, by node (task 1.7F Part B).
    pub question_overrides: &'a HashMap<String, EvidenceSummaryOverrideRecord>,
    /// Accusations humans have linked, by node (task 2.10).
    pub links: &'a HashMap<String, Vec<CardHumanLink>>,
}

/// Build every card and partition into the working pool and the set-aside list.
///
/// Pure — which is why it lives here beside `build_card` rather than in the
/// handler module: it carries domain knowledge (the set-aside partition, the
/// C-code ordering) and no HTTP semantics at all.
///
/// `settings` is threaded straight through to `build_card`. The handler reads the
/// snapshot from `AppState` once per request, so every card in one payload is
/// banded by the same cutoffs even if a human edits them mid-request (v2 §2b).
pub(crate) fn assemble(
    pool: Vec<BiasInstance>,
    extras: &HashMap<String, CollapsedExtras>,
    ref_states: &HashMap<String, CardRefState>,
    ordinals: &HashMap<String, i32>,
    page_text: &HashMap<String, String>,
    settings: &Settings,
    // Everything humans have done to this pool, indexed by node. See
    // [`HumanTouchIndex`] for why the two maps travel together.
    human: HumanTouchIndex<'_>,
) -> ScenarioCardsResponse {
    let default_state = CardRefState::default();
    // One empty slice, borrowed by every card that has no human links — which is
    // most of them. A `Vec::new()` per card would allocate 148 times to say
    // "nothing here".
    const NO_LINKS: &[CardHumanLink] = &[];
    let mut working: Vec<ScenarioCard> = Vec::new();
    let mut set_aside: Vec<ScenarioCard> = Vec::new();

    for instance in pool {
        let ref_state = ref_states
            .get(&instance.evidence_id)
            .unwrap_or(&default_state);

        // The page this quote sits on, if its text is stored.
        let page = instance.document.as_ref().and_then(|d| {
            instance
                .page_number
                .and_then(|p| page_text.get(&page_key(&d.id, p)))
        });

        let card = build_card(
            &instance,
            extras.get(&instance.evidence_id),
            ref_state,
            ordinals.get(&instance.evidence_id).copied(),
            page.map(|s| s.as_str()),
            settings,
            HumanTouches {
                question_override: human.question_overrides.get(&instance.evidence_id),
                links: human
                    .links
                    .get(&instance.evidence_id)
                    .map(Vec::as_slice)
                    .unwrap_or(NO_LINKS),
            },
        );

        // Set-aside items are kept in their own list so the client partitions
        // nothing — the same split the gather endpoint already serves.
        if ref_state.status == Some(FactStatus::Dropped) {
            set_aside.push(card);
        } else {
            working.push(card);
        }
    }

    // The stable order task 1.1 established: ascending C-ordinal, un-numbered
    // last, ties broken by node id. §7.8's binding clause is that confidence is
    // never the default sort — it is not a sort key here at all.
    sort_by_code(&mut working);
    sort_by_code(&mut set_aside);

    // The stuck pile's progress line, counted over the FINISHED cards — both
    // lists, because a set-aside card the machine never linked was still stuck
    // and still counts toward the work (task 2.10, the 1.7E-a pool-truth ruling).
    let link_progress = link_progress(working.iter().chain(set_aside.iter()), &settings.wording);

    ScenarioCardsResponse {
        pool: working,
        set_aside,
        link_progress,
    }
}

/// Sort by the numeric part of the `C-n` code, un-numbered last.
///
/// ## Why not sort the string
///
/// `"C-10" < "C-9"` lexicographically. The code is a rendered identity, so the
/// sort reads the ordinal back out of it rather than comparing display strings —
/// which is also why the card carries `code` and the ordering is derived here
/// rather than trusting the client to re-derive it.
fn sort_by_code(cards: &mut [ScenarioCard]) {
    cards.sort_by_key(|c| {
        let ordinal = c
            .code
            .as_deref()
            .and_then(|code| code.strip_prefix("C-"))
            // best-effort: a code that does not parse as an integer sorts last
            // with the un-numbered candidates. The parse error is not operator-
            // actionable (the code is generated by this system, so a malformed one
            // is a bug the ordering cannot fix) and discarding it here keeps the
            // sort total rather than fallible.
            .and_then(|n| n.parse::<i32>().ok());
        (ordinal.is_none(), ordinal, c.graph_node_id.clone())
    });
}

/// The `doc_id:page` key for the page-text index.
///
/// Shared by the endpoint that BUILDS the index and the assembly that READS it —
/// one function so the two can never disagree about the key's shape, which would
/// silently give every card empty context.
pub(crate) fn page_key(document_id: &str, page: i64) -> String {
    format!("{document_id}:{page}")
}

#[cfg(test)]
#[path = "scenario_card_assembly_tests.rs"]
mod tests;
