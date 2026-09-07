//! Linking a statement to the accusations it bears on (task 2.10).
//!
//! - `GET  …/scenarios/:scenario_id/allegation-options` → the panel's words and
//!   the accusations it offers, short list first.
//! - `POST …/evidence/:graph_node_id/links` → link one or more, with one cut.
//! - `DELETE …/evidence/:graph_node_id/links/:allegation_id` → take one back.
//!
//! ## Domain note: the WRITES have no scenario in the path, deliberately
//!
//! A statement that bears on ¶41 bears on ¶41 in every scenario, exactly as the
//! machine's own graph edges do — the same ruling that made the summary override
//! case-wide (1.7F, R1). The READ is scenario-scoped because the ORDER is: the
//! short list is "the accusations this scenario already serves", which is a fact
//! about the scenario even though the links it produces are not.
//!
//! ## Why the writes return no composed sentence
//!
//! The card's summary is built from the whole set of links plus the stored
//! template, and the client re-reads the pool to get it — which is the same read
//! that unlocks Include and Exclude, so it has to happen anyway. Composing a
//! second copy here would be two places that can disagree about one sentence.
//!
//! ## CRITICAL — the pipeline pool
//!
//! `evidence_allegation_links` lives in `colossus_legal_v2`: `&state.pipeline_pool`.

use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Utc;

use crate::{
    auth::{require_edit, AuthUser},
    dto::evidence_links::{
        AllegationOptionsResponse, SaveLinksRequest, SaveLinksResponse, UnlinkResponse,
    },
    error::AppError,
    repositories::{
        allegation_options_repository::fetch_allegation_options,
        pipeline_repository::{
            delete_link,
            evidence_allegation_rulings::{save_ruling, RulingWrite},
            get_scenario, save_link, LinkWrite,
        },
    },
    services::scenario_link_options::build_options,
    state::AppState,
};

use super::scenario_facts::{ensure_scenario_in_case, parse_scenario_id};
use super::scenario_gather::resolve_gather_subject;

/// The route group for accusation linking.
///
/// The two write paths sit under `/cases/:slug/evidence/:graph_node_id/`, beside
/// the `summary` route 1.7F added — `links` is another static child of the same
/// param, which matchit 0.7.3 accepts for the same reason `gather` and `cards`
/// coexist under `:graph_node_id` on the facts routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/cases/:slug/scenarios/:scenario_id/allegation-options",
            get(list_allegation_options),
        )
        .route(
            "/cases/:slug/evidence/:graph_node_id/links",
            post(save_links),
        )
        .route(
            "/cases/:slug/evidence/:graph_node_id/links/:allegation_id",
            delete(remove_link),
        )
}

/// `GET …/allegation-options` — every accusation, short list first.
///
/// Open read (`Option<AuthUser>`), matching the sibling card and gather routes:
/// reading what a case accuses somebody of is not an edit.
///
/// ## Why this is not folded into the cards payload
///
/// That payload is re-read after every single ruling. The catalogue is 120 rows
/// on DEV and changes only when a document is processed, so riding along would
/// mean re-sending it on every keystroke of a triage session to say the same
/// thing. The client reads this once per scenario.
#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn list_allegation_options(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
) -> Result<Json<AllegationOptionsResponse>, AppError> {
    if let Some(ref u) = user {
        tracing::info!(
            "{} GET /cases/{}/scenarios/{}/allegation-options",
            u.username,
            slug,
            scenario_id
        );
    }

    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    // No target → no catalogue. The accusations offered here are the ones this
    // scenario's SUBJECT is accused of, so a scenario that names no subject has
    // no basis for a list; serving the case-default subject's accusations was
    // the same borrowed-identity defect this task removes from gather and cards
    // (2026-08-07, ruled: kill the fallback everywhere it reaches).
    //
    // In practice this is unreachable from the UI — the panels this feeds only
    // appear on cards, and a target-less scenario now has none — but "unreachable
    // today" is not a contract, and an endpoint that quietly answered with
    // somebody else's accusations would be a live defect the moment a caller
    // reached it.
    let Some(subject_id) = resolve_gather_subject(&state, id).await? else {
        return Ok(Json(empty_options(&state)));
    };

    // The scenario's own anchors, which head the short list. A scenario row that
    // vanished between the fence check and here is a race, and a 404 — the same
    // treatment `resolve_gather_subject` gives it.
    let record = get_scenario(&state.pipeline_pool, id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, scenario_id = %id, "failed to read the scenario for its anchors");
            AppError::Internal {
                message: "failed to load scenario".to_string(),
            }
        })?
        .ok_or_else(|| {
            tracing::warn!(scenario_id = %id, "scenario vanished while building the accusation list");
            AppError::NotFound {
                message: "scenario not found".to_string(),
            }
        })?;

    let rows = fetch_allegation_options(&state.graph, &subject_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, %subject_id, "failed to read the accusation catalogue");
            AppError::Internal {
                message: "failed to read the accusations".to_string(),
            }
        })?;

    // `anchor_allegation_ids` is nullable, and the three states it can be in are
    // all "no anchors" for this purpose — but they are distinguishable in the
    // column, and collapsing them here loses nothing because the ORDER is the only
    // thing that depends on it.
    let anchors = record.anchor_allegation_ids.unwrap_or_default();
    let settings = state.settings.current();
    let response = build_options(rows, &anchors, &settings);

    tracing::info!(
        scenario_id = %id,
        total = response.total,
        short_list = response.serving.len(),
        anchors = anchors.len(),
        "served the accusation options"
    );

    Ok(Json(response))
}

