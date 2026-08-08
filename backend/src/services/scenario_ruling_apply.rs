//! Applying ONE ruling to ONE node: the anchored write, then its place in the
//! list.
//!
//! ## Why this is a service and not a helper inside the route
//!
//! A keystroke can settle more than one node. Ruling a folded card also rules the
//! byte-identical twins it covers (architect ruling R2), so the route resolves the
//! set and calls this once per member — which made the route module outgrow the
//! 300-line limit and, more to the point, made "what one ruling does" a thing
//! worth naming. Everything in here is the same for every member of a keystroke:
//! same verb, same status, same author, same provenance. Only the node changes.

use chrono::Utc;
use uuid::Uuid;

use crate::bias::repository::BiasRepository;
use crate::domain::{fact_status::FactStatus, ruling_anchor::RulingKind};
use crate::error::AppError;
use crate::repositories::pipeline_repository::assign_end_ordinal;
use crate::services::scenario_ruling::{record_ruling, RulingRequest};
use crate::state::AppState;

use crate::api::scenario_facts_mapping::ruling_error_to_app_error;

/// Everything one ruling needs beyond the node it lands on.
///
/// Grouped rather than passed as five more parameters because every member is
/// identical across the twins of one keystroke — a struct built once and reused
/// per target is also the statement that the twins receive the SAME ruling, which
/// five repeated arguments would only imply.
pub struct RulingFields<'a> {
    pub kind: RulingKind,
    pub status: FactStatus,
    pub ruled_by: &'a str,
    pub defer_reason: Option<&'a str>,
    pub source_run_id: Option<Uuid>,
}

/// Record one node's ruling: the anchored write, then its place in the list.
///
/// Called once per node a keystroke settles. Each target gets its OWN anchor,
/// captured from its own graph node — which is the whole point of ruling twins
/// individually rather than writing one row and calling the others covered: a
/// repeated sentence keeps its own document and page, so it still lands at each of
/// its appearances on the timeline.
///
/// ## What a mid-set failure looks like, and why it is not hidden
///
/// The nodes are ruled in order, the human's own card first, so a failure on a
/// twin always leaves the card they pressed a key on settled. The error propagates
/// — the queue's failure handler re-reads the pool, so the screen reconciles to
/// what the database actually holds rather than to what the optimistic advance
/// assumed — and the log names which node failed. Swallowing it would leave a twin
/// silently unruled, to return tomorrow as a fresh proposal (Standing Rule 1).
pub async fn rule_one(
    state: &AppState,
    scenario_id: Uuid,
    graph_node_id: &str,
    fields: RulingFields<'_>,
) -> Result<(), AppError> {
    // role / note = None on every action — see `apply_fact_action`'s doc: no scan
    // judgment should survive an include/drop/undrop ruling. The defer reason is
    // the one caller-supplied field, and `record_ruling` refuses it on any verb
    // but defer (and refuses a defer without one).
    let anchor_id = record_ruling(
        &state.pipeline_pool,
        &BiasRepository::new(state.graph.clone()),
        &RulingRequest {
            scenario_id,
            graph_node_id,
            kind: fields.kind,
            ruled_by: fields.ruled_by,
            role: None,
            note: None,
            defer_reason: fields.defer_reason,
            source_run_id: fields.source_run_id,
        },
        fields.status,
        Utc::now(),
    )
    .await
    .map_err(|e| ruling_error_to_app_error(e, graph_node_id))?;

    tracing::info!(
        anchor_id = %anchor_id,
        %graph_node_id,
        ruling = %fields.kind.code(),
        proposed_by = ?fields.source_run_id,
        "recorded an anchored ruling"
    );

    // Task 2.13c item 10: a fact the human just ruled IN belongs at the END of
    // the facts list, where they will see it. Leaving its ordinal NULL put it
    // wherever its C-number fell — on S-2 a re-included C-129 came back in fourth
    // place, which is the .378 acceptance FAIL.
    //
    // Only on INCLUDE, and only for a row with no ordinal yet: re-ruling a fact
    // somebody has already dragged must not rip it out of the place they put it
    // (§8 — a ruling never edits human augmentation).
    //
    // A failure here is logged and NOT propagated: the ruling itself is recorded
    // and ledgered, and refusing the whole request for a presentation detail
    // would tell the human their ruling failed when it did not. The honest
    // outcome is a stored ruling whose card sits in the old position, and a log
    // line naming it.
    if fields.status == FactStatus::Included {
        if let Err(e) = assign_end_ordinal(
            &state.pipeline_pool,
            scenario_id,
            graph_node_id,
            crate::services::scenario_fact_order::ORDINAL_STEP,
        )
        .await
        {
            tracing::error!(
                error = %e,
                %graph_node_id,
                %scenario_id,
                "ruled a fact in but could not place it at the end of the list; \
                 the ruling stands and the card keeps its C-ordinal position"
            );
        }
    }
    Ok(())
}
