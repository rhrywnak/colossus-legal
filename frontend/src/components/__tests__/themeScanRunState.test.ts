/**
 * Pure-helper tests for the Theme Scan panel's server-owned run state
 * (task SCAN_SERVER_STATE, parts E8 and E9).
 *
 * These are the decisions the panel makes; the component itself is a one-line
 * call to each. Component testing (RTL/jsdom) is not set up in this repo
 * (CLAUDE.md Rule 30), so the logic is extracted to be testable and the source
 * scan at the foot of this file proves the component actually calls it — the
 * same fence `scenarioPageStructure.test.ts` uses, and for the same reason: a
 * helper nothing calls is a tested feature that does not ship.
 */
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { adoptableRun, isSettledRun, isTerminalRun } from "../themeScanRunState";
import type { ScanRunHeader, ScanRunState } from "../../services/themeScan";

function header(overrides: Partial<ScanRunHeader>): ScanRunHeader {
  return {
    run_id: "r1",
    model_id: "qwen-14b",
    status: "completed",
    candidates_total: 10,
    candidates_judged: 10,
    relevant_count: 3,
    irrelevant_count: 7,
    failed_count: 0,
    computed_cost: null,
    duration_ms: 1000,
    started_at: "2026-09-10T14:00:00Z",
    candidates_read: 10,
    error: null,
    dry_run: false,
    pool_delta: null,
    ...overrides,
  };
}

// ─── E8: a page adopts the scan the server says is running ───────────────────

describe("adoptableRun", () => {
  it("adopts the running run, carrying the model and the server's start time", () => {
    // The whole of E8: a page that has just loaded — a reload, a second tab, a
    // navigation back — finds the in-flight scan in the history the server
    // serves, and picks it back up. Before this the run was invisible outside
    // the tab that started it.
    const runs = [
      header({ run_id: "done", status: "completed" }),
      header({
        run_id: "live",
        status: "running",
        model_id: "opus-5",
        started_at: "2026-09-10T14:00:00Z",
      }),
    ];
    expect(adoptableRun(runs, new Set())).toEqual({
      runId: "live",
      modelId: "opus-5",
      startedAtMs: Date.parse("2026-09-10T14:00:00Z"),
    });
  });

  it("adopts nothing when no run is running", () => {
    // The ordinary case: an idle scenario. `null`, so the panel offers Run.
    const runs = [
      header({ run_id: "a", status: "completed" }),
      header({ run_id: "b", status: "failed" }),
      header({ run_id: "c", status: "cancelled" }),
    ];
    expect(adoptableRun(runs, new Set())).toBeNull();
    expect(adoptableRun([], new Set())).toBeNull();
  });

  it("never re-adopts a run this session already watched settle", () => {
    // The flicker guard. The history is refetched asynchronously, so for a moment
    // after a run settles the list still says `running` while the panel has
    // already cleared it. Without this, the panel would re-adopt the run it just
    // finished, the poll would settle it again, and the two would trade places
    // until the refetch landed.
    const runs = [header({ run_id: "live", status: "running" })];
    expect(adoptableRun(runs, new Set(["live"]))).toBeNull();
    // …and a DIFFERENT running run is still adoptable: the memory is per-run, not
    // a switch that turns adoption off.
    const other = adoptableRun([header({ run_id: "other", status: "running" })], new Set(["live"]));
    expect(other?.runId).toBe("other");
  });

  it("takes the FIRST running row, because the server serves them newest-first", () => {
    // The order is the server's (`ORDER BY started_at DESC`) and this must not
    // re-sort and quietly disagree with the history table the human is reading.
    const runs = [
      header({ run_id: "newer", status: "running", started_at: "2026-09-10T15:00:00Z" }),
      header({ run_id: "older", status: "running", started_at: "2026-09-10T09:00:00Z" }),
    ];
    expect(adoptableRun(runs, new Set())?.runId).toBe("newer");
  });

  it("falls back to now when the start time is unreadable, rather than refusing", () => {
    // The run is genuinely in flight. A timer that starts at zero is a smaller
    // lie than a page that hides a running scan over a bad timestamp — and the
    // number it would otherwise produce (NaN) would render as "NaN:aN".
    const adopted = adoptableRun([header({ run_id: "live", status: "running", started_at: "not a date" })], new Set());
    expect(adopted).not.toBeNull();
    expect(Number.isNaN(adopted?.startedAtMs ?? NaN)).toBe(false);
  });
});

// ─── E9: `cancelled` settles exactly as `completed` does ─────────────────────

describe("isSettledRun / isTerminalRun", () => {
  it("treats cancelled exactly as completed", () => {
    // E9's whole claim. The panel clears its active run, re-reads the history and
    // re-reads the cards on either one; the only downstream difference is that a
    // cancelled run has no summary to cache, which its caller handles by ASKING
    // for a summary rather than by asking what the status was.
    expect(isSettledRun("cancelled")).toBe(true);
    expect(isSettledRun("completed")).toBe(true);
  });

  it("does not settle a running run, and does not settle a failed one", () => {
    // `failed` is terminal but NOT settled: it takes the panel's error branch,
    // because a run that failed has a reason the human needs on screen. Folding
    // it in here would silently swallow that reason.
    expect(isSettledRun("running")).toBe(false);
    expect(isSettledRun("failed")).toBe(false);
  });

  it("stops polling on all three terminal states", () => {
    const terminal: ScanRunState[] = ["completed", "cancelled", "failed"];
    for (const status of terminal) {
      expect(isTerminalRun(status)).toBe(true);
    }
    expect(isTerminalRun("running")).toBe(false);
  });
});

// ─── The reachability fence ──────────────────────────────────────────────────

describe("the panel actually uses these helpers", () => {
  const panel = readFileSync(
    join(__dirname, "..", "ThemeScanPanel.tsx"),
    "utf8",
  );

  it("imports and calls both decisions rather than re-deciding inline", () => {
    // Every assertion above is satisfiable by a helper nothing calls. This is the
    // available fence without RTL, and it is the one that would actually catch
    // the mistake: a later edit that inlines `status === "completed"` back into
    // the poll would pass every test above and silently drop `cancelled`.
    expect(panel).toContain('from "./themeScanRunState"');
    expect(panel).toContain("adoptableRun(runs, settledRuns.current)");
    expect(panel).toContain("isSettledRun(status.status)");
  });

  it("does not branch on the completed literal in the poll any more", () => {
    // The specific regression: `status.status === "completed"` was the old
    // settle test, and a cancelled run would fall straight through it into the
    // running branch — the panel polling forever a run that had stopped.
    expect(panel).not.toContain('status.status === "completed"');
  });
});
