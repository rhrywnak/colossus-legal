//! The candidate-card READ, shared by the card queue and the War Room.
//!
//! Extracted from `scenario_cards::get_scenario_cards` for CC_TASK_WAR_ROOM_v1.
//! The War Room's status card shows two numbers — candidates to rule and Matrix
//! linked — that are stored nowhere. They are what this read produces: the graph
//! pool around the scenario's subject, the ruled refs, the stance each card's
//! graph links give it, the human links, and the latest scan PROJECTED onto the
//! pool with precedence R-a and the representative fold.
//!
//! ## Why the dashboard reuses this instead of counting in SQL
//!
//! Re-deriving stance, the projection and the fold in a batched query would be a
//! second authority for "4 of 99", and the first time either copy changed the
//! War Room and the scenario page would disagree about the same scenario (ruling
//! Q1, Option A). So there is ONE assembly, and the two callers differ only in
//! what they skip.
//!
//! ## `CardDetail` — what the War Room does not need
//!
//! Page text (quote-in-context), the humans' question overrides and the scan's
//! reasons on already-ruled cards decorate a card; none of them moves a card's
//! status, its proposal, its stance or its human links. `CountsOnly` skips those
//! three reads. The fact-card hydration and the never-scanned notice happen in the
//! route after this returns, so the War Room never reaches them at all.

use std::collections::HashMap;

use uuid::Uuid;

use crate::{
    bias::{dto::BiasInstance, repository::BiasRepository},
    domain::settings::Settings,
    dto::scenario_card::ScenarioCardsResponse,
    error::AppError,
    repositories::{
        allegation_options_repository::fetch_allegation_options,
        pipeline_repository::{
            get_document_text, list_candidate_ordinals, list_fact_refs_for_scenario,
            list_links_for_nodes, list_ruled_card_reasons, list_summary_overrides,
            ProjectingRunRow,
        },
        scenario_card_repository::fetch_card_extras,
    },
    services::scenario_card::collapse_extras,
    services::scenario_card_assembly::{
        apply_display_order, assemble, attach_ruled_reasons, build_ref_states, page_key,
        ruled_reason_keys, HumanTouchIndex, PoolIndexes, ProposalIndex,
    },
    services::scenario_card_projection::{index_by_covered_node, ProposalGroup},
    services::scenario_human_links::resolve_links,
    services::scenario_link_options::label_index,
    services::scenario_proposal_lookup::{load_projecting_run, project_run},
    state::AppState,
};

use super::scenario_cards_hydrate::append_refs_outside_pool;

/// How much of a card the caller needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CardDetail {
    /// Everything the card queue renders.
    Full,
    /// Only what decides a card's status, proposal, stance and human links —
    /// the War Room's counts. Skips page text, overrides and ruled reasons.
    CountsOnly,
}

/// The assembled cards, and the run their proposals came from.
///
/// The run travels out because the card route attaches its identity to the
/// payload (`proposal_source`) after assembly; returning it saves reading it twice.
pub(crate) struct CardsCore {
    pub response: ScenarioCardsResponse,
    pub projecting_run: Option<ProjectingRunRow>,
}

/// The graph gather: every evidence node about one subject.
///
/// Split out so a caller holding several scenarios about the SAME subject can
/// read it once and hand each scenario a copy (ruling Q1, condition 1). The
/// eleven scenarios on PROD have two subjects between them.
///
/// # Errors
/// A logged 500 when the graph read fails.
pub(crate) async fn read_candidate_pool(
    state: &AppState,
    subject_id: &str,
) -> Result<Vec<BiasInstance>, AppError> {
    BiasRepository::new(state.graph.clone())
        .all_evidence_about_subject(subject_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, subject_id = %subject_id,
                "failed to read candidate pool for cards");
            AppError::Internal {
                message: "failed to read candidate pool".to_string(),
            }
        })
}

