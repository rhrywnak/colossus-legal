//! Theme Scan background-run lifecycle (start → spawn → judge → finalize) + the
//! poll read. Split from `theme_scan.rs` (module-size limit): that module owns
//! the synchronous PRECONDITIONS (scenario load, provider resolution, gate,
//! candidate read) and the error taxonomy; THIS module owns the background job.
//!
//! The synchronous half runs in the POST so a gate/precondition failure is an
//! immediate HTTP error, never a background failure the user must poll to find.
//! The judging then runs in a spawned `tokio` task that updates the `scan_runs`
//! row as it goes; the GET polls it.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use uuid::Uuid;

use crate::domain::llm_params::ResolvedLlmParams;
use crate::dto::{ScanRunHeader, ScanRunListResponse, ScanRunStatusResponse, ThemeScanSummary};
use crate::repositories::pipeline_repository::{
    count_run_provenance, delete_scan_run, fail_scan_run, finalize_scan_run_completed,
    get_scan_run, insert_scan_run_running, list_applied_node_ids_for_run, list_candidate_ordinals,
    list_scan_runs, merge_run_into_scenario_recording, ScanRunFinal, ScanRunHeaderRow,
    ScanRunStart,
};
use crate::services::scan_run_enrich::annotate_summary_logged;
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
            resolved_params: params_snapshot(&prepared.params, &prepared.prompt_file),
            candidates_total,
            started_at: Utc::now(),
        },
    )
    .await
    .map_err(|source| ThemeScanError::ScanRunWriteFailed { run_id, source })?;

    tracing::info!(
        case_slug, %scenario_id, %run_id, model_id = %prepared.model_id,
        concurrency = prepared.concurrency, candidates_total,
        prompt_file = %prepared.prompt_file,
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
    tokio::spawn(async move { execute_scan_job(state, prepared, run_id, scenario_id).await });

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

/// Serialize the resolved params to the `scan_runs.resolved_params` JSONB shape.
///
/// `prompt_file` is the resolved judging-prompt filename (from
/// `THEME_SCAN_PROMPT_FILE`). Recording it here is what makes each run's
/// provenance answerable from data — "which prompt judged this run" — now that
/// the filename is deployment config rather than a compiled-in const. The column
/// is JSONB (caller-owns-serialization), so this addition needs no migration.
fn params_snapshot(p: &ResolvedLlmParams, prompt_file: &str) -> serde_json::Value {
    serde_json::json!({
        "temperature": p.temperature,
        "timeout_secs": p.timeout_secs,
        "max_tokens": p.max_tokens,
        "prompt_file": prompt_file,
    })
}

/// Read the live status of one scan run for the poll endpoint.
///
/// Case-fenced (Standing Rule 1 — a caller must not learn a run exists in another
/// case): the scenario must belong to `case_slug` (fence 1, reusing the scan's own
/// loader), and the run must belong to that scenario (fence 2). Either miss is
/// [`ThemeScanError::ScanRunNotFound`], identical to a truly-absent id.
///
/// ## The summary is ANNOTATED on the way out
///
/// A completed run's stored summary is a historical record and is never rewritten.
/// Two things the results list needs are not in it, because neither belongs to the
/// run: each pick's candidate ordinal (`C-14`, owned by the scenario) and whether
/// this run's judgment for that pick has already been merged. Both are derived here
/// and layered onto a copy — see [`crate::services::scan_run_enrich`].
///
/// This is where "applied" is computed rather than in the merge response, because a
/// reopened HISTORICAL run needs it just as much as the one just merged.
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

    // Only a completed run has a summary to annotate; a running/failed one carries
    // `None` and needs no extra reads.
    let summary = match row.summary_json {
        Some(mut summary) => {
            annotate_run_summary(state, scenario_id, run_id, &mut summary).await?;
            Some(summary)
        }
        None => None,
    };

    Ok(ScanRunStatusResponse {
        run_id: row.run_id,
        status: row.status,
        model_id: row.model_id,
        candidates_total: row.candidates_total,
        candidates_judged: row.candidates_judged,
        relevant_count: row.relevant_count,
        irrelevant_count: row.irrelevant_count,
        failed_count: row.failed_count,
        error: row.error,
        summary,
    })
}

