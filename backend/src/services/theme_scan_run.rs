//! Theme Scan background-run lifecycle (start → spawn → judge → finalize) + the
//! poll read. Split from `theme_scan.rs` (module-size limit): that module owns
//! the synchronous PRECONDITIONS (scenario load, provider resolution, gate,
//! candidate read) and the error taxonomy; THIS module owns the background job.
//!
//! The synchronous half runs in the POST so a gate/precondition failure is an
//! immediate HTTP error, never a background failure the user must poll to find.
//! The judging then runs in a spawned `tokio` task that updates the `scan_runs`
//! row as it goes; the GET polls it.

use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use uuid::Uuid;

use crate::domain::llm_params::ResolvedLlmParams;
use crate::dto::{ScanRunStatusResponse, ThemeScanSummary};
use crate::repositories::pipeline_repository::{
    fail_scan_run, finalize_scan_run_completed, get_scan_run, insert_scan_run_running,
    ScanRunFinal, ScanRunStart,
};
use crate::services::theme_scan::{
    load_scenario_fenced, prepare_scan, PreparedScan, ThemeScanError,
};
use crate::services::theme_scan_judge::judge_all;
use crate::services::theme_scan_persist::{count_to_i32, persist_and_summarize, ScanRunMeta};
use crate::state::AppState;

/// The immediate result of starting a background scan.
pub struct ScanStarted {
    pub run_id: Uuid,
    pub candidates_total: i32,
}

/// Start a Theme Scan as a BACKGROUND job and return its handle immediately.
///
/// The preconditions run SYNCHRONOUSLY — scenario load, provider resolution, the
/// vLLM hard gate, and the candidate-pool read — so a gate failure (or any
/// precondition failure) is an immediate typed error the route returns as its
/// HTTP status, NEVER a background failure the user must poll to discover. Then
/// the `running` `scan_runs` row is inserted (denominator known) and the judging
/// task is spawned; the caller polls [`get_scan_run_status`].
pub async fn start_theme_scan(
    state: &AppState,
    case_slug: &str,
    scenario_id: Uuid,
    requested_model_id: Option<String>,
    dry_run: bool,
) -> Result<ScanStarted, ThemeScanError> {
    let prepared =
        prepare_scan(state, case_slug, scenario_id, requested_model_id.as_deref()).await?;
    let run_id = Uuid::new_v4();
    let candidates_total = count_to_i32(prepared.candidates.len(), "candidates_total");

    insert_scan_run_running(
        &state.pipeline_pool,
        &ScanRunStart {
            run_id,
            scenario_id,
            model_id: prepared.model_id.clone(),
            resolved_params: params_snapshot(&prepared.params),
            dry_run,
            candidates_total,
            started_at: Utc::now(),
        },
    )
    .await
    .map_err(|source| ThemeScanError::ScanRunWriteFailed { run_id, source })?;

    tracing::info!(
        case_slug, %scenario_id, %run_id, model_id = %prepared.model_id, dry_run,
        concurrency = prepared.concurrency, candidates_total,
        "theme scan: started (background)"
    );

    // ## Rust Learning: `tokio::spawn` needs `Send + 'static`
    //
    // The task outlives this function, so its future must own everything it uses
    // (`'static`) and be movable across threads (`Send`). `AppState` is `Clone`
    // (all Arc/pool fields — a clone is refcount bumps) and every field is
    // Send+Sync+'static; `PreparedScan` is likewise (Arc provider, Arc<str>, a
    // Copy params, owned Vec/String). So we clone `state` and MOVE both into the
    // task. The task's own errors are handled inside it (it must never leave the
    // row stuck `running`) — the `JoinHandle` is dropped deliberately.
    let state = state.clone();
    tokio::spawn(
        async move { execute_scan_job(state, prepared, run_id, scenario_id, dry_run).await },
    );

    Ok(ScanStarted {
        run_id,
        candidates_total,
    })
}

/// The spawned judging task. Any failure marks the run `failed` with a reason —
/// it NEVER leaves the row stuck `running` (the startup sweep is the last-resort
/// guard, not the primary one).
async fn execute_scan_job(
    state: AppState,
    prepared: PreparedScan,
    run_id: Uuid,
    scenario_id: Uuid,
    dry_run: bool,
) {
    if let Err(e) = run_scan_job(&state, prepared, run_id, scenario_id, dry_run).await {
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
    dry_run: bool,
) -> Result<(), String> {
    let clock = Instant::now();
    let results = judge_all(
        Arc::clone(&prepared.provider),
        Arc::clone(&state.theme_scan_semaphore),
        prepared.concurrency,
        Arc::clone(&prepared.scan_prompt),
        Arc::clone(&prepared.attack_meaning),
        prepared.params,
        prepared.candidates,
        state.pipeline_pool.clone(),
        run_id,
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
            dry_run,
            cost_per_input_token: prepared.cost_per_input_token,
            cost_per_output_token: prepared.cost_per_output_token,
            duration_ms,
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
        %run_id, %scenario_id, dry_run, candidates_read = summary.candidates_read,
        relevant = summary.relevant_written, irrelevant = summary.irrelevant,
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
        relevant_count: count_to_i32(summary.relevant_written, "relevant_count"),
        irrelevant_count: count_to_i32(summary.irrelevant, "irrelevant_count"),
        failed_count: count_to_i32(summary.failed, "failed_count"),
        input_tokens: summary.input_tokens,
        output_tokens: summary.output_tokens,
        computed_cost: summary.computed_cost,
        duration_ms,
        summary_json,
    }
}

/// Serialize the resolved params to the `scan_runs.resolved_params` JSONB shape.
fn params_snapshot(p: &ResolvedLlmParams) -> serde_json::Value {
    serde_json::json!({
        "temperature": p.temperature,
        "timeout_secs": p.timeout_secs,
        "max_tokens": p.max_tokens,
    })
}

/// Read the live status of one scan run for the poll endpoint.
///
/// Case-fenced (Standing Rule 1 — a caller must not learn a run exists in another
/// case): the scenario must belong to `case_slug` (fence 1, reusing the scan's own
/// loader), and the run must belong to that scenario (fence 2). Either miss is
/// [`ThemeScanError::ScanRunNotFound`], identical to a truly-absent id.
pub async fn get_scan_run_status(
    state: &AppState,
    case_slug: &str,
    scenario_id: Uuid,
    run_id: Uuid,
) -> Result<ScanRunStatusResponse, ThemeScanError> {
    load_scenario_fenced(&state.pipeline_pool, case_slug, scenario_id).await?;

    let row = get_scan_run(&state.pipeline_pool, run_id)
        .await
        .map_err(|source| ThemeScanError::ScanRunReadFailed { run_id, source })?
        .ok_or(ThemeScanError::ScanRunNotFound { run_id })?;

    if row.scenario_id != scenario_id {
        return Err(ThemeScanError::ScanRunNotFound { run_id });
    }

    Ok(ScanRunStatusResponse {
        run_id: row.run_id,
        status: row.status,
        model_id: row.model_id,
        dry_run: row.dry_run,
        candidates_total: row.candidates_total,
        candidates_judged: row.candidates_judged,
        relevant_count: row.relevant_count,
        irrelevant_count: row.irrelevant_count,
        failed_count: row.failed_count,
        error: row.error,
        summary: row.summary_json,
    })
}
