//! Theme Scan START path: the ordered, fail-loud beginning of a scan and the
//! background job it spawns (prepare → stub → promote → judge → finalize).
//!
//! Split from `theme_scan_run.rs` (module-size limit) when the start path grew
//! the stub-row lifecycle. The split is along a real seam: THIS module owns the
//! only code that CREATES a run, while `theme_scan_run.rs` owns everything done
//! to runs that already exist (poll, list, delete, merge).
//!
//! ## The order of operations is the feature (Standing Rule 1)
//!
//! Every step is placed by what it can prove and what it can leave behind:
//!
//! 1. **Read the judging prompt.** The cheapest failure, and one of the two
//!    causes of the eleven-day silence. No row is written for a scan that could
//!    never have run, and the caller is told which path is missing.
//! 2. **Fence the scenario.** Establishes that the scenario exists and belongs to
//!    this case — required before step 4 both because `scan_runs.scenario_id` is
//!    a foreign key and because writing a row keyed to another case's scenario
//!    would be a cross-case write.
//! 3. **Validate the request** (criterion, subject, model choice). Failures here
//!    are the caller's to fix and are answered with a 4xx and NO run row — see
//!    `validate_scan_request` in [`crate::services::theme_scan_validate`] for the
//!    ruling behind that (it is `pub(crate)`, hence named and not linked).
//! 4. **Insert the stub row, born `failed`.** From here on, dying leaves a
//!    record. Anything that goes wrong later updates the reason; nothing has to
//!    remember to create the row.
//! 5. **Prepare** (vLLM gate, candidate read). A failure writes its reason onto
//!    the stub and returns the HTTP error.
//! 6. **Promote to `running`** with the resolved model, params snapshot, and
//!    progress denominator, then spawn the judging task.
//!
//! Steps 3 and 5 are two halves of what used to be one `prepare_scan`, and the
//! seam between them is exactly the seam between "your request was wrong" and
//! "the system could not deliver". The row is what marks the difference.
//!
//! The preconditions all run SYNCHRONOUSLY, so a gate failure is an immediate
//! typed error the route returns as its HTTP status — never a background failure
//! the user must poll to discover.

use chrono::Utc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::domain::llm_params::ResolvedLlmParams;
use crate::repositories::pipeline_repository::{
    fail_scan_run, find_running_scan_run, insert_scan_run_stub, promote_scan_run_running,
    PromoteOutcome, ScanRunStart, ScanRunStub,
};
use crate::services::theme_scan::{
    load_scenario_fenced, prepare_scan, PrefilterSnapshot, PreparedScan, ScanPrompt,
    ThemeScanError, ValidatedScan,
};
use crate::services::theme_scan_job::spawn_scan_job;
use crate::services::theme_scan_persist::count_to_i32;
use crate::services::theme_scan_validate::{load_scan_prompt, validate_scan_request};
use crate::state::AppState;

/// The immediate result of starting a background scan.
pub struct ScanStarted {
    pub run_id: Uuid,
    pub candidates_total: i32,
}

/// Start a Theme Scan as a BACKGROUND job and return its handle immediately.
/// See the module doc for why the steps are in this order.
pub async fn start_theme_scan(
    state: &AppState,
    case_slug: &str,
    scenario_id: Uuid,
    requested_model_id: Option<String>,
) -> Result<ScanStarted, ThemeScanError> {
    // 1 + 2 + 3: everything that must be settled before a row exists. A failure
    // in any of them is answered with an HTTP status and nothing else — the
    // caller can see and fix all three (a missing prompt file is the operator's
    // deploy; the rest are the request's own contents).
    let prompt = load_scan_prompt(state)?;
    let record = load_scenario_fenced(&state.pipeline_pool, case_slug, scenario_id).await?;
    // 3b: is one already going? Placed AFTER the case fence — so a caller cannot
    // learn that another case's scenario is busy — and BEFORE the stub INSERT, so a
    // refused second scan leaves no row in Run History for a run that never was.
    refuse_if_already_running(state, scenario_id).await?;
    let validated = validate_scan_request(state, record, requested_model_id.as_deref()).await?;

    // 4: from here on, a failure is visible in Run History.
    let run_id = write_stub(state, scenario_id, requested_model_id).await?;

    // 5: prepare, recording any failure onto the stub before it propagates.
    let prepared = prepare_or_record(state, run_id, scenario_id, validated, prompt).await?;
    // TWO numbers since task 2.15 Tier 2, and they are no longer the same one.
    // `candidates_total` is the progress DENOMINATOR — what the judge will be
    // asked, after de-duplication and the pre-filter — so "43 of 124" counts the
    // work that is actually happening. `candidates_read` stays the POOL, because
    // the history's Candidates column and its +Δ delta measure how much evidence
    // exists about the subject, which pre-filtering does not change.
    let candidates_total = count_to_i32(prepared.groups.len(), "candidates_total");
    let candidates_read = count_to_i32(prepared.conservation.pool, "candidates_read");

    // 6: promote, register the stop handle, and spawn.
    //
    // The token is registered BEFORE the task starts, not inside it: registering
    // from within the spawned task would leave a window in which the POST has
    // already returned a run_id the browser can render a Stop button for, and the
    // cancel route would find no token and refuse. Ordering it here means "the
    // caller has a run_id" and "that run can be stopped" become true together.
    promote_run(
        state,
        run_id,
        &prepared,
        scenario_id,
        candidates_total,
        candidates_read,
    )
    .await?;
    let cancel = register_stop_handle(state, run_id).await;
    spawn_scan_job(
        state,
        prepared,
        run_id,
        scenario_id,
        candidates_total,
        cancel,
    );

    Ok(ScanStarted {
        run_id,
        candidates_total,
    })
}

