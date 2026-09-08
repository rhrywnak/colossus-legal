//! The ruled facts the gathered pool does not reach (v2.1.2).
//!
//! One defect, one seam. `api::scenario_cards` builds its pool from
//! `all_evidence_about_subject` — every Evidence node the graph says is ABOUT a
//! scenario's target — and `assemble` only ever walks that pool. A human, though,
//! may include a fact that is about somebody else entirely, and that fact then had
//! a reference row, a card, and no place on any screen.
//!
//! Split out of the route module rather than living inside it for the ordinary
//! reason (Rule 17: `scenario_cards.rs` was at 273 non-comment lines and this
//! would have carried it past 300), and for one better one: the SELECTION is pure
//! and can be asserted without a graph, which is where the defect actually lives.
//!
//! ## CRITICAL — this module writes NOTHING
//!
//! It reads the graph and hands back rows. In particular it mints no candidate
//! ordinals: the card route's own doc comment promises it does not assign
//! identity, and that promise stays true. Identity is minted where it is created —
//! at the include, in `api::scenario_fact_include`.

use crate::bias::{dto::BiasInstance, repository::BiasRepository};
use crate::error::AppError;
use crate::repositories::pipeline_repository::ScenarioFactRefRecord;

/// Which of this scenario's ruled facts the gathered pool does not already carry.
///
/// Pure, and split from its caller for exactly that reason: the decision "is this
/// node missing from the pool?" is the whole of the defect, and a pure function of
/// two slices can be asserted in one unit test with no graph and no database.
///
/// Order is the refs' own (`tagged_at, graph_node_id` — see
/// `list_fact_refs_for_scenario`), so a scenario that hydrates twice hydrates in
/// the same order both times.
///
/// ## Rust Learning: `HashSet` from an iterator of `&str`
///
/// `collect()` into a `HashSet<&str>` borrows the pool's ids rather than cloning
/// them — the set lives only as long as this function, which is shorter than the
/// pool it borrows from, so the borrow checker accepts it and no allocation per id
/// happens. The `String`s that come OUT are cloned, because those the caller keeps.
pub(crate) fn refs_outside_pool(
    refs: &[ScenarioFactRefRecord],
    pool: &[BiasInstance],
) -> Vec<String> {
    let present: std::collections::HashSet<&str> =
        pool.iter().map(|c| c.evidence_id.as_str()).collect();
    refs.iter()
        .map(|r| r.graph_node_id.as_str())
        .filter(|id| !present.contains(id))
        .map(str::to_string)
        .collect()
}

/// Put this scenario's ruled facts back on the screen when the gather has stopped
/// reaching them.
///
/// ## Why: a ruled fact was rendered by nothing
///
/// The pool this route builds is `all_evidence_about_subject` — every Evidence
/// node the graph says is ABOUT the scenario's target. A human, though, may include
/// a fact that is about somebody else entirely: Phillips' own admission is *about
/// Phillips*, and it is still the best answer to an accusation against Marie. Such a
/// fact gets its `scenario_fact_refs` row and then appears in NO card list, because
/// `assemble` only ever walks the pool. Measured on DEV 2026-09-08: ten of S-1's
/// nineteen included facts were on no screen at all.
///
/// Appending them to the pool — before `node_ids` is computed — is the whole fix.
/// Everything downstream (card extras, page text, human links, summary overrides,
/// ordinals, the projection and `assemble` itself) is driven off `pool`/`node_ids`,
/// so an appended node is treated exactly like a gathered one: `Included` lands in
/// `pool`, `Dropped` lands in `set_aside`, and `backs_position` is untouched because
/// this route never writes a card field.
///
/// ## Domain note: a node the graph no longer has stays absent
///
/// `evidence_by_ids` returns only the nodes that exist, so a ref pointing at a
/// deleted node adds nothing here — which is correct. That state has an owner
/// already: `GET …/facts/orphans` reports it as an orphan, and duplicating it as a
/// blank card would give a dead pointer a place to hide.
///
/// ## And it mints NO ordinals
///
/// This route's own doc comment promises that it does not assign candidate
/// identity, and that promise stays true: an appended node with no ordinal serves
/// `code: null`, exactly as an un-numbered gathered candidate does. The mint moved
/// to the INCLUDE, where identity is actually created — see
/// `api::scenario_facts::apply_fact_action`.
///
/// ## Rust Learning: `async fn` returning `Result<(), AppError>`
///
/// Nothing is returned on success because the work is a mutation of the `&mut Vec`
/// the caller owns. The `?` in the caller therefore propagates only the failure —
/// and the failure is a real 500, not a silent skip: a graph that cannot answer
/// here would otherwise hide included facts, which is the exact defect being fixed.
///
/// # Errors
/// [`AppError::Internal`] when the graph read fails, logged with the scenario and
/// how many ids were being hydrated.
pub(crate) async fn append_refs_outside_pool(
    graph: neo4rs::Graph,
    scenario_id: uuid::Uuid,
    refs: &[ScenarioFactRefRecord],
    pool: &mut Vec<BiasInstance>,
) -> Result<(), AppError> {
    let missing = refs_outside_pool(refs, pool);
    if missing.is_empty() {
        return Ok(());
    }

    let hydrated = BiasRepository::new(graph)
        .evidence_by_ids(&missing)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e, %scenario_id, wanted = missing.len(),
                "failed to hydrate the ruled facts that sit outside the gathered pool"
            );
            // NOT the sibling's "failed to read candidate pool": that sentence
            // belongs to the GATHER read twenty lines above the call site, and two
            // different failures wearing one message is exactly what Standing Rule
            // 1 forbids. This one names the scenario and how many facts it was
            // hydrating, so a 500 in the browser is diagnosable without the log.
            AppError::Internal {
                message: format!(
                    "failed to read the {} ruled fact(s) that scenario {scenario_id} keeps \
                     outside its gathered pool; reload the cards panel to retry",
                    missing.len()
                ),
            }
        })?;

    // Two counts, not one: `wanted` minus `appended` is the number of refs whose
    // node the graph no longer has — the orphans — and collapsing them into one
    // number would make a dead pointer indistinguishable from a healthy hydrate.
    tracing::info!(
        %scenario_id,
        wanted = missing.len(),
        appended = hydrated.len(),
        "hydrated ruled facts that the gathered pool does not reach"
    );
    pool.extend(hydrated);
    Ok(())
}
