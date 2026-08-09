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

use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    Json,
};

use crate::{
    auth::AuthUser,
    bias::{dto::BiasInstance, repository::BiasRepository},
    dto::scenario_card::{ProposalSource, ScenarioCardsResponse},
    error::AppError,
    repositories::{
        allegation_options_repository::fetch_allegation_options,
        pipeline_repository::{
            get_document_text, list_candidate_ordinals, list_fact_refs_for_scenario,
            list_links_for_nodes, list_ruled_card_reasons, list_summary_overrides,
        },
        scenario_card_repository::fetch_card_extras,
    },
    services::scenario_card::collapse_extras,
    services::scenario_card_assembly::{
        assemble, attach_ruled_reasons, build_ref_states, count_proposed, page_key,
        ruled_reason_keys, HumanTouchIndex, PoolIndexes, ProposalIndex,
    },
    services::scenario_card_projection::index_by_covered_node,
    services::scenario_cards_scan_state::never_scanned_notice,
    services::scenario_human_links::resolve_links,
    services::scenario_link_options::label_index,
    services::scenario_proposal_lookup::{load_projecting_run, project_run},
    state::AppState,
};

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

    let pool = BiasRepository::new(state.graph.clone())
        .all_evidence_about_subject(&subject_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, subject_id = %subject_id,
                "failed to read candidate pool for cards");
            AppError::Internal {
                message: "failed to read candidate pool".to_string(),
            }
        })?;

    let node_ids: Vec<String> = pool.iter().map(|c| c.evidence_id.clone()).collect();

    let extras = fetch_card_extras(&state.graph, &node_ids)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, scenario_id = %id, "failed to read card extras from graph");
            AppError::Internal {
                message: "failed to read candidate details".to_string(),
            }
        })?;
    let extras = collapse_extras(extras);

    let refs = list_fact_refs_for_scenario(&state.pipeline_pool, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, scenario_id = %id, "failed to list scenario fact refs for cards");
            AppError::Internal {
                message: "failed to list scenario fact refs".to_string(),
            }
        })?;
    let ordinals = list_candidate_ordinals(&state.pipeline_pool, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, scenario_id = %id, "failed to read candidate ordinals for cards");
            AppError::Internal {
                message: "failed to read candidate identifiers".to_string(),
            }
        })?;

    // Ruling R3: which (run, node) pairs a ruled card could carry a reason for.
    // Taken BEFORE `build_ref_states` consumes the rows — the decoder does not
    // keep `source_run_id`, and re-reading the refs to get it back would be a
    // second query for a column already in hand.
    let (reason_runs, reason_nodes) = ruled_reason_keys(&refs);

    let mut ref_states = build_ref_states(refs)?;

    // The scan's reason for the cards a human has already ruled.
    //
    // ## Why a failure here degrades and does not propagate
    //
    // The same test `load_page_text` applies: a card without its reason still
    // carries the quote, the pinpoint, the chips and every control — it is
    // poorer, not wrong. Failing the whole request would hide forty-six working
    // facts to protect one missing sentence. The absence stays observable: the
    // `warn` names the scenario and the count, and the card simply shows no
    // reason, which is the same shape as a card nothing ever judged.
    if !reason_runs.is_empty() {
        match list_ruled_card_reasons(&state.pipeline_pool, &reason_runs, &reason_nodes).await {
            Ok(rows) => attach_ruled_reasons(&mut ref_states, rows),
            Err(e) => tracing::warn!(
                error = %e,
                scenario_id = %id,
                cited_runs = reason_runs.len(),
                "failed to read the judging reasons behind the already-ruled cards;                  those cards will render without the scan's reason"
            ),
        }
    }
    // Task 2.13c: the list's ORDER is decided here, once, and travels as a single
    // number per card. Doing it server-side is what stopped the browser and the
    // backend disagreeing about where a newly included fact belongs — they did,
    // and a re-included card came back fourth instead of last.
    crate::services::scenario_card_assembly::apply_display_order(&mut ref_states, &ordinals);

    // Task 1.7F Part B: the humans' corrections for this pool, in ONE query.
    // Read here beside the other joins rather than inside the assembler, which is
    // documented pure and must stay that way.
    let overrides = list_summary_overrides(&state.pipeline_pool, &node_ids)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, scenario_id = %id, "failed to read evidence summary overrides for cards");
            AppError::Internal {
                message: "failed to read the corrected questions".to_string(),
            }
        })?;
    let overrides: HashMap<String, _> = overrides
        .into_iter()
        .map(|row| (row.graph_node_id.clone(), row))
        .collect();

    let page_text = load_page_text(&state, &pool).await;

    // One snapshot for the whole payload: read once here so every card is banded
    // by the same cutoffs, and so the pure assembler stays pure (v2 §2b).
    let settings = state.settings.current();

    let human_links = load_human_links(&state, &node_ids, &subject_id, &settings).await?;

    // The projection (2026-08-08): what the latest COMPLETED run proposes to this
    // queue right now. Read before assembly because a proposal is part of a card's
    // state, not an annotation layered on afterwards.
    let projecting_run = load_projecting_run(&state, id).await?;
    let groups = match &projecting_run {
        None => Vec::new(),
        Some(run) => project_run(&state, run, &pool, &ref_states, &ordinals).await?,
    };
    let proposals: ProposalIndex<'_> = index_by_covered_node(&groups)
        .into_iter()
        // Only the REPRESENTATIVE carries the card. Its covered twins stay ordinary
        // unruled pool rows — the ruling settles them, so proposing them a second
        // time would ask the human to decide the same sentence twice, which is the
        // duplicate work the fold exists to end.
        .filter(|(node, group)| *node == group.representative)
        .collect();

    let mut response = assemble(
        pool,
        &extras,
        &ref_states,
        &ordinals,
        &page_text,
        &settings,
        PoolIndexes {
            human: HumanTouchIndex {
                question_overrides: &overrides,
                links: &human_links,
            },
            proposals: &proposals,
        },
    );

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

