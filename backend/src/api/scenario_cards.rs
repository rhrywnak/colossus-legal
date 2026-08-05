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
    domain::{fact_status::FactStatus, fact_tier::FactTier},
    dto::scenario_card::ScenarioCardsResponse,
    error::AppError,
    repositories::{
        allegation_options_repository::fetch_allegation_options,
        pipeline_repository::{
            get_document_text, list_candidate_ordinals, list_fact_refs_for_scenario,
            list_links_for_nodes, list_summary_overrides,
        },
        scenario_card_repository::fetch_card_extras,
    },
    services::scenario_card::{collapse_extras, CardRefState},
    services::scenario_card_assembly::{assemble, page_key, HumanTouchIndex},
    services::scenario_human_links::resolve_links,
    services::scenario_link_options::label_index,
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
    let subject_id = resolve_gather_subject(&state, id).await?;

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
    let ref_states = build_ref_states(refs)?;

    let ordinals = list_candidate_ordinals(&state.pipeline_pool, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, scenario_id = %id, "failed to read candidate ordinals for cards");
            AppError::Internal {
                message: "failed to read candidate identifiers".to_string(),
            }
        })?;

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

    let response = assemble(
        pool,
        &extras,
        &ref_states,
        &ordinals,
        &page_text,
        &settings,
        HumanTouchIndex {
            question_overrides: &overrides,
            links: &human_links,
        },
    );

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

/// Decode each fact-ref row into the card's view of it.
///
/// ## The status decode is a loud boundary (Standing Rule 1)
///
/// Same discipline as `scenario_gather::build_ref_index`: a status token this
/// build does not understand is a data-integrity fault, logged with its ids and
/// surfaced, never collapsed to `Undecided`. Silently bucketing it would show the
/// human a card labelled "Not yet decided" for an item somebody had already ruled
/// on.
fn build_ref_states(
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
                defer_reason: r.defer_reason,
                tier: Some(tier),
                sort_ordinal: r.sort_ordinal,
            },
        );
    }
    Ok(states)
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
