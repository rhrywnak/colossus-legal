//! Weighing and placing a fact already ruled into a scenario (task 2.13).
//!
//! - `POST …/scenarios/:scenario_id/facts/:graph_node_id/tier`  → how much it carries
//! - `POST …/scenarios/:scenario_id/facts/:graph_node_id/order` → where it sits
//!
//! Two static children under the `:graph_node_id` param, beside the `action`
//! route the ruling path already uses — the same shape matchit 0.7.3 accepts for
//! `gather` / `cards` / `orphans`.
//!
//! ## Why these are separate routes and not fields on the ruling action
//!
//! A ruling says whether a fact belongs in the scenario at all; these say what it
//! is WORTH and where it goes once it does. Folding them into `/action` would put
//! a presentation judgment in the same request as a forensic one, and the ledger
//! would then have to decide whether starring is a ruling. It is not — §8 is
//! explicit that starring and ordering are human augmentation that never triggers
//! re-gathering, and nothing here writes to `scenario_ruling_anchors`.
//!
//! ## Both are edit-gated, and both refuse a fact that is not here
//!
//! `require_edit` first, exactly like the link writes. Then: weighing or placing
//! something that is not in the scenario is a request about a thing that is not
//! there, and it answers 404 rather than quietly creating a row (Standing Rule 1).
//!
//! ## CRITICAL — the pipeline pool
//!
//! `scenario_fact_refs` lives in `colossus_legal_v2`: `&state.pipeline_pool`.

use axum::{
    extract::{Path, State},
    routing::post,
    Json, Router,
};
use serde_json::json;

use crate::{
    auth::{require_edit, AuthUser},
    dto::scenario_curation::{MoveFactRequest, SetTierRequest},
    error::AppError,
    repositories::pipeline_repository::{
        clear_fact_sort_ordinal, set_fact_sort_ordinal, set_fact_tier,
    },
    services::scenario_fact_order::{plan_move, MoveRefusal},
    state::AppState,
};

use super::scenario_fact_curation_reads::read_orderable_facts;
use super::scenario_facts::{ensure_scenario_in_case, parse_scenario_id};

/// The route group for the two curation writes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/cases/:slug/scenarios/:scenario_id/facts/:graph_node_id/tier",
            post(set_tier),
        )
        .route(
            "/cases/:slug/scenarios/:scenario_id/facts/:graph_node_id/order",
            post(move_fact),
        )
        // Un-place: forget where a human dragged one fact. A DELETE on the same
        // path the POST writes, because that is exactly what it does — it removes
        // the placement, it does not create a different one.
        .route(
            "/cases/:slug/scenarios/:scenario_id/facts/:graph_node_id/order",
            axum::routing::delete(unplace_fact),
        )
}

/// `POST …/facts/:graph_node_id/tier` — record how much a fact carries.
#[tracing::instrument(skip(state, user, payload), fields(slug = %slug, scenario_id = %scenario_id, graph_node_id = %graph_node_id))]
pub async fn set_tier(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id, graph_node_id)): Path<(String, String, String)>,
    Json(payload): Json<SetTierRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_edit(&user)?;
    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    let rows = set_fact_tier(
        &state.pipeline_pool,
        id,
        &graph_node_id,
        payload.tier,
        &user.username,
    )
    .await
    .map_err(|e| {
        tracing::error!(
            error = %e,
            %graph_node_id,
            scenario_id = %id,
            author = %user.username,
            "failed to store a fact's tier"
        );
        AppError::Internal {
            message: "failed to save the weight".to_string(),
        }
    })?;

    require_row_touched(
        rows,
        &user.username,
        &graph_node_id,
        id,
        "no fact ref matched when storing a tier; the fact left the scenario \
         between the page load and this write",
    )?;

    tracing::info!(
        author = %user.username,
        %graph_node_id,
        scenario_id = %id,
        tier = payload.tier.code(),
        "stored a fact's tier"
    );
    Ok(Json(json!({ "tier": payload.tier })))
}

/// `POST …/facts/:graph_node_id/order` — record where a dragged fact landed.
///
/// Reads the scenario's included facts, computes ONE ordinal with
/// [`plan_move`], and writes ONE row. The arithmetic is in the service so it is
/// unit-tested without a database; the reads are here because the service is pure.
#[tracing::instrument(skip(state, user, payload), fields(slug = %slug, scenario_id = %scenario_id, graph_node_id = %graph_node_id))]
pub async fn move_fact(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id, graph_node_id)): Path<(String, String, String)>,
    Json(payload): Json<MoveFactRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_edit(&user)?;
    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    let facts = read_orderable_facts(&state, id).await?;
    let ordinal = plan_move(
        &facts,
        &graph_node_id,
        payload.after.as_deref(),
        payload.before.as_deref(),
    )
    .map_err(|refusal| {
        log_move_refusal(&refusal, &user.username, &graph_node_id, id, &payload);
        move_refusal_to_app_error(refusal, &graph_node_id)
    })?;

    store_ordinal(&state, id, &graph_node_id, ordinal, &user.username).await?;
    Ok(Json(json!({ "sort_ordinal": ordinal })))
}

