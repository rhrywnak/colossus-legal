// =============================================================================
// scanHistoryRows.test.ts — the history table's honesty rules (§2.3, ruling R2)
// =============================================================================

import { describe, expect, it } from "vitest";

import { formatPoolDelta, scanHistoryRows, scanHistorySummary } from "../scanHistoryRows";
import type { ScanRunHeader } from "../../services/themeScan";

/** A run header carrying the fields the table reads. */
function run(overrides: Partial<ScanRunHeader> = {}): ScanRunHeader {
  return {
    run_id: "11111111-1111-1111-1111-111111111111",
    model_id: "claude-opus-4-8",
    status: "completed",
    candidates_total: 148,
    candidates_judged: 148,
    relevant_count: 24,
    irrelevant_count: 124,
    failed_count: 0,
    computed_cost: 0.42,
    duration_ms: 95_864,
    started_at: "2026-07-29T18:29:14Z",
    candidates_read: 148,
    error: null,
    dry_run: false,
    pool_delta: 54,
    ...overrides,
  };
}

const MODEL_NAMES: Record<string, string> = {
  "claude-opus-4-8": "Claude Opus 4.8",
  "Qwen/Qwen2.5-14B-Instruct-AWQ": "Qwen 14B (local)",
};
const modelName = (id: string) => MODEL_NAMES[id] ?? id;

describe("the Candidates cell", () => {
  it("shows the pool the run read", () => {
    expect(scanHistoryRows([run()], modelName)[0].candidates).toBe("148");
  });

  it("shows an em dash — not '0' — when the run never read the pool", () => {
    // `candidates_read` is NOT NULL, so a run that failed before the read stores 0.
    // Printing "0" claims it read a pool of zero candidates. Two different facts,
    // one of them false (Standing Rule 1).
    const rows = scanHistoryRows([run({ candidates_read: 0, status: "failed" })], modelName);
    expect(rows[0].candidates).toBe("—");
  });
});

describe("the New cell", () => {
  it("signs a growing pool so growth reads at a glance", () => {
    expect(scanHistoryRows([run({ pool_delta: 54 })], modelName)[0].newCount).toBe("+54");
  });

  it("shows a real zero as 0 — an unchanged pool IS a measurement", () => {
    expect(scanHistoryRows([run({ pool_delta: 0 })], modelName)[0].newCount).toBe("0");
  });

  it("keeps a negative delta rather than clamping it away", () => {
    // After task 2.5's re-anchoring the pool can legitimately shrink, and a -12 that
    // says so is worth more than a 0 that hides it.
    expect(scanHistoryRows([run({ pool_delta: -12 })], modelName)[0].newCount).toBe("-12");
  });

  it("shows an em dash — not '+0' — when there is no baseline", () => {
    // `pool_delta` is null on the first measurable run. "There is nothing to compare
    // against" is not "the pool did not change", and only one of those is
    // reassuring.
    expect(scanHistoryRows([run({ pool_delta: null })], modelName)[0].newCount).toBe("—");
  });

  it("formats the three cases identically wherever they appear", () => {
    // Shared with the meta line, so the table and the control line cannot word the
    // same number two ways.
    expect(formatPoolDelta(54)).toBe("+54");
    expect(formatPoolDelta(0)).toBe("0");
    expect(formatPoolDelta(-12)).toBe("-12");
    expect(formatPoolDelta(null)).toBe("—");
    expect(formatPoolDelta(undefined)).toBe("—");
  });
});

describe("the Status cell", () => {
  it("carries the failure REASON, not just the word Failed", () => {
    // "Failed" alone sends the reader to the logs, which is the silent-failure
    // shape Standing Rule 1 exists to prevent. The reason is stored and now served.
    const rows = scanHistoryRows(
      [run({ status: "failed", error: "vLLM offline" })],
      modelName,
    );
    expect(rows[0].status).toBe("Failed — vLLM offline");
  });

  it("still says Failed honestly when no reason was recorded", () => {
    // A row can be killed between its stub insert and the reason being written.
    // Honestly incomplete beats a fabricated cause.
    const rows = scanHistoryRows([run({ status: "failed", error: null })], modelName);
    expect(rows[0].status).toBe("Failed");
  });

  it("reads Complete for a completed run and Running for a live one", () => {
    expect(scanHistoryRows([run({ status: "completed" })], modelName)[0].status).toBe(
      "Complete",
    );
    expect(scanHistoryRows([run({ status: "running" })], modelName)[0].status).toBe("Running");
  });

  it("shows an UNKNOWN status token verbatim rather than as a success", () => {
    // Labelling a state this build does not understand as "Complete" is how a silent
    // failure gets a green tick.
    const rows = scanHistoryRows(
      [run({ status: "cancelled" as ScanRunHeader["status"] })],
      modelName,
    );
    expect(rows[0].status).toBe("cancelled");
  });
});

describe("the Model cell and the dry-run label", () => {
  it("resolves the model id through the caller's catalogue", () => {
    expect(scanHistoryRows([run()], modelName)[0].model).toBe("Claude Opus 4.8");
  });

  it("shows an unknown model id verbatim rather than blank", () => {
    expect(scanHistoryRows([run({ model_id: "some-new-model" })], modelName)[0].model).toBe(
      "some-new-model",
    );
  });

  it("flags a dry run so it is not read as work done to the pool", () => {
    // Measured on DEV: 3 of the 4 stored runs are dry runs. A table that rendered
    // them identically to real ones would tell the reader something false.
    expect(scanHistoryRows([run({ dry_run: true })], modelName)[0].dryRun).toBe(true);
    expect(scanHistoryRows([run({ dry_run: false })], modelName)[0].dryRun).toBe(false);
  });
});

describe("ordering and the disclosure summary", () => {
  it("preserves the server's newest-first order", () => {
    // The deltas were computed against that ordering, so re-sorting here would pair
    // a row with someone else's delta.
    const rows = scanHistoryRows(
      [run({ run_id: "newest" }), run({ run_id: "older" })],
      modelName,
    );
    expect(rows.map((r) => r.runId)).toEqual(["newest", "older"]);
  });

  it("counts the runs in the summary", () => {
    expect(scanHistorySummary(4)).toBe("Scan history (4)");
  });

  it("has NO summary when there are no runs, so no empty disclosure renders", () => {
    expect(scanHistorySummary(0)).toBeNull();
  });
});
