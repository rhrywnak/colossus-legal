//! The ready gate and the rehearsal read path (task 1.5, v2 §5/§6/§10).
//!
//! Two jobs that belong together because one is the gate on the other:
//!
//! * **The gate.** No scenario enters rehearsal mode without a human declaring it
//!   ready (§5, mandatory). Both directions are human acts, recorded with the
//!   actor.
//! * **The read.** Ready scenarios only, assembled into the allowlisted
//!   `RehearsalScenario` — the four §10 blocks and nothing else.
//!
//! ## The gate has exactly one level: the SCENARIO
//!
//! Ruled 2026-08-01. §6's table has one `drafted ⇄ ready` transition and it is
//! scenario-level; §10 scopes rehearsal to "ready scenarios", not ready points.
//! Every talking point of a ready scenario renders. See `talking_points_of` for
//! why `scenario_responses.status` must never be added to that filter.

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::human_authored::{HumanFactKind, STANDING_CARD};
use crate::domain::scenario_code::scenario_code;
use crate::domain::settings::Settings;
use crate::dto::rehearsal::{RehearsalPayload, RehearsalPoint, RehearsalScenario};
use crate::repositories::pipeline_repository::{
    insert_status_transition, list_human_facts_for_scenario, list_items_for_response,
    list_responses_for_scenario, list_scenarios_for_case, update_scenario_status,
    PipelineRepoError, ScenarioRecord,
};

/// The status a scenario must hold to appear in rehearsal mode.
///
/// ## Domain note: one ready value, two drafted ones
///
/// The column's CHECK allows `draft | needs_evidence | ready`. v2's state machine
/// has `drafted` and `ready`, so `draft` and `needs_evidence` BOTH read as
/// drafted for gate purposes — measured 2026-08-01, `needs_evidence` has zero
/// live rows and nothing branches on it. Testing for `ready` rather than against
/// the two drafted values means a fourth status added later is excluded from
/// rehearsal by default, which is the safe direction: a scenario appears to a
/// witness only when someone deliberately said it should.
// CONST: mirrors the `scenarios.status` CHECK vocabulary — a schema-coupling
// invariant, not a deployment knob (same rationale as `ALLOWED_STATUSES` in
// `api::scenarios` and `DRAFTED_STATUS` below). Changing it requires a migration.
pub const READY_STATUS: &str = "ready";

/// Why a readiness change was refused.
#[derive(Debug, thiserror::Error)]
pub enum ReadinessError {
    #[error("that scenario is already {status} — nothing to change")]
    AlreadyInState { status: String },

    #[error("that scenario no longer exists — nothing was changed or recorded")]
    Vanished,

    #[error("failed to read the scenario: {source}")]
    Read {
        #[source]
        source: PipelineRepoError,
    },

    #[error("failed to record the readiness change: {source}")]
    Write {
        #[source]
        source: PipelineRepoError,
    },
}

/// The status a demotion writes back.
///
/// The column allows two drafted spellings; a demotion picks `draft`, the plain
/// one. `needs_evidence` is a claim about WHY a scenario is not ready, and taking
/// something out of rehearsal is not a statement about its evidence.
// CONST: mirrors the `scenarios.status` CHECK vocabulary — a schema-coupling
// invariant, not a deployment knob (same rationale as `ALLOWED_STATUSES`).
const DRAFTED_STATUS: &str = "draft";

/// Whether a scenario is visible to rehearsal mode.
pub fn is_ready(record: &ScenarioRecord) -> bool {
    record.status == READY_STATUS
}

/// Where a readiness change is heading, or why it is refused.
///
/// Pure, and split out of [`set_readiness`] so the no-op law is testable without
/// a database — and so the write function stays one readable transaction.
///
/// ## Rust Learning: returning `&'static str`
///
/// The two targets are string literals baked into the binary, so they live for
/// the whole program (`'static`) and can be returned by reference with nothing
/// allocated. The caller only needs to BIND the value to a query, not own it.
pub fn target_status(current: &str, ready: bool) -> Result<&'static str, ReadinessError> {
    let target = if ready { READY_STATUS } else { DRAFTED_STATUS };

    // A no-op is refused rather than recorded. Appending a transition that goes
    // nowhere would make the history read as though something happened — and the
    // table's CHECK would reject it anyway, as an opaque 500 instead of this.
    //
    // Both drafted spellings count as "already drafted": demoting a scenario
    // that is `needs_evidence` changes nothing a witness would see, so it is a
    // no-op even though the token differs.
    let unchanged = if ready {
        current == READY_STATUS
    } else {
        current != READY_STATUS
    };
    if unchanged {
        return Err(ReadinessError::AlreadyInState {
            status: current.to_string(),
        });
    }
    Ok(target)
}

