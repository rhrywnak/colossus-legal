//! `GET /cases/:slug/scenarios/:scenario_id/facts/orphans` — the humane orphan
//! strip (task 1.7C, defect D9).
//!
//! ## What this endpoint answers
//!
//! "Which saved references on this scenario point at graph nodes that no longer
//! exist, and which documents did they come from?"
//!
//! Design §2.8 moved that strip from beside the ruling queue to the bottom of the
//! page and made it a plain-language disclosure. It therefore no longer wants the
//! queue's card payload, which is why this is its own read rather than a field on
//! the cards response.
//!
//! ## Why the orphan test changed with the move (stated, not slipped in)
//!
//! Until 1.7C the browser derived orphans by set-differencing the saved facts
//! against the CARD POOL: a saved ref not in the pool was shown as an orphan. That
//! conflates two different things — a node that is GONE, and a node that is alive
//! but no longer about this scenario's subject. The strip's own words are "content
//! unavailable", which is only true of the first.
//!
//! So the test here is the honest one: an orphan is a ref whose `graph_node_id`
//! does not resolve to a live Evidence node. On today's data the two definitions
//! agree exactly — measured 2026-08-03, all 26 of S-2's refs are dead pointers,
//! resolving to nothing at all — so the move costs no coverage today and stops the
//! strip mislabelling a live-but-off-pool ref tomorrow.
//!
//! The orphan guarantee is unchanged and is the reason this endpoint exists: a
//! confirmed fact must never silently disappear.

use axum::{
    extract::{Path, State},
    Json,
};
use std::collections::{HashMap, HashSet};

use crate::{
    api::scenario_facts::{ensure_scenario_in_case, parse_scenario_id},
    auth::AuthUser,
    bias::repository::BiasRepository,
    dto::scenario_orphans::ScenarioOrphansResponse,
    error::AppError,
    repositories::pipeline_repository::{document_titles_by_ids, list_fact_refs_for_scenario},
    services::scenario_orphans::{document_ids_of, group_orphans},
    state::AppState,
};

/// List this scenario's orphaned references, grouped by source document.
///
/// Open read (`Option<AuthUser>`), matching its sibling facts routes.
///
/// Three reads, in the only order that works: the refs (Postgres), which nodes of
/// those still resolve (Neo4j), then the titles of the documents the survivors'
/// dead siblings came from (Postgres). Each failure is logged with the scenario id
/// and surfaced as a 500 — never as an empty strip, which would read as "nothing
/// was lost" and is the exact silent failure the orphan guarantee exists to
/// prevent.
#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn list_scenario_orphans(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
) -> Result<Json<ScenarioOrphansResponse>, AppError> {
    if let Some(ref u) = user {
        tracing::info!(
            "{} GET /cases/{}/scenarios/{}/facts/orphans",
            u.username,
            slug,
            scenario_id
        );
    }

    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    let ids = saved_ref_ids(&state, id).await?;

    // No refs at all → nothing can be orphaned. The scenario exists (fenced above),
    // so this is a genuinely empty strip and needs no graph round trip.
    if ids.is_empty() {
        return Ok(Json(ScenarioOrphansResponse {
            total: 0,
            groups: Vec::new(),
        }));
    }

    let orphan_ids = unresolved_ids(&state, id, ids).await?;
    let document_ids = document_ids_of(&orphan_ids);
    let titles = resolve_titles(&state, id, &document_ids).await?;
    let response = group_orphans(&orphan_ids, &titles);

    // Logged at INFO because this count IS the July loss, and a change in it is
    // something an operator should be able to see in the log without a database.
    tracing::info!(
        scenario_id = %id,
        orphans = response.total,
        groups = response.groups.len(),
        documents_resolved = titles.len(),
        documents_referenced = document_ids.len(),
        "assembled the orphan strip"
    );

    Ok(Json(response))
}

/// Every `graph_node_id` this scenario has a saved ruling for.
///
/// Split out of the handler for the function-size limit (Rule 18), and it earns the
/// split: the handler now reads as its four steps — the refs, which are dead, their
/// documents' titles, the grouping — instead of burying that sequence under three
/// near-identical `map_err` blocks.
async fn saved_ref_ids(state: &AppState, id: uuid::Uuid) -> Result<Vec<String>, AppError> {
    let refs = list_fact_refs_for_scenario(&state.pipeline_pool, id)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e, scenario_id = %id,
                "failed to list scenario fact refs while assembling the orphan strip"
            );
            AppError::Internal {
                message: "failed to list this scenario's saved references".to_string(),
            }
        })?;
    Ok(refs.into_iter().map(|r| r.graph_node_id).collect())
}

/// The subset of `ids` the graph no longer resolves — the orphans.
///
/// A ref is an orphan when `evidence_by_ids` returned nothing for its id. The graph
/// read is a HARD failure rather than a degradation: without knowing which nodes are
/// alive, the honest answer is not "no orphans" — it is "we could not check", and
/// rendering the former would be the page saying nothing was lost.
async fn unresolved_ids(
    state: &AppState,
    id: uuid::Uuid,
    ids: Vec<String>,
) -> Result<Vec<String>, AppError> {
    let live = BiasRepository::new(state.graph.clone())
        .evidence_by_ids(&ids)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e, scenario_id = %id, ref_count = ids.len(),
                "failed to resolve saved references against the graph; the orphan \
                 strip cannot be assembled without knowing which nodes are alive"
            );
            AppError::Internal {
                message: "failed to check this scenario's saved references against the graph"
                    .to_string(),
            }
        })?;

    // ## Rust Learning: a `HashSet` for the membership test, not a `Vec` scan
    //
    // This is a set-difference over up to a few hundred ids. A `Vec` plus
    // `iter().any()` makes it quadratic (and clippy's `manual_contains` lint says
    // so); a `HashSet` makes each lookup constant-time and the intent — "is this id
    // among the live ones?" — reads directly off the call.
    let resolved: HashSet<&str> = live.iter().map(|e| e.evidence_id.as_str()).collect();
    Ok(ids
        .iter()
        .filter(|id| !resolved.contains(id.as_str()))
        .cloned()
        .collect())
}

/// `document_id` → `title`, for the documents the orphans came from.
///
/// An id missing from the map is the ANSWER (that document is gone), not a failure —
/// see `document_titles_by_ids`. Only the read itself failing is an error.
async fn resolve_titles(
    state: &AppState,
    id: uuid::Uuid,
    document_ids: &[String],
) -> Result<HashMap<String, String>, AppError> {
    let rows = document_titles_by_ids(&state.pipeline_pool, document_ids)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e, scenario_id = %id, document_count = document_ids.len(),
                "failed to read document titles for the orphan strip"
            );
            AppError::Internal {
                message: "failed to read the document titles for this scenario's \
                          unavailable references"
                    .to_string(),
            }
        })?;
    Ok(rows.into_iter().collect())
}
