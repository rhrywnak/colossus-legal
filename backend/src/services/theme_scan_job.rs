//! The Theme Scan BACKGROUND JOB: what happens after the POST has answered.
//!
//! Split from `theme_scan_start.rs` (module-size limit) along the seam that
//! module's own doc draws. Its six ordered steps make a run EXIST and hand the
//! caller a `run_id`; everything here runs with nobody waiting, so a failure has
//! no HTTP response to travel in and is recorded on the `scan_runs` row instead.
//!
//! ## The three ways a job ends, and the one rule they share
//!
//! * **completed** — the judging loop returned with the token un-cancelled:
//!   persist the verdicts, summarize, and write the authoritative final counts.
//! * **cancelled** — a human pressed Stop: record the status, keep the counts and
//!   the verdicts exactly as the loop left them, write no summary and no error.
//! * **failed** — anything went wrong: `fail_scan_run` writes the reason.
//!
//! The rule all three share is that the row NEVER stays `running`. The startup
//! sweep exists as the last-resort guard for a process that dies mid-scan, not as
//! the ordinary path — and since the one-running-per-scenario index (migration
//! 20260910142419) a row stuck `running` would also block every future scan of
//! that scenario, which makes leaving one behind expensive as well as untidy.

use std::sync::Arc;
use std::time::Instant;

use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::dto::ThemeScanSummary;
use crate::repositories::pipeline_repository::{
    cancel_scan_run, fail_scan_run, finalize_scan_run_completed, ScanRunFinal,
};
use crate::services::theme_scan::PreparedScan;
use crate::services::theme_scan_judge::{judge_all, JudgeInputs, JudgeRun};
use crate::services::theme_scan_persist::{count_to_i32, persist_and_summarize, ScanRunMeta};
use crate::state::AppState;

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
pub(crate) fn spawn_scan_job(
    state: &AppState,
    prepared: PreparedScan,
    run_id: Uuid,
    scenario_id: Uuid,
    candidates_total: i32,
    cancel: CancellationToken,
) {
    tracing::info!(
        %scenario_id, %run_id, model_id = %prepared.model_id,
        concurrency = prepared.concurrency, candidates_total,
        prompt_file = %prepared.prompt_file,
        "theme scan: started (background)"
    );
    let state = state.clone();
    tokio::spawn(
        async move { execute_scan_job(state, prepared, run_id, scenario_id, cancel).await },
    );
}

