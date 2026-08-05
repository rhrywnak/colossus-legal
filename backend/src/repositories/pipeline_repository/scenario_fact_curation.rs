//! The two curation writes task 2.13 adds: a fact's WEIGHT and its PLACE.
//!
//! Split out of `scenario_store` rather than added to it: that module is already
//! well past the 300-line limit (Rule 17), and these two writes are a coherent
//! pair with a job of their own — they record a human's *presentation judgment*
//! about a fact they have already ruled in, which is a different act from the
//! ruling itself.
//!
//! ## Why these are UPDATEs and never upserts
//!
//! `upsert_fact_ref` creates the reference; these two only ever modify one that
//! already exists. That is not an implementation convenience, it is the rule:
//! weighting or placing a fact that is NOT in the scenario is a request about
//! something that isn't there, and it must fail loudly rather than quietly
//! creating a half-formed row with a tier but no ruling (Standing Rule 1). Both
//! functions therefore return the affected row count so the handler can tell
//! "stored" from "there was nothing to store it on" and answer 404 rather than
//! 200.
//!
//! ## §8 invariant: these never trigger, and never survive, a re-gather
//!
//! Neither column appears in `upsert_fact_ref`'s INSERT column list or its
//! `ON CONFLICT … DO UPDATE SET` list, and neither appears in the merge
//! statement's. That omission is load-bearing in both directions: a newly created
//! reference takes the column DEFAULTs (`backup`, NULL — the honest "nobody has
//! weighed or placed this"), and an existing reference PRESERVES the human's tier
//! and position when a later include / drop / merge rewrites its status. Starring
//! never re-gathers; re-gathering never un-stars.
//!
//! ## CRITICAL — the pipeline pool
//!
//! `scenario_fact_refs` lives in `colossus_legal_v2`: these take
//! `&state.pipeline_pool`, never `state.pg_pool`.

use crate::domain::fact_tier::FactTier;

use super::PipelineRepoError;

/// Record how much one fact carries one scenario.
///
/// ## Rust Learning: a typed `FactTier` in, a raw token out
///
/// The parameter is the ENUM, not a `&str`. A caller cannot pass `"carrys"`,
/// because that is not a variant — the typo is a compile error rather than a row
/// nothing can read back. The SQL binds [`FactTier::code`], exactly the pattern
/// `upsert_fact_ref` uses for `FactStatus::code()`. The column is plain `TEXT`
/// validated in code, deliberately not a DB `CHECK` (see the migration).
///
/// Returns the number of rows updated: `1` when the fact is in the scenario and
/// `0` when it is not. The caller MUST branch on this — see the module doc.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the statement fails to execute.
pub async fn set_fact_tier(
    executor: impl sqlx::PgExecutor<'_>,
    scenario_id: uuid::Uuid,
    graph_node_id: &str,
    tier: FactTier,
    actor: &str,
) -> Result<u64, PipelineRepoError> {
    // `tagged_at` is deliberately NOT advanced. That column records when the fact
    // was tagged into the scenario, and it is the tie-breaker the reference list
    // is ordered by; touching it here would silently reshuffle the unplaced tail
    // of the facts list every time somebody starred something.
    // The actor and the moment are written in the SAME statement as the value
    // they describe (task 2.13c). Two statements could leave a weight stored with
    // nobody against it if the second failed — which is the state this column was
    // added to abolish.
    let result = sqlx::query(
        r#"UPDATE scenario_fact_refs
              SET tier = $3, tier_updated_by = $4, tier_updated_at = NOW()
            WHERE scenario_id = $1 AND graph_node_id = $2"#,
    )
    .bind(scenario_id)
    .bind(graph_node_id)
    .bind(tier.code())
    .bind(actor)
    .execute(executor)
    .await?;
    Ok(result.rows_affected())
}