/// Declare a scenario ready, or take it back out of rehearsal.
///
/// Returns the status it moved to. Both writes — the status change and its
/// record — commit in ONE transaction: a scenario that became ready with no
/// record of who did it, or a record of a change that did not land, are each
/// worse than neither.
///
/// ## Why this is not the generic `PUT /scenarios/:id`
///
/// That route records no actor, so setting `ready` through it would make the
/// scenario visible to a witness with nobody's name against the decision. As of
/// task 1.5 the PUT refuses `status` outright and points here.
///
/// # Errors
/// Returns [`ReadinessError`] if the scenario is already in the target state or
/// a write fails.
pub async fn set_readiness(
    pool: &PgPool,
    record: &ScenarioRecord,
    ready: bool,
    actor: &str,
) -> Result<String, ReadinessError> {
    let target = target_status(&record.status, ready)?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| ReadinessError::Write { source: e.into() })?;

    let changed = update_scenario_status(&mut *tx, record.scenario_id, target)
        .await
        .map_err(|source| ReadinessError::Write { source })?;

    // Zero rows means the scenario vanished between the caller's read and this
    // write. Recording a transition for a row that no longer exists would put a
    // fiction in the forensic record, so the whole thing is abandoned — the
    // transaction is dropped without a commit, which rolls it back.
    if changed == 0 {
        tracing::error!(
            scenario_id = %record.scenario_id,
            %actor,
            "the scenario disappeared during a readiness change; nothing recorded"
        );
        return Err(ReadinessError::Vanished);
    }

    insert_status_transition(
        &mut *tx,
        record.scenario_id,
        &record.status,
        target,
        actor,
        Utc::now(),
    )
    .await
    .map_err(|source| ReadinessError::Write { source })?;

    tx.commit()
        .await
        .map_err(|e| ReadinessError::Write { source: e.into() })?;

    Ok(target.to_string())
}

/// Read one scenario's talking points, ordered and capped.
///
/// ## Why `scenario_responses.status` is NOT consulted — read before changing
///
/// That column is `draft | ready`, and every 1.4 write path sets it to `'draft'`.
/// Filtering rehearsal on it would show an empty page forever, with no error —
/// the silent-empty failure this codebase keeps removing.
///
/// It is also wrong in principle. RULED 2026-08-01: the scenario's readiness is
/// the ONLY gate. §6 has one `drafted ⇄ ready` transition and it is
/// scenario-level; §10 scopes rehearsal to "ready scenarios", not ready points.
/// The column is vestigial. If a per-point editorial gate is ever wanted it
/// arrives as its own ratified change — not by adding a condition here.
async fn talking_points_of(
    pool: &PgPool,
    scenario_id: Uuid,
    settings: &Settings,
) -> Result<Vec<RehearsalPoint>, ReadinessError> {
    let responses = list_responses_for_scenario(pool, scenario_id)
        .await
        .map_err(|source| ReadinessError::Read { source })?;

    let Some(response) = responses.first() else {
        return Ok(Vec::new());
    };

    let items = list_items_for_response(pool, response.id)
        .await
        .map_err(|source| ReadinessError::Read { source })?;

    Ok(items
        .into_iter()
        // The cap is a display law as well as a write law: a list that grew past
        // it (through a direct write, or a lowered cap) must still rehearse as
        // the few points a witness can hold.
        .take(settings.talking_points_cap)
        .map(|item| RehearsalPoint {
            text: item.text,
            // Pairing has no authoring surface yet (tracker 3.9), so this is
            // always `None` today. Deriving a label from the record instead would
            // put words in the witness's mouth and drag pinpoint sourcing into a
            // payload the exclusion law keeps it out of.
            exhibit: None,
        })
        .collect())
}

/// Assemble the four §10 blocks for one ready scenario.
async fn rehearsal_scenario(
    pool: &PgPool,
    record: &ScenarioRecord,
    settings: &Settings,
) -> Result<RehearsalScenario, ReadinessError> {
    let notes = list_human_facts_for_scenario(pool, record.scenario_id)
        .await
        .map_err(|source| ReadinessError::Read { source })?;

    Ok(RehearsalScenario {
        code: scenario_code(record.code_ordinal),
        theme: record.theme_statement.clone(),
        // The attack in plain words, read from the authored definition. Note what
        // is NOT taken from that record: `motivation` sits right beside
        // `theme_statement` on the same struct and is excluded by §10 — the type
        // this maps into has no field for it.
        attack: record
            .definition
            .get("attack_text")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        points: talking_points_of(pool, record.scenario_id, settings).await?,
        watch_list: notes
            .into_iter()
            .filter(|n| n.kind == HumanFactKind::WatchList.code())
            .map(|n| n.text)
            .collect(),
    })
}

/// Every READY scenario in a case, plus the standing card.
///
/// A drafted scenario is absent — that is the gate, and it is applied here rather
/// than in the browser so no client can ask for one.
///
/// # Errors
/// Returns [`ReadinessError`] if a read fails.
pub async fn rehearsal_payload(
    pool: &PgPool,
    case_slug: &str,
    settings: &Settings,
) -> Result<RehearsalPayload, ReadinessError> {
    let records = list_scenarios_for_case(pool, case_slug)
        .await
        .map_err(|source| ReadinessError::Read { source })?;

    let mut scenarios = Vec::new();
    for record in records.iter().filter(|r| is_ready(r)) {
        scenarios.push(rehearsal_scenario(pool, record, settings).await?);
    }

    Ok(RehearsalPayload {
        scenarios,
        standing_card: STANDING_CARD.iter().map(|s| (*s).to_string()).collect(),
    })
}

#[cfg(test)]
#[path = "scenario_readiness_tests.rs"]
mod tests;
