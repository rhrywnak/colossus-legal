// =============================================================================
// scanHeaderFixtures — the run headers, words and models the header tests share
// =============================================================================
//
// One fixture file for three test modules, for the reason `cardFixtures.ts`
// exists: a wording fixture copied into each file drifts one copy at a time,
// and the copy that drifts is the one whose test then proves nothing.
//
// The wording values are the ones migration `20260911144911` seeds. They are
// literals here on purpose — a test asserting an invariant is allowed the
// literal (Rule 2's own carve-out), and a template read from anywhere else
// would make these tests pass by agreeing with whatever shipped.

import type { ScanModel, ScanRunHeader, ScanWording } from "../../services/themeScan";

/** A run header carrying the fields the facts header reads. */
export function run(overrides: Partial<ScanRunHeader> = {}): ScanRunHeader {
  return {
    run_id: "11111111-1111-1111-1111-111111111111",
    model_id: "unsloth/Qwen3.8-27B-NVFP4",
    status: "completed",
    candidates_total: 313,
    candidates_judged: 313,
    relevant_count: 24,
    irrelevant_count: 289,
    failed_count: 0,
    computed_cost: null,
    duration_ms: 3_299_000,
    started_at: "2026-09-11T00:06:00Z",
    candidates_read: 313,
    error: null,
    dry_run: false,
    pool_delta: null,
    ...overrides,
  };
}

/** A catalogue entry. Untimed unless a test says otherwise. */
export function model(overrides: Partial<ScanModel> = {}): ScanModel {
  return {
    model_id: "unsloth/Qwen3.8-27B-NVFP4",
    display_name: "Qwen3.8 27B (NVFP4, local)",
    is_default: true,
    billing_class: "local",
    display_label: "Qwen3.8 27B (NVFP4, local)",
    confirm_label: "Qwen3.8 27B (NVFP4, local) (local · $0)",
    ...overrides,
  };
}

/** Every stored string the header renders, as the migration seeds them. */
export function wording(overrides: Partial<ScanWording> = {}): ScanWording {
  return {
    view_label: "View results",
    delete_confirm_template: "Remove the scan run from {run}?",
    card_collapsed_summary_template: "Last scan {when} · {model} · {count} proposed",
    report_advisory_note: "Advisory only.",
    report_proposed_line_template: "{count} proposed and awaiting your ruling",
    report_tile_gathered: "gathered",
    report_tile_folded: "duplicates folded",
    report_tile_set_aside: "set aside before judging",
    report_tile_judged: "judged",
    report_tile_proposed: "proposed",
    report_tile_failed: "failed",
    status_complete_label: "Complete",
    status_failed_label: "Failed",
    card_collapsed_failed_template: "Last scan {when} · {model} · Failed",
    header_scan_label: "Scan",
    header_history_label: "History",
    header_running_notice: "A scan is running — its progress is below.",
    header_no_model_notice: "No scan-eligible model is available.",
    header_last_scan_template: "Last scan {when} · {status} · {count} candidates",
    header_last_scan_no_count_template: "Last scan {when} · {status}",
    header_status_completed: "completed",
    header_status_cancelled: "cancelled",
    header_status_failed: "failed",
    header_status_running: "running",
    header_confirm_timed_template:
      "Run a theme scan with {model}? {count} candidates, about {minutes} minutes.",
    header_confirm_template: "Run a theme scan with {model}? {count} candidates.",
    header_confirm_no_count_template: "Run a theme scan with {model}?",
    header_confirm_run_label: "Run scan",
    header_confirm_cancel_label: "Cancel",
    ...overrides,
  };
}

/** The locale formatter, stubbed — these tests are about the sentence, not Intl. */
export const formatWhen = () => "Sep 11, 12:06 AM";

/** The served never-scanned sentence, verbatim from migration 20260808084539. */
export const NEVER_SCANNED =
  "No scan has run yet. Run a theme scan above, or browse the raw evidence pool.";
