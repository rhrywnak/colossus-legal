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

use crate::domain::settings::Settings;
use crate::domain::wording_rehearsal::SectionState;
use crate::domain::wording_templates::render;
use crate::dto::rehearsal::{
    RehearsalAlways, RehearsalCollapse, RehearsalEditorWordingDto, RehearsalPayload,
    RehearsalScenario, RehearsalWordingDto,
};
use crate::repositories::pipeline_repository::{
    insert_status_transition, list_scenarios_for_case, update_scenario_status, PipelineRepoError,
    ScenarioRecord,
};
use crate::services::rehearsal_assembly::{assemble_scenario, AssemblyError};

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

    /// A rehearsal scenario could not be built from what is stored.
    ///
    /// Its own variant so the record store's outages and the pipeline database's
    /// stay apart in the log — see `assembly_to_readiness`.
    #[error("failed to build the rehearsal view: {detail}")]
    Assembly { detail: String },

    /// A stored configuration value became unusable after boot accepted it.
    ///
    /// Reachable only if the snapshot was swapped mid-request. Handled rather
    /// than unwrapped, because the value in question is the standing card, and a
    /// rehearsal page with an empty Always block is the one failure this surface
    /// cannot absorb.
    #[error("{source}")]
    Unusable {
        #[source]
        source: crate::domain::settings::SettingError,
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

/// Every READY scenario in a case, plus everything the page speaks with.
///
/// A drafted scenario is absent — that is the gate, and it is applied HERE rather
/// than in the browser so no client can ask for one. There is no parameter to ask
/// with, which is the point.
///
/// ## Why the whole case is served in one read
///
/// §10 is worked scenario by scenario in short sessions, but the LIST of what is
/// rehearsable is the case's. One read means moving between scenarios with the
/// keyboard is instant and needs no network — which is the difference between a
/// rehearsal surface and a data table. Task 2.11 B2 kept that property
/// deliberately: the per-scenario address selects within this payload rather than
/// fetching its own.
///
/// # Errors
/// Returns [`ReadinessError`] if a read fails or a stored value is unusable.
pub async fn rehearsal_payload(
    pool: &PgPool,
    graph: &neo4rs::Graph,
    case_slug: &str,
    settings: &Settings,
) -> Result<RehearsalPayload, ReadinessError> {
    let records = list_scenarios_for_case(pool, case_slug)
        .await
        .map_err(|source| ReadinessError::Read { source })?;

    let ready: Vec<&ScenarioRecord> = records.iter().filter(|r| is_ready(r)).collect();

    let mut scenarios = Vec::new();
    for record in &ready {
        scenarios.push(
            assemble_scenario(pool, graph, record, settings)
                .await
                // The scenario is named HERE because it is the only place that
                // knows it. A case with five ready scenarios otherwise returns one
                // 500 whose log says the slug and the cause but not WHICH of the
                // five failed, leaving an operator to test each one by hand.
                .map_err(|e| assembly_to_readiness(e, record.scenario_id))?,
        );
    }

    page_chrome(scenarios, settings)
}

/// The page's own words, its positions, and which sections start open.
///
/// Split from [`rehearsal_payload`] for the function-size limit, and the seam is
/// real: everything above it is a READ, and everything here is composition over
/// what was read.
///
/// # Errors
/// Returns [`ReadinessError::Unusable`] if a stored value that boot accepted has
/// since become unreadable.
fn page_chrome(
    scenarios: Vec<RehearsalScenario>,
    settings: &Settings,
) -> Result<RehearsalPayload, ReadinessError> {
    let w = &settings.rehearsal_wording;
    // The positions are composed here, one per scenario. A client holding
    // "Scenario {n} of {total}" would be composing prose out of two numbers, and
    // the sentence would then live half in the store and half in a component.
    let total = scenarios.len();
    let positions = (1..=total)
        .map(|n| {
            render(
                &w.position_template,
                &[
                    ("n", n.to_string().as_str()),
                    ("total", total.to_string().as_str()),
                ],
            )
        })
        .collect();

    // Both derived shapes were already parsed and REFUSED at boot if unusable, so
    // a failure here would mean the snapshot changed under us mid-request. It is
    // still handled rather than unwrapped: this page must never render a standing
    // card with no lines in it.
    let lines = w
        .always_lines()
        .map_err(|source| ReadinessError::Unusable { source })?;
    let states = w
        .section_states()
        .map_err(|source| ReadinessError::Unusable { source })?;

    Ok(RehearsalPayload {
        scenarios,
        positions,
        always: RehearsalAlways {
            heading: w.always_heading.clone(),
            lines,
        },
        wording: page_wording(settings),
        collapse: opening_states(&states),
    })
}

/// Which sections arrive open, from the five parsed state tokens.
///
/// ## Why the read is POSITIONAL, and why that is safe here
///
/// `section_states` returns the five in the order the page renders them — what →
/// accusation → timeline → points → watch — and pins that order with a test of
/// its own. Reading positionally means adding a sixth section is a compile error
/// here rather than a section that silently arrives shut; reading by key would
/// compile happily with one of them never looked up.
///
/// Split from [`page_chrome`] for the function-size limit; the seam is the one
/// place a copy-paste in this file would open the wrong section for a witness.
fn opening_states(states: &[(&'static str, SectionState); 5]) -> RehearsalCollapse {
    RehearsalCollapse {
        what_open: states[0].1.is_open(),
        accusation_open: states[1].1.is_open(),
        timeline_open: states[2].1.is_open(),
        points_open: states[3].1.is_open(),
        watch_for_open: states[4].1.is_open(),
    }
}

/// Every stored string the page renders that is not already a finished sentence.
///
/// The templates the server FILLS are absent by construction — there is no field
/// for the count template, a gap template, or either attribution template to
/// travel in, so a browser could not recompose one if it tried. That absence IS
/// the §10 exclusion, made structural rather than promised.
///
/// The ONE template that does cross is `save_failed_template`, whose `{detail}`
/// is the failure's own text — the single value that exists only on the client's
/// side. Same carve-out, and same reasoning, as the working view's.
fn page_wording(settings: &Settings) -> RehearsalWordingDto {
    let w = &settings.rehearsal_wording;
    let chrome = &settings.rehearsal_chrome_wording;
    let editing = &settings.accusation_wording;
    RehearsalWordingDto {
        answer_label: settings.accusation_wording.answer_label.clone(),
        page_heading: w.page_heading.clone(),
        purpose_line: w.purpose_line.clone(),
        previous_label: w.previous_label.clone(),
        next_label: w.next_label.clone(),
        nothing_ready_notice: w.nothing_ready_notice.clone(),
        picker_heading: w.picker_heading.clone(),
        not_ready_notice: w.not_ready_notice.clone(),
        expand_all_label: w.expand_all_label.clone(),
        collapse_all_label: w.collapse_all_label.clone(),
        block_what_heading: w.block_what_heading.clone(),
        block_accusation_heading: w.block_accusation_heading.clone(),
        block_timeline_heading: w.block_timeline_heading.clone(),
        block_points_heading: w.block_points_heading.clone(),
        block_watch_heading: w.block_watch_heading.clone(),
        crumb_trial_prep_label: chrome.crumb_trial_prep_label.clone(),
        scenario_page_label: chrome.scenario_page_label.clone(),
        go_to_row_label: chrome.go_to_row_label.clone(),
        prep_list_heading: chrome.prep_list_heading.clone(),
        row_open_hint: chrome.row_open_hint.clone(),
        add_point_label: chrome.add_point_label.clone(),
        add_watch_label: chrome.add_watch_label.clone(),
        points_authoring_note: chrome.points_authoring_note.clone(),
        // Ruling C3: borrowed from the working view's rows rather than seeded
        // again. "Edit", "Save", "Withdraw it" and "Cancel" are already stored
        // and already rehearsal-voiced; a second set would be four sentences
        // Roman has to keep in step by hand.
        editor: RehearsalEditorWordingDto {
            edit_label: editing.text_edit_label.clone(),
            save_label: editing.text_save_label.clone(),
            cancel_label: editing.text_cancel_label.clone(),
            withdraw_label: editing.text_clear_label.clone(),
            accusation_placeholder: editing.text_placeholder.clone(),
            what_placeholder: chrome.what_placeholder.clone(),
            save_failed_template: editing.save_failed_template.clone(),
        },
    }
}

/// Carry an assembly failure into this module's error, keeping the two record
/// stores apart.
///
/// A Postgres failure and a Neo4j failure are different outages with different
/// operators and different remedies. Collapsing them would produce one "failed to
/// load" that says which service is down to nobody.
fn assembly_to_readiness(error: AssemblyError, scenario_id: uuid::Uuid) -> ReadinessError {
    match error {
        AssemblyError::Read { source } => {
            tracing::error!(error = %source, %scenario_id, "failed to read a rehearsal scenario");
            ReadinessError::Read { source }
        }
        other => {
            tracing::error!(
                error = %other,
                %scenario_id,
                "failed to assemble a rehearsal scenario"
            );
            ReadinessError::Assembly {
                detail: other.to_string(),
            }
        }
    }
}

#[cfg(test)]
#[path = "scenario_readiness_tests.rs"]
mod tests;