/// Write the `failed` stub row and return the run id it was born with.
///
/// Split out to keep [`start_theme_scan`] within the function-size limit; the
/// step is step 4 of the module doc's six, and the line it draws is the important
/// one — before this call a failure returns an HTTP status and nothing else,
/// after it every failure is visible in Run History.
///
/// The id is minted HERE rather than passed in, so there is exactly one place a
/// run's identity comes into existence.
async fn write_stub(
    state: &AppState,
    scenario_id: Uuid,
    requested_model_id: Option<String>,
) -> Result<Uuid, ThemeScanError> {
    let run_id = Uuid::new_v4();
    insert_scan_run_stub(
        &state.pipeline_pool,
        &ScanRunStub {
            run_id,
            scenario_id,
            requested_model_id,
            started_at: Utc::now(),
        },
    )
    .await
    .map_err(|source| ThemeScanError::ScanRunWriteFailed { run_id, source })?;
    Ok(run_id)
}

/// Register the run's stop handle and hand back the token for the judging task.
///
/// Two owners of one token, deliberately: the MAP's copy is what the cancel route
/// finds, and the returned copy is what the judging loop watches. A
/// `CancellationToken` clone is a handle on the same token, not a copy of it, so
/// cancelling either cancels both — which is the entire mechanism.
///
/// Split out for the function-size limit, and the split earns its place by giving
/// the ordering a name: see the call site for why registration must complete
/// before the POST returns.
async fn register_stop_handle(state: &AppState, run_id: Uuid) -> CancellationToken {
    let cancel = CancellationToken::new();
    state
        .scan_cancel
        .lock()
        .await
        .insert(run_id, cancel.clone());
    cancel
}

/// Refuse a second scan of a scenario that already has one in flight.
///
/// The cheap, ordinary path to the 409 (part A2). The
/// `scan_runs_one_running_per_scenario` index is the backstop behind it — see
/// [`promote_run`] — but this check is what produces the refusal a human actually
/// meets, and it produces it BEFORE any row is written, any provider is built or
/// any candidate is read.
///
/// An unreadable check PROPAGATES rather than defaulting to "nothing is running":
/// treating a DB failure as permission would fail in the expensive direction, and
/// a scan is one metered call per candidate over a pool of hundreds.
async fn refuse_if_already_running(
    state: &AppState,
    scenario_id: Uuid,
) -> Result<(), ThemeScanError> {
    let running = find_running_scan_run(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|source| ThemeScanError::ScanRunInFlightCheckFailed {
            scenario_id,
            source,
        })?;

    if let Some(row) = running {
        tracing::info!(
            %scenario_id, run_id = %row.run_id, started_at = %row.started_at,
            "theme scan: refusing a second scan; one is already running on this scenario"
        );
        return Err(ThemeScanError::ScanAlreadyRunning {
            scenario_id,
            run_id: row.run_id,
        });
    }
    Ok(())
}

/// Run the preparation, writing its failure reason onto the stub row before
/// returning it.
///
/// The stub is already `failed`; this replaces its placeholder reason with the
/// real one, so Run History shows *why* rather than "we don't know". A failure of
/// the recording itself is logged and swallowed — deliberately: the caller is
/// already being handed a typed error that names the true problem, and replacing
/// it with a bookkeeping error would hide the diagnosis behind the diagnosis.
///
/// Everything reaching this function has already passed
/// [`validate_scan_request`], so every failure it can see is one the system owes
/// a record for — which is why this arm records unconditionally rather than
/// classifying the error it caught.
async fn prepare_or_record(
    state: &AppState,
    run_id: Uuid,
    scenario_id: Uuid,
    validated: ValidatedScan,
    prompt: ScanPrompt,
) -> Result<PreparedScan, ThemeScanError> {
    match prepare_scan(state, scenario_id, validated, prompt).await {
        Ok(prepared) => Ok(prepared),
        Err(e) => {
            tracing::error!(%run_id, error = %e, "theme scan: preparation failed; run recorded as failed");
            if let Err(fe) = fail_scan_run(&state.pipeline_pool, run_id, &e.to_string()).await {
                tracing::error!(%run_id, error = %fe, original = %e,
                    "theme scan: could not record the preparation failure on the run row");
            }
            Err(e)
        }
    }
}