/// The catalogue for a scenario that names no subject: no accusations, and the
/// panel's wording intact.
///
/// ## Why this goes through `build_options` rather than constructing the DTO
///
/// The response carries `wording`, which `build_options` composes from stored
/// rows (including a "show all" label that bakes in the total). Hand-building
/// the struct here would be a second place that knows how to word this panel,
/// free to drift from the first. Passing no rows and no anchors gives the same
/// function the empty case, and the wording is composed exactly once.
fn empty_options(state: &AppState) -> AllegationOptionsResponse {
    build_options(Vec::new(), &[], &state.settings.current())
}

/// `POST …/evidence/:graph_node_id/links` — link a statement to accusations.
///
/// Every accusation in one request shares one cut, which is how the panel works:
/// a human ticks what this statement bears on, then says which way it runs. A
/// statement that cuts differently on two accusations is saved in two actions,
/// and each row keeps its own cut.
#[tracing::instrument(skip(state, user, payload), fields(slug = %slug, graph_node_id = %graph_node_id))]
pub async fn save_links(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, graph_node_id)): Path<(String, String)>,
    Json(payload): Json<SaveLinksRequest>,
) -> Result<Json<SaveLinksResponse>, AppError> {
    require_edit(&user)?;

    let settings = state.settings.current();
    let ids = validate_allegation_ids(&payload.allegation_ids, &settings)?;

    // One timestamp for every row this request writes, so a save of three
    // accusations reads as one act in the ledger rather than three that happened
    // to be milliseconds apart.
    let written_at = Utc::now();
    let mut linked = 0usize;
    let mut recut = 0usize;

    for allegation_id in &ids {
        let action = save_one_link(
            &state,
            &graph_node_id,
            allegation_id,
            payload.cut,
            &user.username,
            written_at,
        )
        .await?;

        match action {
            crate::domain::link_cut::LinkAction::Recut => recut += 1,
            _ => linked += 1,
        }
    }

    tracing::info!(
        author = %user.username,
        graph_node_id = %graph_node_id,
        cut = payload.cut.code(),
        linked,
        recut,
        "linked a statement to accusations"
    );

    Ok(Json(SaveLinksResponse {
        graph_node_id,
        linked,
        recut,
    }))
}

/// Save one accusation link and mirror it onto the Proof Matrix, in that order.
///
/// Split from [`save_links`] so that handler reads as the loop and the tally it
/// is, and so the two writes one accusation needs sit together, where the failure
/// message can say which of them stopped.
///
/// # Errors
/// Returns [`AppError::Internal`] with the failing half named in the operator
/// log. The enclosing loop is NOT atomic across accusations — an earlier one may
/// already be committed — which is why the message says so.
async fn save_one_link(
    state: &AppState,
    graph_node_id: &str,
    allegation_id: &str,
    cut: crate::domain::link_cut::LinkCut,
    author: &str,
    written_at: chrono::DateTime<Utc>,
) -> Result<crate::domain::link_cut::LinkAction, AppError> {
    let action = save_link(
        &state.pipeline_pool,
        &LinkWrite {
            graph_node_id,
            allegation_id,
            cut,
            authored_by: author,
            written_at,
        },
    )
    .await
    .map_err(|e| {
        tracing::error!(
            error = %e,
            %graph_node_id,
            %allegation_id,
            %author,
            // Named because the loop is not atomic across accusations: an earlier
            // one may already be stored, and an operator reading this needs to
            // know which one stopped.
            "failed to save an accusation link; earlier links in this request \
             are already committed"
        );
        AppError::Internal {
            message: "failed to save the link".to_string(),
        }
    })?;

    mirror_as_matrix_keep(state, graph_node_id, allegation_id, author, written_at).await?;
    Ok(action)
}

