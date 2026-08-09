//! Theme Scan reads and curation of runs that ALREADY EXIST: the poll, the
//! history list, and the delete.
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

use uuid::Uuid;

use crate::dto::theme_scan::ScanPanelWording;
use crate::dto::{ScanRunHeader, ScanRunListResponse, ScanRunStatusResponse};
use crate::repositories::pipeline_repository::{
    count_run_provenance, delete_scan_run, get_scan_run, list_candidate_ordinals, list_scan_runs,
    ScanRunHeaderRow,
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
/// One thing the report needs is not in it, because it does not belong to the run:
/// each pick's candidate ordinal (`C-14`, which the SCENARIO owns and may assign
/// after the run judged). It is derived here and layered onto a copy — see
/// [`crate::services::scan_run_enrich`].
///
/// The "applied" flag that used to ride beside it is gone with merge: a pick is no
/// longer something to apply, and whether a human has ruled it is a question the
/// QUEUE answers, on the card, where the ruling is made.
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

/// Read the scenario's ordinals, then annotate.
///
/// Split from [`get_scan_run_status`] to keep it within the function-size limit.
/// The read is a hard failure rather than a degradation: a report whose entries
/// silently lost their C-codes looks exactly like a report of un-numbered
/// candidates (Standing Rule 1 — a partial answer that looks complete is the
/// failure mode to avoid).
async fn annotate_run_summary(
    state: &AppState,
    scenario_id: Uuid,
    run_id: Uuid,
    summary: &mut serde_json::Value,
) -> Result<(), ThemeScanError> {
    let ordinals = list_candidate_ordinals(&state.pipeline_pool, scenario_id)
        .await
        .map_err(|source| ThemeScanError::ScanRunReadFailed { run_id, source })?;

    annotate_summary_logged(summary, run_id, &ordinals);
    // The reconciliation sentence, composed here from the run's FROZEN counts and
    // the LIVE stored template (task 2.15 item 1c). Read-time for the same reason
    // the two annotations above are: the record is what the run did, the words are
    // what today's store says.
    let words = &state.settings.current().scan_wording;
    annotate_conservation_line(
        summary,
        run_id,
        &words.conservation_line_template,
        &words.conservation_failed_clause_template,
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
        wording: ScanPanelWording {
            view_label: words.history_view_label.clone(),
            delete_confirm_template: words.history_delete_confirm_template.clone(),
            card_collapsed_summary_template: words.card_collapsed_summary_template.clone(),
            report_advisory_note: words.report_advisory_note.clone(),
            report_proposed_line_template: words.report_proposed_line_template.clone(),
            report_tile_gathered: words.report_tile_gathered.clone(),
            report_tile_folded: words.report_tile_folded.clone(),
            report_tile_set_aside: words.report_tile_set_aside.clone(),
            report_tile_judged: words.report_tile_judged.clone(),
            report_tile_proposed: words.report_tile_proposed.clone(),
            report_tile_failed: words.report_tile_failed.clone(),
            status_complete_label: words.status_complete_label.clone(),
            status_failed_label: words.status_failed_label.clone(),
            card_collapsed_failed_template: words.card_collapsed_failed_template.clone(),
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
/// Before deleting, the run is checked for provenance. A run the RECORD depends on
/// — one any ruling cites as the scan that proposed it, or one a historical merge
/// references — is REFUSED with [`ThemeScanError::ScanRunCited`] → 409.
///
/// This is a deliberate restriction, not a database constraint: the FKs would
/// happily let the delete proceed (`scan_run_merges` cascades, `source_run_id`
/// sets null), and that is precisely the problem — one delete would silently
/// destroy both provenance records while leaving the rulings in the case with
/// nothing to say what put those candidates in front of the human. The FK
/// behaviours stay as defence-in-depth for the un-cited path; this check is the
/// primary guard.
///
/// ## What this means under the projection (architect ruling R1, 2026-08-08)
///
/// The guard is UNCHANGED and its reach has grown, deliberately. Every ruling made
/// on a proposed card now records `source_run_id`, so a run one ruling has drawn on
/// becomes undeletable — it is part of the ledger's chain of custody. A junk scan
/// nobody ruled from carries neither count and deletes freely, taking its unruled
/// proposals with it (they are a projection; nothing has to be cleaned up). That is
/// the case R-d exists for, and it is the one that matters for scan hygiene.
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
            "refusing to delete a scan run the record cites; its provenance is retained"
        );
        return Err(ThemeScanError::ScanRunCited {
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