/// The spawned judging task. Any failure marks the run `failed` with a reason —
/// it NEVER leaves the row stuck `running` (the startup sweep is the last-resort
/// guard, not the primary one).
async fn execute_scan_job(
    state: AppState,
    prepared: PreparedScan,
    run_id: Uuid,
    scenario_id: Uuid,
    cancel: CancellationToken,
) {
    let result = run_scan_job(&state, prepared, run_id, scenario_id, cancel).await;

    // The stop handle is dropped on EVERY exit — finished, cancelled or failed —
    // and before the failure is recorded, so the map can never accumulate a token
    // for a task that has gone. An absent entry is what the cancel route reads as
    // "nothing here can stop that", which is the truth from this line onward.
    state.scan_cancel.lock().await.remove(&run_id);

    if let Err(e) = result {
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
    cancel: CancellationToken,
) -> Result<(), String> {
    let clock = Instant::now();
    // ## Rust Learning: moving the summary's fields out BEFORE the partial move
    //
    // `judge_all` takes `prepared.groups` BY VALUE, which partially moves
    // `prepared` — after that line the struct as a whole can no longer be passed
    // anywhere, though its remaining fields can still be read. Lifting the four
    // the summary needs out first keeps that ordering explicit instead of leaving
    // a later reader to discover it as a borrow-check error.
    let model_id = prepared.model_id;
    let cost_per_input_token = prepared.cost_per_input_token;
    let cost_per_output_token = prepared.cost_per_output_token;
    let conservation = prepared.conservation;

    let judged = judge_all(
        JudgeInputs {
            provider: Arc::clone(&prepared.provider),
            semaphore: Arc::clone(&state.theme_scan_semaphore),
            concurrency: prepared.concurrency,
            scan_prompt: Arc::clone(&prepared.scan_prompt),
            scan_criteria: Arc::clone(&prepared.scan_criteria),
            params: prepared.params,
            pool: state.pipeline_pool.clone(),
            run_id,
            // The automatic-retry cap, read once at startup (ruled 2026-08-28).
            // The scan judges hundreds of candidates concurrently, so a permissive
            // cap multiplies across every one of them — which is precisely why the
            // default is zero and why the value is not decided here.
            policy: state.config.llm_retry_policy,
            cancel,
        },
        prepared.groups,
    )
    .await;
    // millis fit i64 for any real scan; the impossible overflow caps (Standing Rule 1).
    let duration_ms = i64::try_from(clock.elapsed().as_millis()).unwrap_or(i64::MAX);

    if judged.cancelled {
        return record_cancelled(state, run_id, scenario_id, &judged, duration_ms).await;
    }

    record_completed(
        state,
        ScanRunMeta {
            run_id,
            scenario_id,
            model_id,
            cost_per_input_token,
            cost_per_output_token,
            duration_ms,
            conservation,
        },
        judged,
    )
    .await
}

/// Record a run that judged its whole pool: persist, summarize, finalize.
///
/// The ordinary ending, split from [`run_scan_job`] for the function-size limit
/// along the branch that was already there — a run either reached the end of its
/// pool or a human stopped it, and this is the first of those two endings.
///
/// The order matters and is not incidental. `persist_and_summarize` replays the
/// verdict batch (a no-op for rows the judging loop already wrote, by the insert's
/// `ON CONFLICT DO NOTHING`) and computes the counts, tokens and conservation.
/// Only then is the header finalized: the summary has to EXIST before the row can
/// claim to carry it, or a poll landing between the two would read `completed`
/// with no report.
async fn record_completed(
    state: &AppState,
    meta: ScanRunMeta,
    judged: JudgeRun,
) -> Result<(), String> {
    let (run_id, scenario_id, duration_ms) = (meta.run_id, meta.scenario_id, meta.duration_ms);
    let summary = persist_and_summarize(&state.pipeline_pool, meta, judged.results).await;

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

/// Record a STOPPED run: `status = 'cancelled'`, counts exactly as they stand.
///
/// ## Why this path does not summarize, and why that is not a gap
///
/// A stopped run has no summary because it has no whole to summarize. Its live
/// counts are already on the row — `bump_scan_run_progress` wrote each one as its
/// group was judged — and its verdicts are already in `scan_run_verdicts`, written
/// per group by the judging loop for exactly this reason. So the record is
/// COMPLETE at the moment of the stop, and this function's whole job is to say so
/// on the status column. Running the summarizer instead would compute a
/// conservation block whose `judged` denominator counts groups this run never
/// asked about, and publish a reconciliation sentence that cannot reconcile.
///
/// `summary_json` therefore stays NULL, which is what the poll and the history
/// read as "no report to render" — absent, never fabricated (Standing Rule 1).
///
/// Zero rows updated is a real outcome, not a failure to swallow: it means
/// something else settled the run first (the delete route, say), and this returns
/// `Ok` after saying so in the log rather than overwriting a terminal state.
async fn record_cancelled(
    state: &AppState,
    run_id: Uuid,
    scenario_id: Uuid,
    judged: &JudgeRun,
    duration_ms: i64,
) -> Result<(), String> {
    let rows = cancel_scan_run(&state.pipeline_pool, run_id)
        .await
        .map_err(|e| format!("failed to record scan run {run_id} as cancelled: {e}"))?;

    if rows == 0 {
        tracing::warn!(
            %run_id, %scenario_id,
            "theme scan: stopped, but the run was no longer 'running' — something \
             else settled it first and its record is left untouched"
        );
        return Ok(());
    }

    tracing::info!(
        %run_id, %scenario_id,
        judged = judged.results.len(), skipped = judged.skipped, duration_ms,
        "theme scan: stopped by request; judged verdicts kept and still projecting"
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