/// Promote the stub to `running` and log the start.
///
/// A promotion that matches ZERO rows is a hard error, not a shrug: the row it
/// was meant to flip is gone or already terminal, so spawning the judging task
/// would spend real LLM budget on a run nothing can report on (Standing Rule 1).
///
/// ## Why this path does NOT write the reason onto the row
///
/// Every other failure in the start sequence records itself via `fail_scan_run`.
/// This one deliberately does not, because zero rows matched means exactly one of
/// two things and both forbid the write: the row is GONE (there is nothing to
/// write to — the update would touch zero rows in turn), or the row is no longer
/// in its birth state, in which case something else already owns its status and
/// overwriting it would destroy a truthful record with a guess. The full context
/// — run id, what failed — rides the returned typed error, which the route logs
/// before answering 500.
///
/// ## The third outcome: the index caught a race the pre-check could not
///
/// [`refuse_if_already_running`] reads, then this writes, and two POSTs a
/// millisecond apart can both pass the read. The
/// `scan_runs_one_running_per_scenario` partial unique index is what makes that
/// window harmless, and this arm is what makes it POLITE: the collision comes
/// back as the same `ScanAlreadyRunning` 409 the pre-check returns, never as a
/// 500 about a constraint the human has no way to understand.
///
/// The losing run's stub row is left as it was born — `failed`, carrying the
/// stub's own reason. That is honest: the run never started, and the row says a
/// scan did not finish starting, which is exactly what happened.
async fn promote_run(
    state: &AppState,
    run_id: Uuid,
    prepared: &PreparedScan,
    scenario_id: Uuid,
    candidates_total: i32,
    candidates_read: i32,
) -> Result<(), ThemeScanError> {
    let outcome = promote_scan_run_running(
        &state.pipeline_pool,
        &ScanRunStart {
            run_id,
            model_id: prepared.model_id.clone(),
            resolved_params: params_snapshot(
                &prepared.params,
                &prepared.prompt_file,
                &prepared.prefilter,
            ),
            candidates_total,
            candidates_read,
        },
    )
    .await
    .map_err(|source| ThemeScanError::ScanRunWriteFailed { run_id, source })?;

    match outcome {
        PromoteOutcome::Promoted => Ok(()),
        PromoteOutcome::NotPromotable => Err(ThemeScanError::ScanRunNotPromotable { run_id }),
        PromoteOutcome::AlreadyRunning => {
            // Re-read to name the run the caller should watch. A failure to
            // re-read is not allowed to become a 500 for a request whose answer is
            // already known to be "no": the index said so. But the browser needs an
            // id to adopt, so an unreadable re-read propagates as the check failure
            // it is, rather than as a 409 pointing at nothing.
            let winner = find_running_scan_run(&state.pipeline_pool, scenario_id)
                .await
                .map_err(|source| ThemeScanError::ScanRunInFlightCheckFailed {
                    scenario_id,
                    source,
                })?
                .ok_or(ThemeScanError::ScanRunNotPromotable { run_id })?;
            tracing::info!(
                %scenario_id, refused = %run_id, running = %winner.run_id,
                "theme scan: the one-running-per-scenario index refused a concurrent start"
            );
            Err(ThemeScanError::ScanAlreadyRunning {
                scenario_id,
                run_id: winner.run_id,
            })
        }
    }
}

// The BACKGROUND JOB — spawn, judge, finalize-or-cancel — moved to the sibling
// `theme_scan_job` when this module reached the 300-line limit (task
// SCAN_SERVER_STATE). The seam is the one the module doc already draws: steps 1-6
// above make a run EXIST and answer the POST; everything after the spawn happens
// with nobody waiting, and its failures are recorded on the row rather than
// returned. `spawn_scan_job` is the last line of the first story and the first of
// the second.

/// Serialize everything that DECIDED this run into the `scan_runs.resolved_params`
/// JSONB snapshot.
///
/// Three groups, and all three are settings a human can change between two runs:
///
/// * the LLM parameters — HOW each candidate was judged;
/// * `prompt_file` — WHAT the judge was told; the `theme_scan_prompt_file`
///   settings row as it stood when this run started (task 2.15 — it was an env
///   var with a compiled default before, and could not change between two runs);
/// * the pre-filter dials — WHICH candidates were put in front of it at all.
///
/// The last group is the 2.15 addition, and it closes a real gap: the conservation
/// block records that fifteen quotes were set aside for length, but not the
/// threshold that set them aside. Without both, an operator comparing two runs a
/// week apart cannot tell a prompt change from a settings change — the numbers
/// moved and nothing says which dial turned.
///
/// A SNAPSHOT, never a pointer at the mutable rows (design 5.9): an operator
/// editing a parameter between two benchmark runs would otherwise make them
/// incomparable. The column is JSONB (caller-owns-serialization), so growing this
/// shape needs no migration.
fn params_snapshot(
    p: &ResolvedLlmParams,
    prompt_file: &str,
    prefilter: &PrefilterSnapshot,
) -> serde_json::Value {
    serde_json::json!({
        "temperature": p.temperature,
        "timeout_secs": p.timeout_secs,
        "max_tokens": p.max_tokens,
        "prompt_file": prompt_file,
        "prefilter_min_chars": prefilter.min_chars,
        "prefilter_statement_types": prefilter.statement_types,
    })
}

#[cfg(test)]
#[path = "theme_scan_start_tests.rs"]
mod tests;
