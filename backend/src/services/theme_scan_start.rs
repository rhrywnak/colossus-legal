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

use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use uuid::Uuid;

use crate::domain::llm_params::ResolvedLlmParams;
use crate::dto::ThemeScanSummary;
use crate::repositories::pipeline_repository::{
    fail_scan_run, finalize_scan_run_completed, insert_scan_run_stub, promote_scan_run_running,
    ScanRunFinal, ScanRunStart, ScanRunStub,
};
use crate::services::theme_scan::{
    load_scenario_fenced, prepare_scan, PrefilterSnapshot, PreparedScan, ScanPrompt,
    ThemeScanError, ValidatedScan,
};
use crate::services::theme_scan_judge::judge_all;
use crate::services::theme_scan_persist::{count_to_i32, persist_and_summarize, ScanRunMeta};
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
    let validated = validate_scan_request(state, record, requested_model_id.as_deref()).await?;

    // 4: from here on, a failure is visible in Run History.
    let run_id = Uuid::new_v4();
    insert_scan_run_stub(
        &state.pipeline_pool,
        &ScanRunStub {
            run_id,
            scenario_id,
            requested_model_id: requested_model_id.clone(),
            started_at: Utc::now(),
        },
    )
    .await
    .map_err(|source| ThemeScanError::ScanRunWriteFailed { run_id, source })?;

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

    // 6: promote and spawn.
    promote_run(state, run_id, &prepared, candidates_total, candidates_read).await?;
    spawn_scan_job(state, prepared, run_id, scenario_id, candidates_total);

    Ok(ScanStarted {
        run_id,
        candidates_total,
    })
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
async fn promote_run(
    state: &AppState,
    run_id: Uuid,
    prepared: &PreparedScan,
    candidates_total: i32,
    candidates_read: i32,
) -> Result<(), ThemeScanError> {
    let rows = promote_scan_run_running(
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

    if rows == 0 {
        return Err(ThemeScanError::ScanRunNotPromotable { run_id });
    }
    Ok(())
}

/// Spawn the judging task.
///
/// ## Rust Learning: `tokio::spawn` needs `Send + 'static`
///
/// The task outlives this function, so its future must own everything it uses
/// (`'static`) and be movable across threads (`Send`). `AppState` is `Clone`
/// (all Arc/pool fields — a clone is refcount bumps) and every field is
/// Send+Sync+'static; `PreparedScan` is likewise (Arc provider, Arc<str>, a
/// Copy params, owned Vec/String). So we clone `state` and MOVE both into the
/// task. The task's own errors are handled inside it (it must never leave the
/// row stuck `running`) — the `JoinHandle` is dropped deliberately.
fn spawn_scan_job(
    state: &AppState,
    prepared: PreparedScan,
    run_id: Uuid,
    scenario_id: Uuid,
    candidates_total: i32,
) {
    tracing::info!(
        %scenario_id, %run_id, model_id = %prepared.model_id,
        concurrency = prepared.concurrency, candidates_total,
        prompt_file = %prepared.prompt_file,
        "theme scan: started (background)"
    );
    let state = state.clone();
    tokio::spawn(async move { execute_scan_job(state, prepared, run_id, scenario_id).await });
}

/// The spawned judging task. Any failure marks the run `failed` with a reason —
/// it NEVER leaves the row stuck `running` (the startup sweep is the last-resort
/// guard, not the primary one).
async fn execute_scan_job(
    state: AppState,
    prepared: PreparedScan,
    run_id: Uuid,
    scenario_id: Uuid,
) {
    if let Err(e) = run_scan_job(&state, prepared, run_id, scenario_id).await {
        tracing::error!(%run_id, %scenario_id, error = %e, "theme scan: background job failed");
        if let Err(fe) = fail_scan_run(&state.pipeline_pool, run_id, &e).await {
            tracing::error!(%run_id, error = %fe,
                "theme scan: could not mark run failed (startup sweep will catch it)");
        }
    }
}

/// The fallible inner body: judge (with live progress) → persist → finalize.
/// Returns `Err(message)` on a completion-time failure so [`execute_scan_job`]
/// can mark the run `failed`.
async fn run_scan_job(
    state: &AppState,
    prepared: PreparedScan,
    run_id: Uuid,
    scenario_id: Uuid,
) -> Result<(), String> {
    let clock = Instant::now();
    let results = judge_all(
        Arc::clone(&prepared.provider),
        Arc::clone(&state.theme_scan_semaphore),
        prepared.concurrency,
        Arc::clone(&prepared.scan_prompt),
        Arc::clone(&prepared.scan_criteria),
        prepared.params,
        prepared.groups,
        state.pipeline_pool.clone(),
        run_id,
        // The automatic-retry cap, read once at startup (ruled 2026-08-28). The
        // scan judges hundreds of candidates concurrently, so a permissive cap
        // multiplies across every one of them — which is precisely why the
        // default is zero and why the value is not decided here.
        state.config.llm_retry_max,
    )
    .await;
    // millis fit i64 for any real scan; the impossible overflow caps (Standing Rule 1).
    let duration_ms = i64::try_from(clock.elapsed().as_millis()).unwrap_or(i64::MAX);

    let summary = persist_and_summarize(
        &state.pipeline_pool,
        ScanRunMeta {
            run_id,
            scenario_id,
            model_id: prepared.model_id,
            cost_per_input_token: prepared.cost_per_input_token,
            cost_per_output_token: prepared.cost_per_output_token,
            duration_ms,
            conservation: prepared.conservation,
        },
        results,
    )
    .await;

    let summary_json = serde_json::to_value(&summary)
        .map_err(|e| format!("failed to serialize scan summary: {e}"))?;
    let final_ = build_run_final(&summary, run_id, duration_ms, summary_json);
    finalize_scan_run_completed(&state.pipeline_pool, &final_)
        .await
        .map_err(|e| format!("failed to finalize scan run: {e}"))?;

    tracing::info!(
        %run_id, %scenario_id, candidates_read = summary.candidates_read,
        relevant = summary.relevant, irrelevant = summary.irrelevant,
        failed = summary.failed, duration_ms, "theme scan: complete"
    );
    Ok(())
}

/// Assemble the finalize record from the completed summary (narrowing the usize
/// counts to the `INTEGER` columns). Split out to keep [`run_scan_job`] under the
/// function-size limit.
fn build_run_final(
    summary: &ThemeScanSummary,
    run_id: Uuid,
    duration_ms: i64,
    summary_json: serde_json::Value,
) -> ScanRunFinal {
    ScanRunFinal {
        run_id,
        relevant_count: count_to_i32(summary.relevant, "relevant_count"),
        irrelevant_count: count_to_i32(summary.irrelevant, "irrelevant_count"),
        failed_count: count_to_i32(summary.failed, "failed_count"),
        input_tokens: summary.input_tokens,
        output_tokens: summary.output_tokens,
        computed_cost: summary.computed_cost,
        duration_ms,
        summary_json,
    }
}

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
