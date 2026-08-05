//! Reading a scenario's facts into the shape the ordering service needs.
//!
//! Split from `api::scenario_fact_curation` for the 300-line limit (Rule 17),
//! the same way `scenario_facts_mapping` was split from `scenario_facts`. What
//! lands here is the I/O and the two loud decodes; what stays next door is the
//! four HTTP handlers.
//!
//! ## CRITICAL — the pipeline pool
//!
//! `scenario_fact_refs` lives in `colossus_legal_v2`: `&state.pipeline_pool`.

use crate::{
    domain::{fact_status::FactStatus, fact_tier::FactTier},
    error::AppError,
    repositories::pipeline_repository::{list_candidate_ordinals, list_fact_refs_for_scenario},
    services::scenario_fact_order::OrderableFact,
    state::AppState,
};

/// The scenario's INCLUDED facts, in the shape the ordering service needs.
///
/// Only included ones: the facts list is what a human has put IN the scenario,
/// and an undecided or dropped candidate has no place in an order it does not
/// appear in. Filtering here rather than in the service keeps the service's input
/// honest — it orders what it is given, and is never handed rows that should not
/// be orderable.
pub(crate) async fn read_orderable_facts(
    state: &AppState,
    scenario_id: uuid::Uuid,
) -> Result<Vec<OrderableFact>, AppError> {
    let refs = list_fact_refs_for_scenario(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, %scenario_id, "failed to read fact refs for a move");
            AppError::Internal {
                message: "failed to read this scenario's facts".to_string(),
            }
        })?;
    let ordinals = list_candidate_ordinals(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, %scenario_id, "failed to read candidate ordinals for a move");
            AppError::Internal {
                message: "failed to read this scenario's facts".to_string(),
            }
        })?;

    let mut out = Vec::new();
    for r in refs {
        // `flatten` in effect: a ref that is not included yields `None` and is
        // skipped, while an UNREADABLE one propagates. The two are different
        // answers and the `?` keeps them so.
        if let Some(fact) = decode_orderable(r, &ordinals, scenario_id)? {
            out.push(fact);
        }
    }
    Ok(out)
}

/// One reference row as an orderable fact, or `None` when it is not included.
///
/// ## Why both decodes are loud, and the skip is not
///
/// "This fact is not included" is a normal answer — the queue is full of
/// candidates nobody has ruled in, and they have no place in an order they do not
/// appear in. That is a `None`.
///
/// A status or tier token this build cannot READ is not an answer at all, it is a
/// data-integrity fault. Treating an unreadable status as "not included" would
/// silently drop a fact out of the list the human is rearranging; treating an
/// unreadable tier as `backup` would move it into the wrong block. Both refuse,
/// and both name the row and the scenario in the log (Standing Rule 1).
fn decode_orderable(
    r: crate::repositories::pipeline_repository::ScenarioFactRefRecord,
    ordinals: &std::collections::HashMap<String, i32>,
    scenario_id: uuid::Uuid,
) -> Result<Option<OrderableFact>, AppError> {
    let status = FactStatus::try_from(r.status.as_str()).map_err(|e| {
        tracing::error!(
            error = %e,
            graph_node_id = %r.graph_node_id,
            %scenario_id,
            "scenario_fact_refs carries a status token this build does not define"
        );
        AppError::Internal {
            message: "failed to read this scenario's facts".to_string(),
        }
    })?;
    if status != FactStatus::Included {
        return Ok(None);
    }

    let tier = FactTier::try_from(r.tier.as_str()).map_err(|e| {
        tracing::error!(
            error = %e,
            graph_node_id = %r.graph_node_id,
            %scenario_id,
            "scenario_fact_refs carries a tier token this build does not define"
        );
        AppError::Internal {
            message: "failed to read this scenario's facts".to_string(),
        }
    })?;

    Ok(Some(OrderableFact {
        code_ordinal: ordinals.get(&r.graph_node_id).copied(),
        graph_node_id: r.graph_node_id,
        sort_ordinal: r.sort_ordinal,
        tier,
    }))
}
