//! Reading the accusation section (task 2.11 B1).
//!
//! `GET /cases/:slug/scenarios/:scenario_id/accusation` → everything the working
//! view's accusation section renders, in one read.
//!
//! Split from `api::scenario_accusation` for Rule 17, the same way
//! `scenario_fact_curation_reads` was split from its handlers: the writes and
//! their fence stay next door, and the five reads plus their two loud decodes land
//! here. The composition itself is pure and lives in
//! `services::scenario_accusation_panel`, so every sentence this surface speaks is
//! decided somewhere a test can reach without a database.
//!
//! ## The five reads, and why the graph one is so small
//!
//! Four from Postgres — the scenario row (its authored accusation), the fact refs
//! (what is included), the human-fact rows (the judgments), and the candidate
//! ordinals (the handles). One from Neo4j, and it projects two columns: an
//! evidence id and the document it sits in. See
//! `repositories::scenario_accusation_repository` for why that query exists rather
//! than reusing the 148-row card pool.
//!
//! ## CRITICAL — the pipeline pool
//!
//! `scenarios`, `scenario_fact_refs`, `scenario_human_facts` and
//! `scenario_candidate_ordinals` all live in `colossus_legal_v2`, so every SQL
//! read here uses `&state.pipeline_pool`, NOT `state.pg_pool`.

use std::collections::HashSet;

use axum::{
    extract::{Path, State},
    Json,
};

use crate::{
    auth::AuthUser,
    domain::{fact_status::FactStatus, human_authored::HumanFactKind},
    dto::scenario_accusation::AccusationPanelDto,
    error::AppError,
    repositories::{
        pipeline_repository::{
            get_scenario, list_candidate_ordinals, list_fact_refs_for_scenario,
            list_human_facts_for_scenario, ScenarioHumanFactRecord,
        },
        scenario_accusation_repository::fetch_documents_for_nodes,
    },
    services::{
        scenario_accusation::{derive, StoredJudgment},
        scenario_accusation_panel::{build_panel, PanelInput},
    },
    state::AppState,
};

use super::scenario_facts::{ensure_scenario_in_case, parse_scenario_id};

/// `GET …/accusation` — the whole accusation section in one read.
///
/// Open read (`Option<AuthUser>`), matching the sibling scenario reads: looking at
/// what a human has marked is not an edit. The WRITES next door are edit-gated.
#[tracing::instrument(skip(state, user), fields(slug = %slug, scenario_id = %scenario_id))]
pub async fn get_accusation_panel(
    user: Option<AuthUser>,
    State(state): State<AppState>,
    Path((slug, scenario_id)): Path<(String, String)>,
) -> Result<Json<AccusationPanelDto>, AppError> {
    if let Some(ref u) = user {
        tracing::info!("{} GET accusation panel for {}", u.username, scenario_id);
    }

    let id = parse_scenario_id(&scenario_id)?;
    ensure_scenario_in_case(&state, id, &slug).await?;

    let scenario = get_scenario(&state.pipeline_pool, id)
        .await
        .map_err(|e| read_failure(e, id, "the scenario row"))?
        .ok_or_else(|| AppError::NotFound {
            message: "scenario not found".to_string(),
        })?;

    let refs = list_fact_refs_for_scenario(&state.pipeline_pool, id)
        .await
        .map_err(|e| read_failure(e, id, "this scenario's fact refs"))?;
    let included = included_ids(&refs, id)?;

    let records = list_human_facts_for_scenario(&state.pipeline_pool, id)
        .await
        .map_err(|e| read_failure(e, id, "this scenario's human judgments"))?;
    let judgments = judgments_in(&records, id)?;

    let ordinals = list_candidate_ordinals(&state.pipeline_pool, id)
        .await
        .map_err(|e| read_failure(e, id, "this scenario's candidate ordinals"))?;

    let state_of_accusation = derive(&judgments, &included);
    let documents = documents_of_marked(&state, id, &state_of_accusation).await?;

    let settings = state.settings.current();
    Ok(Json(build_panel(PanelInput {
        accusation_text: scenario.accusation_text.as_deref(),
        state: &state_of_accusation,
        ordinals: &ordinals,
        documents: &documents,
        included_count: included.len(),
        wording: &settings.accusation_wording,
    })))
}