/// Read the scenario's ordinals and this run's applied picks, then annotate.
///
/// Split from [`get_scan_run_status`] to keep it within the function-size limit.
/// Both reads are hard failures rather than degradations: serving a results list
/// with silently-missing chips or a silently-absent applied state would invite the
/// human to re-merge picks that are already applied (Standing Rule 1 — a partial
/// answer that looks complete is the failure mode to avoid).
async fn annotate_run_summary(
    state: &AppState,
    scenario_id: Uuid,
    run_id: Uuid,
    summary: &mut serde_json::Value,
) -> Result<(), ThemeScanError> {
    let ordinals = list_candidate_ordinals(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|source| ThemeScanError::ScanRunReadFailed { run_id, source })?;

    let applied: HashSet<String> = list_applied_node_ids_for_run(&state.pipeline_pool, run_id)
        .await
        .map_err(|source| ThemeScanError::ScanRunReadFailed { run_id, source })?
        .into_iter()
        .collect();

    annotate_summary_logged(summary, run_id, &ordinals, &applied);
    Ok(())
}

/// List a scenario's scan-run HISTORY (newest first) as lightweight headers.
///
/// Case-fenced identically to [`get_scan_run_status`] but with **fence 1 only**:
/// the scenario must belong to `case_slug` (else the whole list is
/// [`ThemeScanError::ScenarioNotFound`] → 404 — a caller must not learn a
/// scenario exists in another case). No per-row fence is needed here: the repo
/// query is already scoped `WHERE scenario_id = $1`, so every returned row
/// belongs to this fenced scenario by construction (contrast `get_scan_run`,
/// keyed by `run_id` alone, which needs the extra `scenario_id` match).
pub async fn list_scenario_scan_runs(
    state: &AppState,
    case_slug: &str,
    scenario_id: Uuid,
) -> Result<ScanRunListResponse, ThemeScanError> {
    load_scenario_fenced(&state.pipeline_pool, case_slug, scenario_id).await?;

    let rows = list_scan_runs(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|source| ThemeScanError::ScanRunListFailed {
            scenario_id,
            source,
        })?;

    let runs = rows.into_iter().map(scan_run_header_from_row).collect();
    Ok(ScanRunListResponse { runs })
}

/// Delete one of a scenario's scan runs.
///
/// Case-fenced with the SAME two fences as [`get_scan_run_status`], but the
/// second fence lives in the SQL rather than a post-read compare:
///   * **fence 1** — the scenario must belong to `case_slug`
///     ([`load_scenario_fenced`]); a miss is [`ThemeScanError::ScenarioNotFound`]
///     → 404, so a caller cannot probe another case's scenarios.
///   * **fence 2** — the delete is scoped `WHERE run_id = $1 AND scenario_id = $2`
///     (see [`delete_scan_run`]), so a run that exists but belongs to a different
///     scenario deletes zero rows — indistinguishable from a truly-absent id.
///
/// Zero rows deleted → [`ThemeScanError::ScanRunNotFound`] (→ 404), NOT a silent
/// success (Standing Rule 1 — "I deleted it" and "there was nothing to delete"
/// are different observable outcomes). A running run is deletable like any other;
/// its `scan_run_verdicts` cascade with it.
///
/// ## The provenance gate (fence 3)
///
/// Before deleting, the run is checked for merge provenance. A run whose judgments
/// have entered the case is REFUSED with [`ThemeScanError::ScanRunMerged`] → 409.
///
/// This is a deliberate restriction, not a database constraint: the FKs would
/// happily let the delete proceed (`scan_run_merges` cascades, `source_run_id`
/// sets null), and that is precisely the problem — one delete would silently
/// destroy both provenance records while leaving the merged judgments in the case.
/// The FK behaviors stay as defence-in-depth for the unmerged path; this check is
/// the primary guard.
///
/// The check runs AFTER the case fence, so it can never reveal the existence of
/// another case's run, and BEFORE the delete, so a refusal leaves nothing
/// half-done.
pub async fn delete_scenario_scan_run(
    state: &AppState,
    case_slug: &str,
    scenario_id: Uuid,
    run_id: Uuid,
) -> Result<(), ThemeScanError> {
    load_scenario_fenced(&state.pipeline_pool, case_slug, scenario_id).await?;

    // A failed check propagates rather than defaulting to "no provenance": treating
    // an unreadable check as permission to delete would fail in the destructive
    // direction (Standing Rule 1).
    let provenance = count_run_provenance(&state.pipeline_pool, run_id)
        .await
        .map_err(|source| ThemeScanError::ScanRunProvenanceCheckFailed { run_id, source })?;

    if provenance.is_protected() {
        tracing::info!(
            %run_id, %scenario_id,
            merge_events = provenance.merge_events,
            attributed_facts = provenance.attributed_facts,
            "refusing to delete a merged scan run; its provenance is retained"
        );
        return Err(ThemeScanError::ScanRunMerged {
            run_id,
            merge_events: provenance.merge_events,
            attributed_facts: provenance.attributed_facts,
        });
    }

    let rows_affected = delete_scan_run(&state.pipeline_pool, scenario_id, run_id)
        .await
        .map_err(|source| ThemeScanError::ScanRunDeleteFailed { run_id, source })?;

    if rows_affected == 0 {
        return Err(ThemeScanError::ScanRunNotFound { run_id });
    }
    Ok(())
}