/// Mirror a human link onto the Proof Matrix as a `keep` ruling (v2 §2).
///
/// ## Why one act writes two tables
///
/// Including a statement in a scenario with a stance and an accusation IS the
/// judgment the Matrix's Keep button records: a human said this statement bears
/// on that accusation and belongs there. Leaving the Matrix unaware of it would
/// mean the same person, having done the work once, is asked to do it again on
/// another page — and, worse, that the Matrix goes on marking the item `machine`
/// after a human has been through it. §2 states it as one human act on two
/// surfaces, and this is where that becomes true.
///
/// ## Why this is a second transaction and not one
///
/// The two tables live in the same database, so a single transaction is possible
/// — and the two writes are NOT equals. The link is what makes a card rulable;
/// the mirrored keep is a convenience on another page. Widening the link's
/// transaction to hold both would make a Matrix failure roll back a link the
/// human is waiting on. Instead both are attempted, both are propagated, and the
/// log below says exactly which half is stored — the same honesty the enclosing
/// loop already applies across accusations.
///
/// The write is an UPSERT, so a statement linked, unlinked and linked again does
/// not accumulate rulings; it re-affirms the one row.
async fn mirror_as_matrix_keep(
    state: &AppState,
    evidence_id: &str,
    allegation_id: &str,
    author: &str,
    written_at: chrono::DateTime<Utc>,
) -> Result<(), AppError> {
    save_ruling(
        &state.pipeline_pool,
        &RulingWrite {
            evidence_id,
            allegation_id,
            ruling: crate::domain::matrix_ruling::MatrixRuling::Keep,
            ruled_by: author,
            note: None,
            written_at,
        },
    )
    .await
    .map(|_| ())
    .map_err(|e| {
        tracing::error!(
            error = %e,
            graph_node_id = %evidence_id,
            %allegation_id,
            %author,
            "the accusation link WAS saved but its Proof Matrix keep was not — the \
             card is linked and rulable, and the Matrix will still show this item \
             as unread machine output until the link is saved again"
        );
        AppError::Internal {
            message: "failed to record the link on the Proof Matrix".to_string(),
        }
    })
}

/// The one validation rule on the save path, extracted so it can be tested.
///
/// ## Why the refusal's words come from the STORE
///
/// The panel refuses an empty selection before making the round trip, using the
/// same stored sentence — and if those two sentences were written in two places
/// they would drift, leaving a human told two different things about one mistake
/// depending on whether the browser or the server caught it (Roman's R4).
///
/// The cut needs no check here: it is a typed enum on the request, so serde
/// refuses anything outside the vocabulary before this handler runs.
///
/// Returns the DEDUPLICATED ids, so a client that ticks the same accusation twice
/// writes one row and one ledger entry rather than a link followed by a re-cut of
/// itself.
fn validate_allegation_ids(
    raw: &[String],
    settings: &crate::domain::settings::Settings,
) -> Result<Vec<String>, AppError> {
    let mut ids: Vec<String> = Vec::new();
    for id in raw {
        let id = id.trim();
        if id.is_empty() {
            continue;
        }
        if !ids.iter().any(|kept| kept == id) {
            ids.push(id.to_string());
        }
    }

    if ids.is_empty() {
        return Err(AppError::BadRequest {
            message: settings.wording.link_missing_allegation_refusal.clone(),
            details: serde_json::json!({ "field": "allegation_ids" }),
        });
    }
    Ok(ids)
}

/// `DELETE …/evidence/:graph_node_id/links/:allegation_id` — take a link back.
///
/// One click, reversible, and honest about what it did: unlinking a pair that was
/// not linked reports `removed: false` rather than claiming work it did not do.
/// The card returns to defer-only on the next read, which is the truthful state —
/// nothing links it to an accusation again.
#[tracing::instrument(skip(state, user), fields(slug = %slug, graph_node_id = %graph_node_id))]
pub async fn remove_link(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, graph_node_id, allegation_id)): Path<(String, String, String)>,
) -> Result<Json<UnlinkResponse>, AppError> {
    require_edit(&user)?;

    let removed = delete_link(
        &state.pipeline_pool,
        &graph_node_id,
        &allegation_id,
        &user.username,
        Utc::now(),
    )
    .await
    .map_err(|e| {
        tracing::error!(
            error = %e,
            slug = %slug,
            graph_node_id = %graph_node_id,
            %allegation_id,
            author = %user.username,
            "failed to remove an accusation link"
        );
        AppError::Internal {
            message: "failed to remove the link".to_string(),
        }
    })?;

    // Removing nothing is not an error — the caller wanted the pair unlinked and
    // unlinked is what it is. It IS worth recording differently, because it means
    // the client was showing a link the database did not have.
    if removed {
        tracing::info!(
            author = %user.username,
            graph_node_id = %graph_node_id,
            %allegation_id,
            "unlinked a statement from an accusation"
        );
    } else {
        tracing::warn!(
            author = %user.username,
            graph_node_id = %graph_node_id,
            %allegation_id,
            "unlink found no link to remove — the client's view was stale"
        );
    }

    Ok(Json(UnlinkResponse {
        graph_node_id,
        allegation_id,
        removed,
    }))
}

#[cfg(test)]
#[path = "evidence_links_tests.rs"]
mod tests;