/// Which document each MARKED instance sits in.
///
/// Only the marked anchors are looked up, never the whole pool: the count is about
/// where the accusation was said, and nothing else on this payload needs a
/// document at all. A scenario with nothing marked skips the round trip entirely
/// (the repository returns early on an empty list).
///
/// ## Rust Learning: the ids are collected before the await, not borrowed across it
///
/// `fetch_documents_for_nodes` takes `&[String]`, and building that slice from
/// `state.instances` means cloning the ids. Borrowing them instead would tie the
/// future's lifetime to `AccusationState`, which the caller still needs afterwards
/// — the clone buys an independent value, and it is a few dozen short strings.
async fn documents_of_marked(
    state: &AppState,
    scenario_id: uuid::Uuid,
    accusation: &crate::services::scenario_accusation::AccusationState,
) -> Result<std::collections::HashMap<String, String>, AppError> {
    let marked: Vec<String> = accusation
        .instances
        .iter()
        .map(|instance| instance.anchor_graph_node_id.clone())
        .collect();

    fetch_documents_for_nodes(&state.graph, &marked)
        .await
        .map_err(|e| {
            // A graph read that FAILED must not become "in 0 documents". That
            // would be a confidently wrong number on a surface whose whole claim
            // is that its counts are honest — and the human would have no way to
            // tell it apart from an accusation genuinely said nowhere.
            tracing::error!(
                error = %e,
                %scenario_id,
                marked = marked.len(),
                "failed to read the documents the marked instances sit in"
            );
            AppError::Internal {
                message: "failed to load the accusation".to_string(),
            }
        })
}

/// One read failure, logged with what it was reading and served opaquely.
///
/// Opaque on purpose, unlike the write refusals next door: nothing here is about
/// what the human typed, so there is nothing for them to act on. The LOG names
/// which of the five reads failed, which is what an operator needs.
fn read_failure(
    error: crate::repositories::pipeline_repository::PipelineRepoError,
    scenario_id: uuid::Uuid,
    what: &'static str,
) -> AppError {
    tracing::error!(error = %error, %scenario_id, "failed to read {}", what);
    AppError::Internal {
        message: "failed to load the accusation".to_string(),
    }
}

/// The graph node ids currently INCLUDED in the scenario.
///
/// This set is what turns the Remove law from a promise into a rendered gap:
/// every anchor a stored judgment points at is checked against it.
///
/// ## Why an unreadable status refuses instead of being treated as "not included"
///
/// Silently excluding it would drop a marked instance out of the count and out of
/// the gap list at the same time — the accusation would quietly be said one fewer
/// time, with nothing on screen to say why. That is the exact class of failure
/// Standing Rule 1 forbids, and it is a data-integrity fault rather than an
/// answer.
fn included_ids(
    refs: &[crate::repositories::pipeline_repository::ScenarioFactRefRecord],
    scenario_id: uuid::Uuid,
) -> Result<HashSet<String>, AppError> {
    let mut out = HashSet::new();
    for r in refs {
        let status = FactStatus::try_from(r.status.as_str()).map_err(|e| {
            tracing::error!(
                error = %e,
                graph_node_id = %r.graph_node_id,
                %scenario_id,
                "scenario_fact_refs carries a status token this build does not define"
            );
            AppError::Internal {
                message: "failed to load the accusation".to_string(),
            }
        })?;
        if status == FactStatus::Included {
            out.insert(r.graph_node_id.clone());
        }
    }
    Ok(out)
}

/// The stored ANCHORED judgments, in the narrow shape the derivation needs.
///
/// ## The three cases, and why only one of them is silent
///
/// A row with no anchor is a fact or a watch-list note — prose a human wrote,
/// about nothing but itself. It belongs to another section entirely and is skipped
/// without comment, which is the ordinary state of every scenario.
///
/// A row WITH an anchor whose kind this build cannot read is different in kind: it
/// is somebody's judgment about a specific statement, and this build cannot say
/// which judgment. Rendering the panel without it would silently drop a human's
/// decision — so it refuses, and the log names the token and the row.
///
/// A row with an anchor and a KNOWN prose kind (a `fact` that somehow carries an
/// anchor) is refused for the same reason: the shape says the row is about a
/// statement while the kind says it is not, and nothing here may pick a winner.
fn judgments_in(
    records: &[ScenarioHumanFactRecord],
    scenario_id: uuid::Uuid,
) -> Result<Vec<StoredJudgment>, AppError> {
    let mut out = Vec::new();
    for r in records {
        let Some(anchor) = r.anchor_graph_node_id.as_deref() else {
            continue;
        };

        let kind = HumanFactKind::try_from(r.kind.as_str()).map_err(|e| {
            tracing::error!(
                error = %e,
                fact_id = %r.id,
                %scenario_id,
                anchor,
                "an anchored row carries a kind token this build does not define"
            );
            AppError::Internal {
                message: "failed to load the accusation".to_string(),
            }
        })?;

        if !kind.is_anchored() {
            tracing::error!(
                fact_id = %r.id,
                %scenario_id,
                anchor,
                kind = kind.code(),
                "a prose kind carries an anchor; the row's shape and its kind disagree"
            );
            return Err(AppError::Internal {
                message: "failed to load the accusation".to_string(),
            });
        }

        out.push(StoredJudgment {
            kind,
            anchor_graph_node_id: anchor.to_string(),
            answers_graph_node_id: r.answers_graph_node_id.clone(),
        });
    }
    Ok(out)
}

#[cfg(test)]
#[path = "scenario_accusation_read_tests.rs"]
mod tests;
