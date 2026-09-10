// =============================================================================
// themeScanRunState.ts — the pure decisions behind "the server owns the scan".
// -----------------------------------------------------------------------------
// Extracted from ThemeScanPanel so they can be unit-tested without rendering
// (CLAUDE.md frontend test pattern: pure-helper tests + service tests; component
// testing infrastructure — RTL, jsdom — is not set up in this repo).
//
// Two questions, both of which the panel used to answer with `false` by
// construction because a scan only existed inside the tab that started it:
//
//   1. "Is a scan already going that this page should be showing?" — the answer
//      is now in the run history the server serves, so a fresh page load, a
//      second tab, or a navigation away and back all pick the scan back up.
//   2. "Has the run I am polling settled?" — three terminal states now, not two.
// =============================================================================

import type { ScanRunHeader, ScanRunState } from "../services/themeScan";

/** The run a freshly-loaded page should start polling, with what it needs. */
export type AdoptableRun = {
  runId: string;
  modelId: string;
  /** `started_at` as epoch milliseconds, so the running view's timer shows the
   *  scan's REAL elapsed time rather than restarting from zero on this page.
   *  A run adopted eight minutes in should say eight minutes. */
  startedAtMs: number;
};

/**
 * The run this page should adopt and poll, or `null` when there is none.
 *
 * ## Why `alreadySettled` is a parameter and not an implementation detail
 *
 * The run history is fetched asynchronously, so there is a moment after a run
 * settles when the panel has already cleared its active run but the list it is
 * looking at still says `running`. Without a memory of what has settled in this
 * session, this function would re-adopt the run it just finished, the poll would
 * see `cancelled` again, and the two would trade places until the refetch landed
 * — a flicker with no upper bound on a slow connection.
 *
 * Passing the set in (rather than keeping one inside this module) keeps the
 * function pure and the memory owned by the component whose lifetime it matches.
 *
 * ## Why `started_at` is parsed here and not rendered here
 *
 * The panel needs a NUMBER (the timer counts from it); the history table needs a
 * formatted local string. Those are different jobs, so this one does not format
 * and `themeScanFormat` does not parse. An unparseable timestamp degrades to
 * "now" — the run is genuinely in flight, and a timer that starts at zero is a
 * smaller lie than refusing to show the scan at all.
 */
export function adoptableRun(
  runs: ScanRunHeader[],
  alreadySettled: ReadonlySet<string>,
): AdoptableRun | null {
  const live = runs.find((run) => run.status === "running" && !alreadySettled.has(run.run_id));
  if (!live) return null;
  const parsed = Date.parse(live.started_at);
  return {
    runId: live.run_id,
    modelId: live.model_id,
    startedAtMs: Number.isNaN(parsed) ? Date.now() : parsed,
  };
}

/**
 * Has this run finished, one way or another?
 *
 * `cancelled` settles exactly as `completed` does — the panel clears its active
 * run, re-reads the history and re-reads the cards — and that equivalence is the
 * whole of part E9. The only difference downstream is that a cancelled run has no
 * `summary` to cache, which its caller handles by asking for one rather than by
 * asking what the status was.
 *
 * `failed` is settled too, and is deliberately NOT folded in with these two: it
 * takes the panel's error branch, because a run that failed has a reason the
 * human needs on screen. Callers that mean "stop polling" should use
 * [`isTerminalRun`]; callers that mean "it finished normally" use this.
 */
export function isSettledRun(status: ScanRunState): boolean {
  return status === "completed" || status === "cancelled";
}

/** Any state the poll should stop on — the two settled ones plus `failed`. */
export function isTerminalRun(status: ScanRunState): boolean {
  return isSettledRun(status) || status === "failed";
}
