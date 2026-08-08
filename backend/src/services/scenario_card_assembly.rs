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
//! than beside the handler that performs the reads.
//!
//! ## Why the ref-row DECODERS moved here (2026-08-08)
//!
//! `build_ref_states` and `apply_display_order` used to live beside the handler,
//! and the projection pushed that module past the 300-line limit. This is where
//! they belonged anyway: both reason about the POOL rather than about HTTP — one
//! turns the whole scenario's reference rows into per-card state, the other
//! computes each included fact's place from every other fact's. Neither has any
//! transport semantics, and the handler now reads as four reads and an assembly.

use std::collections::HashMap;

use crate::bias::dto::BiasInstance;
use crate::domain::fact_status::FactStatus;
use crate::domain::fact_tier::FactTier;
use crate::dto::scenario_card::{CardHumanLink, ScenarioCard, ScenarioCardsResponse};
use crate::error::AppError;

use super::scenario_card::{build_card, CardRefState, CollapsedExtras};
use super::scenario_card_projection::{to_card_proposal, ProposalGroup};
use super::scenario_human_links::{link_progress, HumanTouches};
use crate::domain::settings::Settings;
use crate::repositories::pipeline_repository::EvidenceSummaryOverrideRecord;
use crate::services::scenario_fact_order::{effective_order, OrderableFact};

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

/// What the latest completed scan proposes, keyed by the card that carries it.
///
/// Empty when nothing projects — no completed run, or every admitted verdict
/// already ruled. A node absent from the map is simply not proposed; there is no
/// third state to check.
pub(crate) type ProposalIndex<'a> = HashMap<&'a str, &'a ProposalGroup>;

/// Everything known about a POOL that is not in the graph pool itself.
///
/// ## Why the two indexes travel as one argument
///
/// The same arithmetic that produced [`HumanTouchIndex`], one level up: `assemble`
/// was at seven arguments and the projection made an eighth. And the same
/// justification — these are per-node maps, each read ONCE for the whole pool and
/// consulted per card. What they are NOT is one kind of thing, which is why they
/// stay two named fields rather than merging: `human` is what a person did to a
/// candidate, `proposals` is what a machine said about one, and precedence R-a
/// exists precisely because those two must never be confused. Keeping the seam
/// visible in the type is the point.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PoolIndexes<'a> {
    pub human: HumanTouchIndex<'a>,
    pub proposals: &'a ProposalIndex<'a>,
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
    // Everything known about this pool that is not in the pool itself: what humans
    // have done to each candidate, and what the latest completed scan proposes
    // about it. See [`PoolIndexes`] for why they travel together and why they stay
    // two fields.
    indexes: PoolIndexes<'_>,
) -> ScenarioCardsResponse {
    let PoolIndexes { human, proposals } = indexes;
    // One empty slice, borrowed by every card that has no human links — which is
    // most of them. A `Vec::new()` per card would allocate 148 times to say
    // "nothing here".
    const NO_LINKS: &[CardHumanLink] = &[];
    let mut working: Vec<ScenarioCard> = Vec::new();
    let mut set_aside: Vec<ScenarioCard> = Vec::new();

    for instance in pool {
        // Owned rather than borrowed because a PROJECTED card's state is composed
        // right here and has no stored row to point at. One small clone per card
        // buys a single code path — the alternative was a borrow of either a stored
        // row or a temporary, which is the shape that needs a lifetime dance to
        // express and reads as if the two were different KINDS of state. They are
        // not: they are the same state from two sources, and R-a says which wins.
        let ref_state = match ref_states.get(&instance.evidence_id) {
            Some(stored) => stored.clone(),
            None => projected_state(
                proposals.get(instance.evidence_id.as_str()).copied(),
                ordinals,
                settings,
            ),
        };
        let ref_state = &ref_state;

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
        // Filled in by the route once it knows WHICH run these proposals came
        // from, exactly as `never_scanned_notice` is: the run's identity is a fact
        // about the history, in another database, and this assembler is documented
        // pure. The COUNT that rides with it is counted over this finished payload
        // (`count_proposed`), so the number and the cards it describes are the same
        // set by construction.
        proposal_source: None,
        // A pool was gathered, so these cards are the answer for a scenario that
        // HAS a subject. The no-target notice belongs to the route's early
        // return, which never reaches this assembler.
        no_target_notice: None,
        // Left `None` here for a different reason than the line above: whether
        // anything has scanned this scenario is a fact about the RUN HISTORY, in
        // another database, and this assembler is documented pure. The route fills
        // it in after this returns (task 2.15, piece 3).
        never_scanned_notice: None,
    }
}

/// The non-graph state of a card NOBODY has ruled: a proposal, or nothing.
///
/// Split out so the branch above reads as one decision rather than five lines of
/// construction. The two fields it fills are the whole of what a projection
/// contributes — the score (which the builder bands, §7.8) and the display-ready
/// proposal — and everything else stays at its `Default`, because a candidate with
/// no reference row genuinely has no status, no tier, no defer reason and no
/// position in a list it is not in.
fn projected_state(
    group: Option<&ProposalGroup>,
    ordinals: &HashMap<String, i32>,
    settings: &Settings,
) -> CardRefState {
    match group {
        None => CardRefState::default(),
        Some(group) => CardRefState {
            confidence: group.confidence,
            proposal: Some(to_card_proposal(group, ordinals, &settings.wording)),
            ..CardRefState::default()
        },
    }
}