/// Assemble one scenario's cards from a pool already gathered for its subject.
///
/// `settings` is the caller's ONE snapshot, so a route that words anything else
/// from the store after assembly uses the same read the cards were banded by.
///
/// `pool` is taken by value because the scenario's ruled refs that the gather no
/// longer reaches are appended to it — each scenario needs its own copy.
///
/// # Errors
/// Every store and graph read here propagates as a logged 500, except the ruled
/// reasons and page text, which degrade exactly as they always have.
pub(crate) async fn assemble_cards(
    state: &AppState,
    id: Uuid,
    subject_id: &str,
    mut pool: Vec<BiasInstance>,
    settings: &Settings,
    detail: CardDetail,
) -> Result<CardsCore, AppError> {
    // The refs are read BEFORE `node_ids` is computed, because a ruled fact whose
    // node the gather no longer reaches has to join the pool before anything else
    // measures it. See `append_refs_outside_pool` for why that matters.
    let refs = read_refs(state, id).await?;
    append_refs_outside_pool(state.graph.clone(), id, &refs, &mut pool).await?;

    let node_ids: Vec<String> = pool.iter().map(|c| c.evidence_id.clone()).collect();
    // NOT skipped for the War Room: a card's STANCE comes from these graph links,
    // and "Matrix linked" counts the cards that have none.
    let extras = read_extras(state, id, &node_ids).await?;
    let ordinals = read_ordinals(state, id).await?;

    // Ruling R3: which (run, node) pairs a ruled card could carry a reason for.
    // Taken BEFORE `build_ref_states` consumes the rows.
    let (reason_runs, reason_nodes) = ruled_reason_keys(&refs);
    let mut ref_states = build_ref_states(refs)?;
    if detail == CardDetail::Full {
        attach_reasons(state, id, &mut ref_states, &reason_runs, &reason_nodes).await;
    }
    // Task 2.13c: the list's ORDER is decided here, once.
    apply_display_order(&mut ref_states, &ordinals);

    let (overrides, page_text) = read_decorations(state, id, &node_ids, &pool, detail).await?;

    let human_links = load_human_links(state, &node_ids, subject_id, settings).await?;

    // The projection (2026-08-08): what the latest COMPLETED run proposes now.
    let (projecting_run, groups) =
        read_projection(state, id, &pool, &ref_states, &ordinals).await?;
    let proposals: ProposalIndex<'_> = index_by_covered_node(&groups)
        .into_iter()
        // Only the REPRESENTATIVE carries the card; its covered twins stay
        // ordinary unruled rows, so nobody decides the same sentence twice.
        .filter(|(node, group)| *node == group.representative)
        .collect();

    let response = assemble(
        pool,
        &extras,
        &ref_states,
        &ordinals,
        &page_text,
        settings,
        PoolIndexes {
            human: HumanTouchIndex {
                question_overrides: &overrides,
                links: &human_links,
            },
            proposals: &proposals,
        },
    );
    Ok(CardsCore {
        response,
        projecting_run,
    })
}

/// This scenario's persisted include/drop/undecided rulings.
async fn read_refs(
    state: &AppState,
    id: Uuid,
) -> Result<Vec<crate::repositories::pipeline_repository::ScenarioFactRefRecord>, AppError> {
    list_fact_refs_for_scenario(&state.pipeline_pool, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, scenario_id = %id, "failed to list scenario fact refs for cards");
            AppError::Internal {
                message: "failed to list scenario fact refs".to_string(),
            }
        })
}

/// The card decorations the War Room does not need: overrides and page text.
///
/// ## Rust Learning: `match` as an expression
///
/// Both arms produce the same tuple type, so the `match` itself is the value
/// returned. The empty maps on the counts-only arm are not a fake read: the
/// assembler treats "no override" and "no page text" as ordinary per-card
/// states, and neither feeds a count.
async fn read_decorations(
    state: &AppState,
    id: Uuid,
    node_ids: &[String],
    pool: &[BiasInstance],
    detail: CardDetail,
) -> Result<
    (
        HashMap<String, crate::repositories::pipeline_repository::EvidenceSummaryOverrideRecord>,
        HashMap<String, String>,
    ),
    AppError,
