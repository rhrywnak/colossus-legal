//! The projection's READS: what a completed run proposes to a scenario's queue,
//! and what ruling one of those cards settles.
//!
//! Two callers, one subject. The cards route asks it for a WHOLE POOL ("what is
//! proposed here?"); the ruling route asks it for ONE NODE ("is this a proposal,
//! and what does ruling it settle?"). Both answers come from the same two reads
//! and the same pure fold in `scenario_card_projection`, which is why they live
//! together rather than one of them living beside a handler — two copies of "read
//! the projecting run, then fold" would be two chances to disagree about what the
//! queue is showing.
//!
//! ## Why the ruling path re-resolves instead of trusting the payload
//!
//! The card the human clicked knows both answers already, and sending them back
//! would cost nothing. It is still wrong, for the reason `scenario_ruling`'s
//! module doc gives about anchors: a value supplied by the caller records what the
//! caller CLAIMED. A tab left open across a re-scan would attribute today's ruling
//! to yesterday's run; a covered-twin list sent by a client would let a caller
//! rule nodes it was never shown. Both are unforgeable when the server re-derives
//! them from `scan_run_verdicts` at the moment of the write (architect rulings R2
//! and R4).
//!
//! The cost is one indexed Postgres read plus one targeted graph read per ruling —
//! on a human keystroke, against a ~30-verdict run.
//!
//! ## What it returns, and what the caller does with it
//!
//! [`ProposedRuling::run_id`] becomes `scenario_fact_refs.source_run_id`, closing
//! the measured NULL-provenance hole for every future ruling.
//! [`ProposedRuling::covers`] is every node the one keystroke settles — the card
//! itself plus any byte-identical twin folded into it — and each gets its own
//! reference row and its own ledger anchor, so the repeated sentence keeps its own
//! document and page. Only the human's keystroke is de-duplicated; the record is
//! not.

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::bias::dto::BiasInstance;
use crate::bias::repository::BiasRepository;
use crate::error::AppError;
use crate::repositories::pipeline_repository::{
    fetch_projecting_run, list_fact_refs_for_scenario, list_relevant_verdicts_for_run,
    ProjectingRunRow,
};
use crate::services::scenario_card::CardRefState;
use crate::services::scenario_card_projection::{index_by_covered_node, project, ProposalGroup};
use crate::state::AppState;

/// A proposal the human is about to rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedRuling {
    /// The completed run that proposed it — recorded as the ruling's provenance.
    pub run_id: Uuid,
    /// Every node this ruling settles, the ruled card first.
    pub covers: Vec<String>,
}

/// What ruling `graph_node_id` settles, or `None` when it is not a proposal.
///
/// `None` is the ordinary answer for most rulings and is not a failure: a human
/// picking a raw candidate out of the full pool, re-ruling something they ruled
/// last week, or working a scenario nothing has scanned all land here. The caller
/// then rules exactly the one node it was given, with no provenance — which is the
/// behaviour that existed before the projection.
///
/// # Errors
/// Returns [`AppError::Internal`] if any read fails. Deliberately NOT degraded to
/// `None`: guessing "not a proposal" would silently drop the provenance this
/// exists to record and silently leave a twin unruled, and both failures would
/// look exactly like the ordinary case (Standing Rule 1).
pub async fn resolve_proposed_ruling(
    state: &AppState,
    scenario_id: Uuid,
    graph_node_id: &str,
) -> Result<Option<ProposedRuling>, AppError> {
    let Some(run) = fetch_projecting_run(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| read_failed(e, "the projecting scan run", scenario_id))?
    else {
        // Nothing has completed for this scenario, so nothing can be a proposal.
        return Ok(None);
    };

    let verdicts = list_relevant_verdicts_for_run(&state.pipeline_pool, run.run_id)
        .await
        .map_err(|e| read_failed(e, "the run's admitted verdicts", scenario_id))?;
    if !verdicts.iter().any(|v| v.graph_node_id == graph_node_id) {
        // The run judged this node irrelevant, or never saw it. Either way the card
        // in front of the human was not a proposal, and one read has answered it
        // before the two more expensive ones below.
        return Ok(None);
    }

    let refs = list_fact_refs_for_scenario(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| read_failed(e, "the scenario's fact references", scenario_id))?;
    let ruled: HashSet<String> = refs.into_iter().map(|r| r.graph_node_id).collect();

    let quotes = quotes_for(
        state,
        &verdicts
            .iter()
            .map(|v| v.graph_node_id.clone())
            .collect::<Vec<_>>(),
        scenario_id,
        run.run_id,
    )
    .await?;

    // The ordinal index is deliberately EMPTY here. It decides only which member of
    // a group carries the card — a display question, already settled on the screen
    // the human is looking at — and never which nodes are in the group. Reading the
    // ordinals would be a fourth query per keystroke to re-choose something this
    // caller does not use.
    let groups = project(&verdicts, &quotes, &ruled, &HashMap::new());
    let Some(group) = index_by_covered_node(&groups).get(graph_node_id).copied() else {
        // A verdict exists but precedence dropped it: the human has already ruled
        // this node, and is re-ruling it. No provenance is invented for that —
        // whatever the row already carries is preserved by the upsert.
        return Ok(None);
    };

    Ok(Some(ProposedRuling {
        run_id: run.run_id,
        covers: ruled_card_first(graph_node_id, &group.covers),
    }))
}