/// Record where one fact sits in one scenario's order.
///
/// The ordinal is computed by [`crate::services::scenario_fact_order::plan_move`]
/// from the fact's new neighbours — this layer stores the number it is given and
/// forms no opinion about it. Keeping the arithmetic in the service and the
/// storage here is what lets the placement rules be unit-tested with no database
/// (Rule 12's spirit: the judgment lives in one place, and it is not the browser).
///
/// ONE row, always. There is no whole-list renumber anywhere in this module — the
/// sparse ordinal scheme exists precisely so that moving one card cannot rewrite
/// another card's stored position (the signed design's first law: every card
/// independent).
///
/// Returns the number of rows updated, with the same contract as
/// [`set_fact_tier`]: `0` means the fact is not in this scenario.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the statement fails to execute.
pub async fn set_fact_sort_ordinal(
    executor: impl sqlx::PgExecutor<'_>,
    scenario_id: uuid::Uuid,
    graph_node_id: &str,
    sort_ordinal: i32,
    actor: &str,
) -> Result<u64, PipelineRepoError> {
    let result = sqlx::query(
        r#"UPDATE scenario_fact_refs
              SET sort_ordinal = $3, order_updated_by = $4, order_updated_at = NOW()
            WHERE scenario_id = $1 AND graph_node_id = $2"#,
    )
    .bind(scenario_id)
    .bind(graph_node_id)
    .bind(sort_ordinal)
    .bind(actor)
    .execute(executor)
    .await?;
    Ok(result.rows_affected())
}

/// Forget where a human placed one fact, returning it to the list's natural order.
///
/// ## Why un-placing needs a route of its own
///
/// Before task 2.13c a drag was PERMANENT: `sort_ordinal` could be overwritten
/// but never cleared, so the only way to undo a placement was to remove the fact
/// from the scenario entirely — a ruling, ledgered, with side effects — in order
/// to undo a presentation judgment. That is a punishment for using the feature.
///
/// The attribution is cleared WITH the ordinal, deliberately: `order_updated_by`
/// answers "who placed this", and a row with no placement has no such person. A
/// name left behind pointing at a position that no longer exists is a false
/// record, which is the failure the column was added to prevent.
///
/// Returns the affected row count with the same contract as its siblings: `0`
/// means the fact is not in this scenario.
///
/// # Errors
/// Returns [`PipelineRepoError`] if the statement fails to execute.
pub async fn clear_fact_sort_ordinal(
    executor: impl sqlx::PgExecutor<'_>,
    scenario_id: uuid::Uuid,
    graph_node_id: &str,
) -> Result<u64, PipelineRepoError> {
    let result = sqlx::query(
        r#"UPDATE scenario_fact_refs
              SET sort_ordinal = NULL, order_updated_by = NULL, order_updated_at = NULL
            WHERE scenario_id = $1 AND graph_node_id = $2"#,
    )
    .bind(scenario_id)
    .bind(graph_node_id)
    .execute(executor)
    .await?;
    Ok(result.rows_affected())
}

/// Give a newly included fact a position at the END of the scenario's list.
///
/// ## Domain note: why "end of list" needs arithmetic, not NULL
///
/// The signed design says a newly included fact lands at the end. Leaving its
/// ordinal NULL does NOT achieve that — an unplaced fact sorts into the tail in
/// C-ordinal order, which is wherever its number happens to fall, and on S-2 that
/// put a freshly re-included C-129 back in fourth place (the .378 acceptance
/// FAIL). The human ruled it in a second ago and it appeared in the middle of
/// forty-six cards.
///
/// So the ordinal must exceed every number the list can currently show — both the
/// stored ones and the PROVISIONAL ones `services::scenario_fact_order` derives
/// for the unplaced tail (`ceiling + i * STEP` for the i-th unplaced fact). One
/// past the largest of those is `ceiling + (unplaced_count + 1) * STEP`, which is
/// what the statement below computes in a single pass.
///
/// Only rows with NO ordinal are touched: re-ruling a fact somebody has already
/// dragged must not rip it out of the place they put it (§8 — re-gathering never
/// edits human augmentation).
///
/// # Errors
/// Returns [`PipelineRepoError`] if the statement fails to execute.
pub async fn assign_end_ordinal(
    executor: impl sqlx::PgExecutor<'_>,
    scenario_id: uuid::Uuid,
    graph_node_id: &str,
    step: i32,
) -> Result<u64, PipelineRepoError> {
    let result = sqlx::query(
        r#"UPDATE scenario_fact_refs SET sort_ordinal = (
               SELECT COALESCE(MAX(sort_ordinal), 0)
                    + (COUNT(*) FILTER (WHERE sort_ordinal IS NULL) + 1) * $3
                 FROM scenario_fact_refs
                WHERE scenario_id = $1 AND status = 'included'
           )
           WHERE scenario_id = $1 AND graph_node_id = $2 AND sort_ordinal IS NULL"#,
    )
    .bind(scenario_id)
    .bind(graph_node_id)
    .bind(step)
    .execute(executor)
    .await?;
    Ok(result.rows_affected())
}