> {
    Ok(match detail {
        CardDetail::Full => (
            read_overrides(state, id, node_ids).await?,
            load_page_text(state, pool).await,
        ),
        CardDetail::CountsOnly => (HashMap::new(), HashMap::new()),
    })
}

/// The latest completed run, and its verdicts projected onto this pool.
async fn read_projection(
    state: &AppState,
    id: Uuid,
    pool: &[BiasInstance],
    ref_states: &HashMap<String, crate::services::scenario_card::CardRefState>,
    ordinals: &HashMap<String, i32>,
) -> Result<(Option<ProjectingRunRow>, Vec<ProposalGroup>), AppError> {
    let projecting_run = load_projecting_run(state, id).await?;
    let groups = match &projecting_run {
        None => Vec::new(),
        Some(run) => project_run(state, run, pool, ref_states, ordinals).await?,
    };
    Ok((projecting_run, groups))
}

/// The graph extras (links, speaker labels) for every card, collapsed per node.
async fn read_extras(
    state: &AppState,
    id: Uuid,
    node_ids: &[String],
) -> Result<HashMap<String, crate::services::scenario_card::CollapsedExtras>, AppError> {
    let extras = fetch_card_extras(&state.graph, node_ids)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, scenario_id = %id, "failed to read card extras from graph");
            AppError::Internal {
                message: "failed to read candidate details".to_string(),
            }
        })?;
    Ok(collapse_extras(extras))
}

/// The scenario's candidate ordinals (`C-nnn`), read and never minted here.
async fn read_ordinals(state: &AppState, id: Uuid) -> Result<HashMap<String, i32>, AppError> {
    list_candidate_ordinals(&state.pipeline_pool, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, scenario_id = %id, "failed to read candidate ordinals for cards");
            AppError::Internal {
                message: "failed to read candidate identifiers".to_string(),
            }
        })
}

/// The scan's reason for the cards a human has already ruled.
///
/// ## Why a failure here degrades and does not propagate
///
/// A card without its reason still carries the quote, the pinpoint, the chips and
/// every control — it is poorer, not wrong. The absence stays observable: the
/// `warn` names the scenario and the count.
async fn attach_reasons(
    state: &AppState,
    id: Uuid,
    ref_states: &mut HashMap<String, crate::services::scenario_card::CardRefState>,
    reason_runs: &[Uuid],
    reason_nodes: &[String],
) {
    if reason_runs.is_empty() {
        return;
    }
    match list_ruled_card_reasons(&state.pipeline_pool, reason_runs, reason_nodes).await {
        Ok(rows) => attach_ruled_reasons(ref_states, rows),
        Err(e) => tracing::warn!(
            error = %e,
            scenario_id = %id,
            cited_runs = reason_runs.len(),
            "failed to read the judging reasons behind the already-ruled \
             cards; those cards render without the scan's reason. The rest of \
             the payload is unaffected — if this recurs, check the pipeline \
             database (scan_run_verdicts) rather than the graph"
        ),
    }
}

/// Task 1.7F Part B: the humans' corrected questions for this pool, in ONE query.
async fn read_overrides(
    state: &AppState,
    id: Uuid,
    node_ids: &[String],
) -> Result<
    HashMap<String, crate::repositories::pipeline_repository::EvidenceSummaryOverrideRecord>,
    AppError,
> {
    let overrides = list_summary_overrides(&state.pipeline_pool, node_ids)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, scenario_id = %id, "failed to read evidence summary overrides for cards");
            AppError::Internal {
                message: "failed to read the corrected questions".to_string(),
            }
        })?;
    Ok(overrides
        .into_iter()
        .map(|row| (row.graph_node_id.clone(), row))
        .collect())
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
pub(crate) async fn load_human_links(
    state: &AppState,
    node_ids: &[String],
    subject_id: &str,
    settings: &crate::domain::settings::Settings,
) -> Result<HashMap<String, Vec<crate::dto::scenario_card::CardHumanLink>>, AppError> {
    let rows = list_links_for_nodes(&state.pipeline_pool, node_ids)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, %subject_id, "failed to read the human accusation links for cards");
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