/// Write the computed ordinal, and account for every way that can go wrong.
///
/// Split from the handler for the function-size limit; the split is also where
/// the three outcomes of one UPDATE become legible side by side — the statement
/// failed (a server fault, opaque to the client), it touched nothing (a race,
/// 404), or it landed (logged with the number, so the ordering is reconstructable
/// from the log alone).
async fn store_ordinal(
    state: &AppState,
    scenario_id: uuid::Uuid,
    graph_node_id: &str,
    ordinal: i32,
    author: &str,
) -> Result<(), AppError> {
    let rows = set_fact_sort_ordinal(
        &state.pipeline_pool,
        scenario_id,
        graph_node_id,
        ordinal,
        author,
    )
    .await
    .map_err(|e| {
        tracing::error!(
            error = %e,
            %graph_node_id,
            %scenario_id,
            %author,
            "failed to store a fact's position"
        );
        AppError::Internal {
            message: "failed to save the new order".to_string(),
        }
    })?;

    // Unlike the tier path, a zero here names a genuine RACE: `plan_move` already
    // refused a fact that is not here, so the read SAW the row and the write did
    // not. An operator seeing this repeatedly is looking at two people fighting
    // over one scenario, which no other log line would tell them.
    require_row_touched(
        rows,
        author,
        graph_node_id,
        scenario_id,
        "the fact ref was present when the move was planned and absent when it was \
         written; a concurrent removal won the race",
    )?;

    tracing::info!(
        %author,
        %graph_node_id,
        %scenario_id,
        ordinal,
        "stored a fact's position"
    );
    Ok(())
}

/// Turn "the UPDATE touched nothing" into a named 404, loudly.
///
/// ## Why a zero row count is never a success
///
/// Both writes are `UPDATE … WHERE (scenario_id, graph_node_id) = …`. Zero rows
/// means the pair matched nothing — the human is acting on a fact that is not in
/// this scenario, because their page is stale or somebody removed it underneath
/// them. Returning 200 would report a save that never happened, which is the
/// exact shape of failure Standing Rule 1 exists to forbid.
///
/// `warn`, not `error`: this is an expected consequence of two people working one
/// scenario, not a fault. But it IS operationally distinct from the SQL-failure
/// path and from success, so it gets its own event — a `#[tracing::instrument]`
/// span does not emit one on an early return, and an operator chasing "my star
/// would not stick" needs something to search for.
///
/// Shared by both handlers so the two cannot drift into answering differently;
/// the `context` message is what distinguishes them in the log.
fn require_row_touched(
    rows: u64,
    author: &str,
    graph_node_id: &str,
    scenario_id: uuid::Uuid,
    context: &str,
) -> Result<(), AppError> {
    if rows > 0 {
        return Ok(());
    }
    tracing::warn!(%author, %graph_node_id, %scenario_id, "{}", context);
    Err(AppError::NotFound {
        message: "that fact is not in this scenario".to_string(),
    })
}

/// Record a refused move for the operator, with everything needed to reproduce it.
///
/// Split from the handler for the function-size limit, and it earns the split:
/// the fields it names — which fact, which two neighbours, whose session — are
/// the whole reason the refusal is diagnosable at all. `warn`, not `error`: all
/// three refusals are expected states of a page whose list moved underneath it.
fn log_move_refusal(
    refusal: &MoveRefusal,
    author: &str,
    graph_node_id: &str,
    scenario_id: uuid::Uuid,
    payload: &MoveFactRequest,
) {
    tracing::warn!(
        refusal = %refusal,
        %author,
        %graph_node_id,
        %scenario_id,
        // `<top>` / `<bottom>` rather than an empty field: "dropped at the top"
        // and "the client sent no neighbour" read identically as a blank, and the
        // first is a normal drop while the second would be a client bug.
        after = payload.after.as_deref().unwrap_or("<top>"),
        before = payload.before.as_deref().unwrap_or("<bottom>"),
        "refused to move a fact"
    );
}

/// `DELETE …/facts/:graph_node_id/order` — forget where a fact was placed.
///
/// Returns the fact to the list's natural (C-ordinal) order. Edit-gated and
/// case-fenced like its siblings; a fact that is not in this scenario is a 404,
/// never a silent success.
#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id, graph_node_id = %graph_node_id))]
pub async fn unplace_fact(
    user: AuthUser,
    State(state): State<AppState>,
    Path((slug, scenario_id, graph_node_id)): Path<(String, String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_edit(&user)?;
    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    let rows = clear_fact_sort_ordinal(&state.pipeline_pool, id, &graph_node_id)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                %graph_node_id,
                scenario_id = %id,
                author = %user.username,
                "failed to clear a fact's position"
            );
            AppError::Internal {
                message: "failed to clear the order".to_string(),
            }
        })?;

    require_row_touched(
        rows,
        &user.username,
        &graph_node_id,
        id,
        "no fact ref matched when clearing a position; the fact left the scenario \
         between the page load and this write",
    )?;

    tracing::info!(
        author = %user.username,
        %graph_node_id,
        scenario_id = %id,
        "cleared a fact's position"
    );
    Ok(Json(json!({ "sort_ordinal": serde_json::Value::Null })))
}

/// Map a [`MoveRefusal`] onto its HTTP status, keeping the service HTTP-free.
///
/// ## Why these messages reach the client verbatim
///
/// Same reasoning as `ruling_error_to_app_error`: each names something about the
/// ITEM the human is looking at, and the human is the only one who can act on it.
/// "There is no room left between those two facts" tells them to drop it
/// somewhere slightly different; a generic "could not save" would leave them
/// dragging the same row forever.
///
/// A stale neighbour is a 409 rather than a 404: the route, the scenario and the
/// dragged fact all exist and the request was well-formed — it is the list that
/// moved underneath it.
fn move_refusal_to_app_error(refusal: MoveRefusal, graph_node_id: &str) -> AppError {
    let details = json!({ "graph_node_id": graph_node_id });
    match refusal {
        MoveRefusal::DraggedNotHere => AppError::NotFound {
            message: refusal.to_string(),
        },
        MoveRefusal::NeighbourNotHere => AppError::Conflict {
            message: refusal.to_string(),
            details,
        },
        MoveRefusal::GapExhausted => AppError::Conflict {
            message: refusal.to_string(),
            details,
        },
    }
}

#[cfg(test)]
#[path = "scenario_fact_curation_tests.rs"]
mod tests;