/// The accusations humans have linked, display-ready, by node (task 2.10).
///
/// ## Why the graph read is conditional and the Postgres read is not
///
/// The link rows are one indexed `= ANY($1)` and answer the question "is anything
/// linked?", so they are always read. LABELLING those links needs the accusation
/// catalogue — 120 rows on DEV — and a pool where nobody has linked anything has
/// nothing to label, which is every pool on the day this ships. So the catalogue
/// is fetched only when there is at least one link to spell out. That keeps the
/// card path exactly as expensive as it was until the feature is used.
///
/// ## Why a failure here is fatal to the request, unlike page text
///
/// Quote-in-context degrades: a card without it still carries everything needed to
/// rule. A card silently missing its human links does not degrade — it reports
/// itself DEFER-ONLY, which is a false statement about what the human may do, and
/// it would be indistinguishable from a card nobody has linked. So this
/// propagates (Standing Rule 1: an operationally distinct state gets a distinct
/// observable, and "the link read failed" must never look like "there are no
/// links").
async fn load_human_links(
    state: &AppState,
    node_ids: &[String],
    subject_id: &str,
    settings: &crate::domain::settings::Settings,
) -> Result<HashMap<String, Vec<crate::dto::scenario_card::CardHumanLink>>, AppError> {
    let rows = list_links_for_nodes(&state.pipeline_pool, node_ids)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to read the human accusation links for cards");
            AppError::Internal {
                message: "failed to read the accusation links".to_string(),
            }
        })?;

    if rows.is_empty() {
        return Ok(HashMap::new());
    }

    let options = fetch_allegation_options(&state.graph, subject_id)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                %subject_id,
                "failed to read the accusation catalogue needed to label human links"
            );
            AppError::Internal {
                message: "failed to read the accusations".to_string(),
            }
        })?;

    Ok(resolve_links(
        rows,
        &label_index(&options),
        &settings.wording,
    ))
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

/// Read the page text backing every candidate's quote, keyed `doc_id:page`.
///
/// ## Why a failure here is a degraded card, not a failed request
///
/// Quote-in-context is the one §7 element with a soft failure mode: without it
/// the card still carries the quote, the pinpoint and the viewer link, so the
/// human can read the passage in the PDF. Failing the whole request because one
/// document's text is unavailable would hide every OTHER card too — a worse
/// outcome than a card with empty context, and the reason this read is the only
/// one that logs-and-continues rather than propagating.
///
/// The absence stays observable in BOTH shapes: a `warn` names the document
/// whether the read failed or simply returned no pages, and the card's empty
/// context is visible on screen.
///
/// ## The third shape, corrected in task 1.7A
///
/// A quote the loaded page text does not contain used to be described here as
/// carrying "its own signal — `grounding_status: not_found`". That was only ever
/// true of one case. A quote that spans a page BOUNDARY grounds perfectly well —
/// the verifier matches across an adjacent page pair and records the LEFT page —
/// but this function loads that one page, so the words are not all there and the
/// card renders bare while reporting itself grounded. Nothing on the card says
/// which of the two happened.
///
/// That shape is now counted rather than inferred: `cards_without_context`
/// reports it on the "served scenario cards" line, so a rise in bare cards is
/// visible without anyone having to notice it on screen first.
async fn load_page_text(state: &AppState, pool: &[BiasInstance]) -> HashMap<String, String> {
    // One read per distinct document, not per candidate: a deposition contributes
    // dozens of candidates from the same file.
    let mut document_ids: Vec<String> = pool
        .iter()
        .filter_map(|c| c.document.as_ref().map(|d| d.id.clone()))
        .collect();
    document_ids.sort();
    document_ids.dedup();

    let mut by_key = HashMap::new();
    for document_id in document_ids {
        match get_document_text(&state.pipeline_pool, &document_id).await {
            Ok(pages) => {
                // An EMPTY result is a different fact from a failed read, and it
                // was silent until 2026-08-01: a document that was never OCR'd into
                // `document_text` produced context-less cards with nothing in the
                // log to say why. Both cases now name the document.
                if pages.is_empty() {
                    tracing::warn!(
                        %document_id,
                        "no page text is stored for this document; its cards will \
                         carry the quote without surrounding context"
                    );
                }
                for page in pages {
                    by_key.insert(
                        page_key(&document_id, i64::from(page.page_number)),
                        page.text_content,
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    %document_id,
                    "no stored page text for this document; its cards will carry the \
                     quote without surrounding context"
                );
            }
        }
    }
    by_key
}

#[cfg(test)]
#[path = "scenario_cards_tests.rs"]
mod tests;
