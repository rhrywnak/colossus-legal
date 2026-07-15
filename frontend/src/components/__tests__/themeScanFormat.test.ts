/**
 * Pure-helper tests for the Theme Scan panel (formatting + agreement).
 */
import { describe, expect, it } from "vitest";

import { computeAgreement, costLabel, formatElapsed } from "../themeScanFormat";
import type { ThemeScanSummary } from "../../services/themeScan";

function summary(overrides: Partial<ThemeScanSummary>): ThemeScanSummary {
  return {
    run_id: "r",
    model_id: "m",
    dry_run: true,
    input_tokens: null,
    output_tokens: null,
    computed_cost: null,
    duration_ms: 0,
    candidates_read: 0,
    relevant_written: 0,
    irrelevant: 0,
    failed: 0,
    suggestions: [],
    rejected_sample: [],
    ...overrides,
  };
}

function sug(id: string, role: string) {
  return {
    graph_node_id: id,
    proposed_role: role,
    reason: "r",
    confidence: 0.9,
    content: { evidence_id: id, title: "", pattern_tags: [], about: [] },
  };
}

describe("formatElapsed", () => {
  it("formats mm:ss with a zero-padded seconds field", () => {
    expect(formatElapsed(0)).toBe("0:00");
    expect(formatElapsed(9_000)).toBe("0:09");
    expect(formatElapsed(65_000)).toBe("1:05");
    expect(formatElapsed(600_000)).toBe("10:00");
  });
  it("clamps negatives to 0:00", () => {
    expect(formatElapsed(-500)).toBe("0:00");
  });
});

describe("costLabel", () => {
  it("shows a dash for a local model with no cost, else a $ figure", () => {
    expect(costLabel(summary({ computed_cost: null }))).toBe("—");
    expect(costLabel(summary({ computed_cost: 0.1234 }))).toBe("$0.1234");
  });
});

describe("computeAgreement", () => {
  it("is 100% when both relevant sets are empty", () => {
    expect(computeAgreement(summary({}), summary({})).relevantPct).toBe(100);
  });
  it("computes Jaccard of the relevant sets and role agreement on the overlap", () => {
    const a = summary({ suggestions: [sug("n1", "supports"), sug("n2", "rebuts")] });
    const b = summary({ suggestions: [sug("n1", "supports"), sug("n3", "contradicts")] });
    // relevant union = {n1,n2,n3}=3, intersection={n1}=1 → 33%
    const r = computeAgreement(a, b);
    expect(r.relevantPct).toBe(33);
    expect(r.sharedCount).toBe(1);
    expect(r.rolePct).toBe(100); // n1: supports == supports
  });
  it("reports role disagreement on the shared set", () => {
    const a = summary({ suggestions: [sug("n1", "supports")] });
    const b = summary({ suggestions: [sug("n1", "rebuts")] });
    const r = computeAgreement(a, b);
    expect(r.relevantPct).toBe(100);
    expect(r.rolePct).toBe(0);
  });
});
