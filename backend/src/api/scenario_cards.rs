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
    domain::fact_status::FactStatus,
    dto::scenario_card::ScenarioCardsResponse,
    error::AppError,
    repositories::{
        pipeline_repository::{
            get_document_text, list_candidate_ordinals, list_fact_refs_for_scenario,
        },
        scenario_card_repository::fetch_card_extras,
    },
    services::scenario_card::{collapse_extras, CardRefState},
    services::scenario_card_assembly::{assemble, page_key},
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

    let page_text = load_page_text(&state, &pool).await;

    let response = assemble(pool, &extras, &ref_states, &ordinals, &page_text);

    tracing::info!(
        pool = response.pool.len(),
        set_aside = response.set_aside.len(),
        defer_required = response.pool.iter().filter(|c| c.defer_required).count(),
        "served scenario cards"
    );

    Ok(Json(response))
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
        states.insert(
            r.graph_node_id,
            CardRefState {
                status: Some(status),
                confidence: r.confidence,
                defer_reason: r.defer_reason,
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
/// context is visible on screen. (A quote the page text does not contain is the
/// third shape, and it has its own signal — `grounding_status: not_found` on the
/// same card.)
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
