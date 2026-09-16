//! The candidate card endpoint (task 1.2, v2 §7).
//!
//! `GET /cases/:slug/scenarios/:scenario_id/facts/cards` → the complete card
//! payload for every candidate in the scenario's pool.
//!
//! ## Why a new route beside `…/facts/gather`
//!
//! Gather serves the raw candidate shape the current workbench renders. This
//! serves the §7 card contract: everything display-ready, in plain trial
//! language, with the unrulable items flagged. Task 1.3 switches the UI over;
//! until it does, both exist and the shipped screen is untouched — which is what
//! keeps "no frontend file in this task" honest rather than merely stated.
//!
//! ## The four reads
//!
//! The graph pool, the card extras (also graph), the persisted fact-refs, and the
//! page text for quote-in-context. Everything after them is the pure assembly in
//! `services::scenario_card`.
//!
//! ## CRITICAL — the pipeline pool
//!
//! `scenario_fact_refs` and `document_text` live in the **pipeline** database
//! (`colossus_legal_v2`), so those reads use `&state.pipeline_pool`, NOT
//! `state.pg_pool`.

use axum::{
    extract::{Path, State},
    Json,
};

use crate::{
    auth::AuthUser,
    dto::scenario_card::{ProposalSource, ScenarioCardsResponse},
    error::AppError,
    services::scenario_card_assembly::count_proposed,
    services::scenario_cards_scan_state::never_scanned_notice,
    state::AppState,
};

use super::scenario_card_fact_cards::attach_scenario_fact_cards;
use super::scenario_cards_core::{assemble_cards, read_candidate_pool, CardDetail, CardsCore};
use super::scenario_facts::{ensure_scenario_in_case, parse_scenario_id};
use super::scenario_gather::resolve_gather_subject;

/// `GET /cases/:slug/scenarios/:scenario_id/facts/cards` — the §7 card payload.
///
/// Open read (`Option<AuthUser>`), matching the sibling gather and facts-list
/// routes: reading a scenario's candidates is not an edit.
///
/// ## Why this route does NOT assign ordinals
///
/// Gather memoizes candidate ordinals on read — the one sanctioned write on a
/// read path, because identity must exist the moment a candidate appears. This
/// route deliberately does not repeat that: it READS whatever ordinals exist and
/// serves `code: null` for a candidate gather has not numbered yet. Two endpoints
/// racing to mint the same ordinal is a unique-violation waiting to happen, and
/// duplicating the write would put the "one place assigns identity" rule at the
/// mercy of whichever endpoint the UI happened to call first.
#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn get_scenario_cards(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
) -> Result<Json<ScenarioCardsResponse>, AppError> {
    if let Some(ref u) = user {
        tracing::info!(
            "{} GET /cases/{}/scenarios/{}/facts/cards",
            u.username,
            slug,
            scenario_id
        );
    }

    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    // No target → no queue. The same early return the gather route makes, for the
    // same reason: this endpoint fed the 148 borrowed cards Roman saw on
    // 2026-08-07, so an empty payload carrying its own explanation is the fix's
    // visible half. Every read below (graph pool, extras, refs, ordinals,
    // overrides, page text) is skipped — there is nothing to read them ABOUT.
    let Some(subject_id) = resolve_gather_subject(&state, id).await? else {
        return Ok(Json(no_target_response(
            &state
                .settings
                .current()
                .scenario_authoring_wording
                .no_target_notice,
        )));
    };

    // One snapshot for the whole payload: the cards and the fact-card sentences
    // laid over them below are banded and worded from the same read (v2 §2b).
    let settings = state.settings.current();
    let pool = read_candidate_pool(&state, &subject_id).await?;
    let CardsCore {
        mut response,
        projecting_run,
    } = assemble_cards(&state, id, &subject_id, pool, &settings, CardDetail::Full).await?;

    // Attach the run's identity to the count the assembly produced.
    //
    // The `filter` is what keeps ONE meaning for this field's presence: "there are
    // proposals below". A completed run whose every admitted verdict has already
    // been ruled is a FINISHED queue, and a source object carrying a count of zero
    // would make the page lead with a proposal heading over nothing.
    let proposed_count = count_proposed(&response);
    response.proposal_source =
        projecting_run
            .filter(|_| proposed_count > 0)
            .map(|run| ProposalSource {
                run_id: run.run_id,
                model_id: run.model_id,
                started_at: run.started_at,
                proposed_count,
            });

    // FACT_CARD_v2 §2: the five sentences a witness reads, laid over the cards
    // the assembler just built. After assembly rather than inside it — the stored
    // cards are a SCENARIO-wide read and `assemble` is per-candidate.
    attach_scenario_fact_cards(&state, id, &subject_id, &settings, &mut response).await?;

    // Task 2.15 piece 3: the cards are still served in full — this only says
    // whether anything has ever JUDGED them, which is what the page needs before
    // it may call them candidates from a scan.
    response.never_scanned_notice = never_scanned_notice(&state, id).await?;

    tracing::info!(
        pool = response.pool.len(),
        set_aside = response.set_aside.len(),
        defer_required = response.pool.iter().filter(|c| c.defer_required).count(),
        // How much of the stuck pile humans have cleared, on the same line as the
        // refusal count it is the cure for (task 2.10).
        human_linked = response
            .pool
            .iter()
            .filter(|c| !c.human_links.is_empty())
            .count(),
        without_context = cards_without_context(&response),
        "served scenario cards"
    );

    Ok(Json(response))
}