/// How many served cards carry a proposal (R-e, the live number).
///
/// Counted over the FINISHED payload rather than accumulated during assembly, for
/// the same reason `api::scenario_cards::cards_without_context` is: the assembler's
/// output is what the §7 completeness tests assert against, and a counter mutated
/// inside the loop is a second derivation that can drift from the cards it claims
/// to describe. Both lists are walked because precedence puts an excluded card in
/// `set_aside` — it can carry no proposal, so the sum is unaffected, and walking
/// both means a future change to the partition cannot silently lose a number.
pub(crate) fn count_proposed(response: &ScenarioCardsResponse) -> usize {
    response
        .pool
        .iter()
        .chain(response.set_aside.iter())
        .filter(|card| card.proposed.is_some())
        .count()
}

/// Compute each INCLUDED fact's place in the list, and record it on its state.
///
/// ## Why this runs over the whole set rather than per card
///
/// Where a fact sits depends on every other fact: an unplaced one takes its
/// position from the tail it shares with the others, and the tail's numbering
/// starts above the largest stored ordinal. A per-card function could not see
/// that. `effective_order` already computes exactly this ordering for the drag
/// route, so the list the human READS and the arithmetic a drop is computed
/// against are the same function — which is the property that broke when the
/// browser had its own copy of the rule.
///
/// Only `Included` refs take part: an undecided or dropped candidate has no
/// position in a list it does not appear in, and its `display_ordinal` stays
/// `None`.
pub(crate) fn apply_display_order(
    states: &mut std::collections::HashMap<String, CardRefState>,
    ordinals: &std::collections::HashMap<String, i32>,
) {
    let facts: Vec<OrderableFact> = states
        .iter()
        .filter(|(_, st)| st.status == Some(FactStatus::Included))
        .map(|(node, st)| OrderableFact {
            graph_node_id: node.clone(),
            sort_ordinal: st.sort_ordinal,
            code_ordinal: ordinals.get(node).copied(),
            tier: st.tier.unwrap_or(FactTier::DEFAULT),
        })
        .collect();

    for (node, position) in effective_order(&facts) {
        if let Some(state) = states.get_mut(&node) {
            state.display_ordinal = Some(position);
        }
    }
}

/// Decode each fact-ref row into the card's view of it.
///
/// ## The status decode is a loud boundary (Standing Rule 1)
///
/// Same discipline as `scenario_gather::build_ref_index`: a status token this
/// build does not understand is a data-integrity fault, logged with its ids and
/// surfaced, never collapsed to `Undecided`. Silently bucketing it would show the
/// human a card labelled "Not yet decided" for an item somebody had already ruled
/// on.
pub(crate) fn build_ref_states(
    refs: Vec<crate::repositories::pipeline_repository::ScenarioFactRefRecord>,
) -> Result<HashMap<String, CardRefState>, AppError> {
    let mut states = HashMap::new();
    for r in refs {
        let status = FactStatus::try_from(r.status.as_str()).map_err(|e| {
            tracing::error!(
                error = %e,
                graph_node_id = %r.graph_node_id,
                scenario_id = %r.scenario_id,
                "scenario_fact_refs carries a status token this build does not define"
            );
            AppError::Internal {
                message: "failed to read candidate state".to_string(),
            }
        })?;
        // Task 2.13: the tier decode is the SAME loud boundary as the status
        // decode above, for the same reason. A token this build does not define
        // must not collapse to `backup` — that would show the human "Backup" for a
        // fact they had deliberately marked as carrying the scenario, and the
        // screen would be confidently wrong with nothing in the log.
        let tier = FactTier::try_from(r.tier.as_str()).map_err(|e| {
            tracing::error!(
                error = %e,
                graph_node_id = %r.graph_node_id,
                scenario_id = %r.scenario_id,
                "scenario_fact_refs carries a tier token this build does not define"
            );
            AppError::Internal {
                message: "failed to read candidate state".to_string(),
            }
        })?;
        states.insert(
            r.graph_node_id,
            CardRefState {
                status: Some(status),
                confidence: r.confidence,
                // A card with a reference row is never a proposal: R-a says a row
                // always wins, and `project` has already dropped this node from the
                // projection. `None` here is that law restated where the state is
                // built, not a placeholder waiting to be filled.
                proposal: None,
                defer_reason: r.defer_reason,
                tier: Some(tier),
                sort_ordinal: r.sort_ordinal,
                // Filled in by `apply_display_order` once every ref is decoded —
                // a card's position depends on the whole set, so it cannot be
                // known while the set is still being built.
                display_ordinal: None,
            },
        );
    }
    Ok(states)
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