/// The nodes one keystroke settles, with the card the human actually pressed a
/// key on FIRST.
///
/// ## Why the order is a rule and not a preference
///
/// The caller rules these in sequence, each in its own transaction (a ruling
/// captures its own anchor from its own graph node — that is what keeps a repeated
/// sentence's document and page intact). So a failure part-way through the set
/// leaves an incomplete result, and WHICH part completed is the difference between
/// "your ruling landed and a twin did not" and "your ruling did nothing". Putting
/// the human's own card first makes the first outcome the only reachable one.
///
/// Extracted from [`resolve_proposed_ruling`] so the rule is testable: that
/// function's other decisions are all I/O, and this one is not.
///
/// The group is a projection of the run's verdicts and the ruled node is a member
/// of it by construction (it is how the group was found) — but the filter does not
/// assume that. A node absent from `covers` still comes back first and alone,
/// which is the honest degradation: rule what the human pressed.
fn ruled_card_first(graph_node_id: &str, covers: &[String]) -> Vec<String> {
    let mut ordered = vec![graph_node_id.to_string()];
    ordered.extend(
        covers
            .iter()
            .filter(|node| node.as_str() != graph_node_id)
            .cloned(),
    );
    ordered
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nodes(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn the_card_the_human_pressed_is_ruled_first() {
        // The twins are ruled in sequence, each in its own transaction. If the set
        // fails part-way, the human's own card must already be settled — otherwise
        // they pressed I, saw an error, and their card is still unruled while a
        // twin they never looked at is included.
        let ordered = ruled_card_first("node-46", &nodes(&["node-45", "node-46"]));

        assert_eq!(ordered, nodes(&["node-46", "node-45"]));
    }

    #[test]
    fn every_covered_node_is_settled_exactly_once() {
        // The representative must not be ruled twice when it IS the pressed card —
        // a second ruling would append a second ledger anchor for one human act,
        // and the ledger's value rests on every row being a real, distinct act.
        let ordered = ruled_card_first("node-45", &nodes(&["node-45", "node-46"]));

        assert_eq!(ordered, nodes(&["node-45", "node-46"]));
        assert_eq!(ordered.len(), 2, "no duplicate of the pressed card");
    }

    #[test]
    fn a_card_that_covers_nothing_rules_only_itself() {
        // The ordinary case: most proposals speak for one node.
        assert_eq!(
            ruled_card_first("ev-1", &nodes(&["ev-1"])),
            nodes(&["ev-1"])
        );
    }
}

/// The verbatim quotes of the run's judged nodes — the fold key.
///
/// A targeted `evidence_by_ids` rather than the whole subject pool: the ruling
/// path needs exactly the nodes this run judged, which is ~30 ids, and reading the
/// 148-row pool to answer a question about 30 of them is work nobody uses.
///
/// A node the graph no longer holds simply has no entry, so it cannot be folded
/// with anything — the same treatment `project` gives a quote-less verdict, and
/// the honest one: an item that has vanished must not silently drag another card
/// into a ruling.
/// ## Why the log carries the scenario AND the run
///
/// A node count alone is not a diagnosis. This read happens on a human keystroke,
/// on one of several scenarios a curator may have open, against one particular
/// run's verdict set — and the failure means their ruling was refused. An operator
/// reading "failed to read the judged quotes, nodes = 30" has no path back to
/// which queue broke without running SQL, which is the shape Standing Rule 1
/// exists to prevent. Both ids are in scope at the call site; they are passed in
/// rather than re-derived.
async fn quotes_for(
    state: &AppState,
    node_ids: &[String],
    scenario_id: Uuid,
    run_id: Uuid,
) -> Result<HashMap<String, String>, AppError> {
    let instances = BiasRepository::new(state.graph.clone())
        .evidence_by_ids(node_ids)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                %scenario_id,
                %run_id,
                nodes = node_ids.len(),
                "failed to read the judged quotes while resolving a proposed ruling; \
                 the ruling was refused"
            );
            AppError::Internal {
                message: "failed to read the candidate's text".to_string(),
            }
        })?;

    Ok(instances
        .into_iter()
        .filter_map(|i| i.verbatim_quote.map(|q| (i.evidence_id, q)))
        .collect())
}