/// The payload for a scenario that names no target: no cards, and the stored
/// sentence explaining that this is a definition the human has not finished, not
/// a case with no evidence.
///
/// ## Why `link_progress` stays `None` here
///
/// That line counts how much of a stuck pile has been cleared. There is no pile
/// — not an empty one, but no pile at all, because nothing was gathered. "0 of 0
/// linked" would be a true sentence about a question nobody asked, sitting under
/// a notice that says the queue does not exist yet.
/// ## Rust Learning: why this takes `&str` and not `&AppState`
///
/// The notice comes from the settings snapshot, so `fn(&AppState)` looks
/// natural — and it is untestable, because a function taking `&AppState` needs a
/// database, a graph and a settings store to call at all. Taking the
/// already-read string makes this a pure function of one `&str`: the caller
/// does the reading (it holds the state), this does the shaping, and a unit test
/// can ask in one line whether an un-targeted scenario carries its explanation.
/// Same seam as `scenario_gather::no_target_response`.
fn no_target_response(notice: &str) -> ScenarioCardsResponse {
    ScenarioCardsResponse {
        pool: Vec::new(),
        set_aside: Vec::new(),
        link_progress: None,
        no_target_notice: Some(notice.to_string()),
        // A scenario with no target has no pool to describe, scanned or not. The
        // no-target notice is the whole explanation this payload owes; adding a
        // second one would stack two answers to two questions the human has not
        // reached yet.
        never_scanned_notice: None,
        // Nothing was gathered, so nothing can be proposed about it — and a scan
        // cannot have run on a scenario that names no target.
        proposal_source: None,
    }
}

/// How many served cards carry a quote but no surrounding context (§7.1).
///
/// ## Why this is counted rather than logged per card
///
/// After task 1.7A a context-less card is RARE — the assembler now finds a quote
/// whose wording only matches after normalization, which was 320 of 320 such
/// items before. What remains is the residue: a quote that spans a page boundary
/// (grounding records the LEFT page of the pair, and only that page's text is
/// loaded), an OCR transposition, or a document with no stored text at all.
///
/// Those cards look fully grounded — `grounding_status` reads `exact` or
/// `normalized` — while showing nothing around the quote, so without this number
/// an operator asking "why is this grounded card bare?" has nothing to consult.
/// One count per request rather than one line per card: a 150-card payload would
/// otherwise emit 150 lines to say something about a handful of them, and the
/// number is what tells you whether this is a stray or a systemic regression.
///
/// Pure, and outside `build_card` on purpose: the assembler is documented pure
/// and its output is what the §7 completeness test asserts against, so the
/// counting happens HERE, over the finished payload, rather than as a side effect
/// inside it.
fn cards_without_context(response: &ScenarioCardsResponse) -> usize {
    response
        .pool
        .iter()
        .chain(response.set_aside.iter())
        .filter(|card| {
            !card.quote.text.trim().is_empty()
                && card.quote.context_before.is_empty()
                && card.quote.context_after.is_empty()
        })
        .count()
}

#[cfg(test)]
#[path = "scenario_cards_tests.rs"]
mod tests;
