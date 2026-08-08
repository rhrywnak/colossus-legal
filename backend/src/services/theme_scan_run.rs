//! Theme Scan reads and curation of runs that ALREADY EXIST: the poll, the
//! history list, the delete, and the merge.
//!
//! The START path — the only code that creates a run — lives in the sibling
//! [`crate::services::theme_scan_start`]; `theme_scan.rs` owns the synchronous
//! preconditions and the error taxonomy. The three-way split keeps each module
//! within the size limit along a real seam: preconditions, creation, and
//! everything done to a run afterwards.
//!
//! Every function here is case-fenced. A caller must not learn that a scenario —
//! or a run — exists in another case, so a cross-case id is reported exactly like
//! an absent one.

use std::collections::HashSet;

use chrono::Utc;
use uuid::Uuid;

use crate::dto::theme_scan::ScanHistoryWording;
use crate::dto::{ScanRunHeader, ScanRunListResponse, ScanRunStatusResponse};
use crate::repositories::pipeline_repository::{
    count_run_provenance, delete_scan_run, get_scan_run, list_applied_node_ids_for_run,
    list_candidate_ordinals, list_scan_runs, merge_run_into_scenario_recording, ScanRunHeaderRow,
};
use crate::services::scan_conservation::annotate_conservation_line;
use crate::services::scan_run_delta::with_pool_deltas;
use crate::services::scan_run_enrich::annotate_summary_logged;
use crate::services::theme_scan::{load_scenario_fenced, ThemeScanError};
use crate::state::AppState;

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
    // The reconciliation sentence, composed here from the run's FROZEN counts and
    // the LIVE stored template (task 2.15 item 1c). Read-time for the same reason
    // the two annotations above are: the record is what the run did, the words are
    // what today's store says.
    annotate_conservation_line(
        summary,
        run_id,
        &state
            .settings
            .current()
            .scan_wording
            .conservation_line_template,
    );
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

    // Map each row, then fill in the position-derived pool deltas over the whole
    // history at once (task 1.7C, R2) — a single row cannot know how much bigger
    // its pool read was than the one before it.
    let runs = with_pool_deltas(rows.into_iter().map(scan_run_header_from_row).collect());
    let words = &state.settings.current().scan_wording;
    Ok(ScanRunListResponse {
        runs,
        wording: ScanHistoryWording {
            view_label: words.history_view_label.clone(),
            delete_confirm_template: words.history_delete_confirm_template.clone(),
        },
    })
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
        candidates_read: row.candidates_read,
        error: row.error,
        dry_run: row.dry_run,
        // Position-derived, so it is filled by `with_pool_deltas` once the whole
        // history is in hand — a single row cannot know it (task 1.7C, R2).
        pool_delta: None,
    }
}

#[cfg(test)]
#[path = "theme_scan_run_tests.rs"]
mod tests;