/// One read failure, named where it happened.
///
/// Split out because three call sites report the same class of fault and a
/// message that could not say WHICH read failed would leave an operator with
/// three candidates and no way to choose between them (Standing Rule 1).
fn read_failed(
    error: crate::repositories::pipeline_repository::PipelineRepoError,
    what: &str,
    scenario_id: Uuid,
) -> AppError {
    tracing::error!(%error, %scenario_id,
        "failed to read {what} while resolving a proposed ruling");
    AppError::Internal {
        message: "failed to read the scan results behind this candidate".to_string(),
    }
}

/// The completed run whose verdicts this queue is showing, or `None`.
///
/// ## Why a read failure is fatal to the request
///
/// The same reasoning as the human links below, applied to the other direction.
/// Degrading to "no proposals" would render a scanned scenario as an unscanned
/// one — the precise false claim task 2.15 piece 3 removed from this page — and it
/// would be indistinguishable from a scenario whose picks are all ruled. An
/// unreadable run history is a real failure and says so (Standing Rule 1).
pub(crate) async fn load_projecting_run(
    state: &AppState,
    scenario_id: uuid::Uuid,
) -> Result<Option<ProjectingRunRow>, AppError> {
    fetch_projecting_run(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, %scenario_id,
                "failed to read the projecting scan run for the candidate cards");
            AppError::Internal {
                message: "failed to read the scenario's scan results".to_string(),
            }
        })
}

/// Fold one run's admitted verdicts into the proposals this pool should show.
///
/// The reads are one query; everything after is the pure projection. Precedence
/// (R-a) is applied by handing `project` the set of nodes that HAVE a reference
/// row — the presence of a row, not its status, which is what closes the diagnostic
/// §6 defer gap.
pub(crate) async fn project_run(
    state: &AppState,
    run: &ProjectingRunRow,
    pool: &[BiasInstance],
    ref_states: &HashMap<String, CardRefState>,
    ordinals: &HashMap<String, i32>,
) -> Result<Vec<ProposalGroup>, AppError> {
    let verdicts = list_relevant_verdicts_for_run(&state.pipeline_pool, run.run_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, run_id = %run.run_id,
                "failed to read the admitted verdicts for the candidate cards");
            AppError::Internal {
                message: "failed to read the scan's results".to_string(),
            }
        })?;

    // The fold key is the quote text, which lives in the graph — so it comes from
    // the pool already read for the cards rather than from a second query.
    let quotes: HashMap<String, String> = pool
        .iter()
        .filter_map(|c| {
            c.verbatim_quote
                .as_ref()
                .map(|q| (c.evidence_id.clone(), q.clone()))
        })
        .collect();
    let ruled: std::collections::HashSet<String> = ref_states.keys().cloned().collect();

    let groups = project(&verdicts, &quotes, &ruled, ordinals);
    tracing::info!(
        run_id = %run.run_id,
        admitted = verdicts.len(),
        proposed = groups.len(),
        already_ruled = verdicts.len() - groups.iter().map(|g| g.covers.len()).sum::<usize>(),
        "projected a completed scan run onto the candidate queue"
    );
    Ok(groups)
}