/// Merge one stored scan run's relevant picks into the scenario's candidate facts.
///
/// The Merge (set-as-basis) feature: promote a run you already paid for into the
/// working scenario, status-preserving, with zero LLM calls. Case-fenced with the
/// SAME two fences as [`get_scan_run_status`] (a caller must not merge across
/// cases or scenarios):
///   * **fence 1** — the scenario belongs to `case_slug` ([`load_scenario_fenced`]).
///   * **fence 2** — the run belongs to THIS scenario. A run that is absent, or
///     that lives under a different scenario, is [`ThemeScanError::ScanRunNotFound`]
///     → 404 (identical to the poll's fence-2). This is why fence 2 is an explicit
///     read+compare here and not left to the merge SQL's own scenario JOIN: the
///     JOIN would silently merge zero rows, which we must NOT collapse with a
///     legitimate "run has no relevant picks" zero (Standing Rule 1).
///
/// Returns the number of picks that landed as `undecided` suggestions (new or
/// refreshed); picks preserved as existing `included`/`dropped` curation are not
/// counted. A completed benchmark run is the normal input, but no status gate is
/// imposed — a run with no relevant verdicts simply merges zero.
///
/// `selected_ids` are the graph_node_ids the human CHECKED in the results list —
/// merge writes the scan's judgment onto ONLY these (Option A). An empty selection
/// is rejected up front as a 400 ([`ThemeScanError::EmptySelection`]) rather than
/// silently merging zero rows, so "you selected nothing" stays a distinct,
/// actionable observable from "the run had no relevant picks" (Standing Rule 1).
pub async fn merge_scenario_scan_run(
    state: &AppState,
    case_slug: &str,
    scenario_id: Uuid,
    run_id: Uuid,
    selected_ids: &[String],
) -> Result<u64, ThemeScanError> {
    // A merge with nothing checked is a user error, not a no-op: fail loudly with a
    // 400 so the caller knows to check at least one pick. The frontend also disables
    // Merge until a pick is checked, so this is defence-in-depth, not the happy path.
    if selected_ids.is_empty() {
        return Err(ThemeScanError::EmptySelection { run_id });
    }

    // fence 1: the scenario belongs to the case.
    load_scenario_fenced(&state.pipeline_pool, case_slug, scenario_id).await?;

    // fence 2: the run belongs to THIS scenario (else 404) — read+compare, exactly
    // as get_scan_run_status does, so a wrong-scenario run is a clean not-found
    // rather than a silent zero-count merge.
    let row = get_scan_run(&state.pipeline_pool, run_id)
        .await
        .map_err(|source| ThemeScanError::ScanRunReadFailed { run_id, source })?
        .ok_or(ThemeScanError::ScanRunNotFound { run_id })?;
    if row.scenario_id != scenario_id {
        return Err(ThemeScanError::ScanRunNotFound { run_id });
    }

    // Merge the run's picks AND record the merge event in ONE transaction (decision:
    // same-transaction atomicity — either both land or neither). The transaction is
    // owned by the repository layer (`merge_run_into_scenario_recording`), matching
    // the house pattern where multi-statement writes hold their own `pool.begin()`
    // (e.g. `insert_scan_run_verdicts`); this service keeps only the case/scenario
    // fences. `Utc::now()` is bound here so the timestamp is the application's.
    merge_run_into_scenario_recording(
        &state.pipeline_pool,
        scenario_id,
        run_id,
        selected_ids,
        Utc::now(),
    )
    .await
    .map_err(|source| ThemeScanError::ScanRunMergeFailed { run_id, source })
}

/// Map one repository header row to its wire DTO. Pure (no I/O) and split out so
/// the field mapping is unit-testable without a database — every column the
/// history row shows is carried across 1:1.
fn scan_run_header_from_row(row: ScanRunHeaderRow) -> ScanRunHeader {
    ScanRunHeader {
        run_id: row.run_id,
        model_id: row.model_id,
        status: row.status,
        candidates_total: row.candidates_total,
        candidates_judged: row.candidates_judged,
        relevant_count: row.relevant_count,
        irrelevant_count: row.irrelevant_count,
        failed_count: row.failed_count,
        computed_cost: row.computed_cost,
        duration_ms: row.duration_ms,
        started_at: row.started_at,
    }
}

#[cfg(test)]
#[path = "theme_scan_run_tests.rs"]
mod tests;
