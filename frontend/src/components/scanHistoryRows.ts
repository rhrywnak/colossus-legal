// =============================================================================
// scanHistoryRows.ts — the scan-history table's rows, as data (§2.3, ruling R2)
// =============================================================================
//
// Design §2.3 item 2: a collapsed disclosure holding a table — When / Model /
// Candidates / New / Status — with failures shown honestly ("Failed — vLLM
// offline"). This module decides what each cell says; `ScanHistoryTable` renders
// it and chooses nothing.
//
// ## The three honesty rules the cells encode
//
// 1. **`candidates_read == 0` is an em dash, not a zero.** Zero means the run
//    never reached the pool read, and printing "0" claims it read a pool of zero
//    candidates. Two different facts, one of them false.
// 2. **A missing delta is an em dash, not a `+0`.** `pool_delta` is `null` for the
//    first measurable run — there is nothing to compare against, which is not the
//    same as "the pool did not change". The backend computes the delta (Rule 12);
//    this only renders it.
// 3. **A failure carries its reason.** `"Failed"` alone sends the reader to the
//    logs, which is the silent-failure shape Standing Rule 1 exists to prevent.
//    The reason is stored and now served, so it is shown.
//
// Plus the labelling of dry runs (measured: 3 of the 4 runs stored on DEV are dry
// runs, and a table that renders them identically to real ones tells the reader
// something false about what has been done to the pool).

import type { ScanRunHeader } from "../services/themeScan";
import { formatRunTimestamp } from "./themeScanFormat";

/** The em dash used wherever a number is genuinely absent rather than zero. */
// CONST: a typographic convention, not a tunable.
const ABSENT = "—";

/** One row of the history table. Every field is a string ready to render. */
export type ScanHistoryRow = {
  runId: string;
  /** `"Aug 2, 9:14 PM"`. */
  when: string;
  /** The model's display name, resolved by the panel's catalogue. */
  model: string;
  /** The pool this run read, or `"—"` when it never read one. */
  candidates: string;
  /** `"+54"` / `"0"` / `"-12"`, or `"—"` when there is no baseline. */
  newCount: string;
  /** `"Complete"`, `"Running"`, or `"Failed — vLLM offline"`. */
  status: string;
  /** Whether to render the "(Dry run)" label beside the model. */
  dryRun: boolean;
};

/**
 * The status cell.
 *
 * A `failed` run with no stored reason still reads "Failed" — honestly incomplete
 * rather than fabricating a cause. That case exists: a row can be killed between
 * its stub insert and the reason being written.
 */
function statusCell(run: ScanRunHeader): string {
  if (run.status === "failed") {
    const reason = run.error?.trim();
    return reason ? `Failed — ${reason}` : "Failed";
  }
  if (run.status === "running") return "Running";
  // `completed` is the only remaining token this build knows. An UNKNOWN token is
  // shown verbatim rather than mapped to "Complete": labelling a state this build
  // does not understand as a success is how a silent failure gets a green tick.
  return run.status === "completed" ? "Complete" : run.status;
}

/**
 * The signed delta cell.
 *
 * Positive gets an explicit `+` so a growing pool reads as growth at a glance;
 * negative keeps its own sign; zero is bare. Exported for the meta line, which
 * needs the same three cases worded the same way.
 */
export function formatPoolDelta(delta: number | null | undefined): string {
  if (delta == null) return ABSENT;
  return delta > 0 ? `+${delta}` : String(delta);
}

/**
 * Turn the served history into table rows.
 *
 * `runs` arrives newest-first (the backend's `ORDER BY started_at DESC`), and this
 * preserves that order — the deltas were computed against that ordering, so
 * re-sorting here would silently pair a row with someone else's delta.
 *
 * @param modelName Resolves a model id to its display label; owned by the panel's
 *   model catalogue, passed in so this module holds no catalogue of its own.
 */
export function scanHistoryRows(
  runs: ScanRunHeader[],
  modelName: (modelId: string) => string,
): ScanHistoryRow[] {
  return runs.map((run) => ({
    runId: run.run_id,
    when: formatRunTimestamp(run.started_at),
    model: modelName(run.model_id),
    candidates: run.candidates_read > 0 ? String(run.candidates_read) : ABSENT,
    newCount: formatPoolDelta(run.pool_delta),
    status: statusCell(run),
    dryRun: run.dry_run,
  }));
}

/**
 * The disclosure's own summary, e.g. `"Scan history (4)"`.
 *
 * Returns `null` when there are no runs: a disclosure that opens onto an empty
 * table is a worse answer than no disclosure, and the section's own empty-state
 * copy already covers a never-scanned scenario.
 */
export function scanHistorySummary(runCount: number): string | null {
  return runCount > 0 ? `Scan history (${runCount})` : null;
}
