/**
 * Pure-helper tests for the Theme Scan panel (formatting + agreement).
 */
import { describe, expect, it } from "vitest";

import {
  collapsedCardSummary,
  collapsedScanSummary,
  computeAgreement,
  costLabel,
  formatCost,
  formatElapsed,
  formatRunTimestamp,
  lastRunSummary,
} from "../themeScanFormat";
import type { ScanRunHeader, ThemeScanSummary } from "../../services/themeScan";
import { run, wording } from "./scanHeaderFixtures";

function summary(overrides: Partial<ThemeScanSummary>): ThemeScanSummary {
  return {
    run_id: "r",
    model_id: "m",
    input_tokens: null,
    output_tokens: null,
    computed_cost: null,
    duration_ms: 0,
    candidates_read: 0,
    relevant: 0,
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
    // A pick with no folded twin — the ordinary case (task 2.15).
    covers_node_ids: [id],
    duplicate_count: 1,
    // Backend-annotated at read time; the agreement maths ignores both, but the
    // fixture must be a real ThemeScanSuggestion.
    ordinal: null,
    applied: false,
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

describe("formatCost", () => {
  it("shows a dash for null (local model / no usage), else a 4-dp $ figure", () => {
    expect(formatCost(null)).toBe("—");
    expect(formatCost(0)).toBe("$0.0000");
    expect(formatCost(0.0125)).toBe("$0.0125");
  });
});

describe("formatRunTimestamp", () => {
  it("renders a parseable ISO timestamp as a compact non-empty label", () => {
    const label = formatRunTimestamp("2026-07-16T14:32:00Z");
    // Locale/timezone vary by environment, so assert it produced SOMETHING other
    // than the raw ISO string rather than pinning an exact rendering.
    expect(label).not.toBe("2026-07-16T14:32:00Z");
    expect(label.length).toBeGreaterThan(0);
  });
  it("degrades to the raw string on an unparseable value (no throw)", () => {
    expect(formatRunTimestamp("not-a-date")).toBe("not-a-date");
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

// ── The scan control's last-run summary (task 1.7B) ─────────────────────────

describe("lastRunSummary", () => {
  it("reads the newest COMPLETED run — the backend already ordered them", () => {
    const summary = lastRunSummary([
      { status: "completed", candidates_total: 148, started_at: "2026-08-02T09:14:00Z" },
      { status: "completed", candidates_total: 12, started_at: "2026-07-30T09:14:00Z" },
    ]);
    expect(summary).toContain("148 candidates");
  });

  // ── Task R1 Piece 3: the line that described a scan that never worked ──────

  it("SKIPS a newer failed run and summarises the last one that completed", () => {
    // The measured defect (audit 10). `fail_scan_run` writes `status` and `error`
    // and leaves `candidates_total` exactly where promote-time put it, so a run
    // that died mid-judging sits at index 0 carrying a full-looking count. The
    // old `runs[0]` rendered it as "104 candidates · …" — a description of a scan
    // that worked, about one that did not.
    const summary = lastRunSummary([
      { status: "failed", candidates_total: 104, started_at: "2026-08-09T20:43:00Z" },
      { status: "completed", candidates_total: 148, started_at: "2026-07-29T18:29:00Z" },
    ]);
    expect(summary).toContain("148 candidates");
    expect(summary).not.toContain("104");
  });

  it("SKIPS a run that is still going", () => {
    // A running run has a denominator and no verdict. Describing it as the last
    // scan would claim a result that does not exist yet; the RunningView above is
    // what speaks for a run in flight.
    const summary = lastRunSummary([
      { status: "running", candidates_total: 104, started_at: "2026-08-10T01:52:00Z" },
      { status: "completed", candidates_total: 148, started_at: "2026-07-29T18:29:00Z" },
    ]);
    expect(summary).toContain("148 candidates");
  });

  it("says NOTHING when the only run failed", () => {
    // Not "0 candidates", and not the failed run's own numbers dressed as a
    // success. The collapsed card carries the failure's own sentence
    // (`collapsedFailedSummary`); this line stays silent rather than adding a
    // second, differently-worded account of the same failure to the same screen.
    expect(
      lastRunSummary([
        { status: "failed", candidates_total: 104, started_at: "2026-08-09T20:43:00Z" },
      ]),
    ).toBeNull();
  });

  it("says nothing when no scan has run", () => {
    // "0 candidates" would describe a scan that never happened.
    expect(lastRunSummary([])).toBeNull();
  });

  it("says nothing when the newest completed run has no count yet", () => {
    expect(
      lastRunSummary([{ status: "completed", candidates_total: null, started_at: "2026-08-02T09:14:00Z" }]),
    ).toBeNull();
  });

  // ── Task 1.7C, §2.3 / ruling R2: the meta line gained two clauses ──────────

  it("adds the pool delta, signed, when there is a baseline to compare against", () => {
    // Ruling R2's wording, ratified: "+54 since the previous scan". The delta is
    // computed BACKEND-side (Standing Rule 12); this only words it.
    const summary = lastRunSummary([
      { status: "completed", candidates_total: 148, started_at: "2026-08-02T09:14:00Z", pool_delta: 54 },
    ]);
    expect(summary).toContain("+54 since the previous scan");
  });

  it("keeps a negative delta rather than hiding a shrinking pool", () => {
    // After task 2.5's re-anchoring the pool can legitimately shrink, and a -12
    // that says so is worth more than a 0 that hides it.
    const summary = lastRunSummary([
      { status: "completed", candidates_total: 136, started_at: "2026-08-02T09:14:00Z", pool_delta: -12 },
    ]);
    expect(summary).toContain("-12 since the previous scan");
  });

  it("OMITS the delta clause entirely on the first measurable run", () => {
    // The honesty rule R2 turns on, and the frontend half of the five backend
    // tests in `scan_run_delta_tests.rs`: `pool_delta: null` means "there is
    // nothing to compare against", which is NOT "the pool did not change". Never
    // "+0", and no orphaned " · " left behind by the missing clause either.
    const summary = lastRunSummary([
      { status: "completed", candidates_total: 148, started_at: "2026-08-02T09:14:00Z", pool_delta: null },
    ]);
    expect(summary).toContain("148 candidates");
    expect(summary).not.toContain("since the previous scan");
    expect(summary).not.toContain("+0");
    expect(summary).not.toContain(" ·  · ");
  });

  it("shows a real zero delta — an unchanged pool IS a measurement", () => {
    const summary = lastRunSummary([
      { status: "completed", candidates_total: 148, started_at: "2026-08-02T09:14:00Z", pool_delta: 0 },
    ]);
    expect(summary).toContain("0 since the previous scan");
  });

  it("adds the model name through the caller's catalogue", () => {
    // §2.3's example meta line carries the model. Resolved by the panel's own
    // catalogue rather than a lookup here, so this module holds no vocabulary.
    const summary = lastRunSummary(
      [
        {
          status: "completed",
          candidates_total: 148,
          started_at: "2026-08-02T09:14:00Z",
          pool_delta: 54,
          model_id: "claude-opus-4-8",
        },
      ],
      (id) => (id === "claude-opus-4-8" ? "Claude Opus 4.8" : id),
    );
    expect(summary).toContain("Claude Opus 4.8");
  });

  it("omits the model clause when no catalogue was supplied", () => {
    // The parameter is optional so the pre-1.7C callers and their tests keep
    // working; without it the clause is simply absent, not a raw model id.
    const summary = lastRunSummary([
      {
        status: "completed",
        candidates_total: 148,
        started_at: "2026-08-02T09:14:00Z",
        model_id: "claude-opus-4-8",
      },
    ]);
    expect(summary).not.toContain("claude-opus-4-8");
  });
});

// ─── The collapsed scan card (piece 4a, 2026-08-08) ─────────────────────────

describe("scan_card_collapsed_summary_reports_run_model_and_proposed_count", () => {
  const TEMPLATE = "Last scan {when} · {model} · {count} proposed";

  it("names all three facts, so nobody has to expand the card to learn them", () => {
    // The card folds by default once a run has completed, because the work has
    // moved to the queue below it. This line is the only thing most readers will
    // see of that run — if it withheld any of the three, the fold would cost a
    // click to answer a question the summary exists to answer.
    const line = collapsedScanSummary(TEMPLATE, "Aug 7, 9:37 PM", "Claude Opus 4.8", 30);

    expect(line).toBe("Last scan Aug 7, 9:37 PM · Claude Opus 4.8 · 30 proposed");
    expect(line).not.toContain("{");
  });

  it("says zero rather than an em dash when nothing is proposed", () => {
    // On a collapsed card "0 proposed" is true and useful — the run finished and
    // there is nothing waiting. A dash would read as "not measured", which is a
    // different (and here false) claim.
    expect(collapsedScanSummary(TEMPLATE, "Aug 7", "Qwen", null)).toContain("0 proposed");
  });

  it("renders whatever the stored template says, inventing nothing", () => {
    // The configuration law: Roman can re-word this line on the Settings page and
    // it takes effect on the next read. A composer that hard-coded the separators
    // or the order would silently ignore the edit.
    expect(collapsedScanSummary("{count} waiting — {model}, {when}", "Aug 7", "Qwen", 4)).toBe(
      "4 waiting — Qwen, Aug 7",
    );
  });
});

describe("which run the collapsed card describes", () => {
  // The branching that used to sit inline in `ThemeScanPanel`'s JSX and moved
  // here on 2026-09-11. Extracting it is what made it testable; these are the
  // four answers it can give.
  const words = wording();
  const when = () => "Sep 11, 12:06 AM";
  const name = (id: string) => (id === "qwen" ? "Qwen3.8 27B" : id);
  const line = (runs: ScanRunHeader[], proposedCount: number | null = 30) =>
    collapsedCardSummary({ runs, wording: words, proposedCount, formatWhen: when, modelName: name });

  it("says nothing until the words have loaded", () => {
    // The absent-not-fake law. A card with no stored template renders no
    // summary, rather than one composed here.
    expect(
      collapsedCardSummary({
        runs: [run()],
        wording: null,
        proposedCount: 30,
        formatWhen: when,
        modelName: name,
      }),
    ).toBeNull();
  });

  it("says nothing when no run has settled", () => {
    // A running run is not a summary — the card shows its progress instead, and
    // `null` here is what keeps the card expanded while it works.
    expect(line([run({ status: "running" })])).toBeNull();
    expect(line([])).toBeNull();
  });

  it("describes the newest run when it FAILED, not the last one that worked", () => {
    // Ruling R3: a run whose every judged call failed records `failed`, so it is
    // invisible to a search for the latest COMPLETED run. It is still the run
    // the human just watched, and describing the one before it would report a
    // scan that worked to someone who had just seen one fail.
    const summary = line([
      run({ status: "failed", failed_count: 104, model_id: "qwen" }),
      run({ status: "completed", model_id: "opus" }),
    ]);
    expect(summary).toContain("Failed");
    expect(summary).toContain("Qwen3.8 27B");
    expect(summary).not.toContain("opus");
  });

  it("describes the latest COMPLETED run when the newest settled one is not failed", () => {
    // A cancelled run settles but proposes nothing, so the collapsed card still
    // describes the completed run behind it. (The facts HEADER makes the other
    // choice and names the cancelled one — two surfaces, two questions: this one
    // is "what is in the queue below", the header's is "what happened last".)
    const summary = line([
      run({ status: "cancelled", model_id: "qwen" }),
      run({ status: "completed", model_id: "opus" }),
    ]);
    expect(summary).toBe("Last scan Sep 11, 12:06 AM · opus · 30 proposed");
  });

  it("leaves no placeholder unfilled in either branch", () => {
    for (const runs of [[run({ status: "failed" })], [run({ status: "completed" })]]) {
      expect(line(runs)).not.toMatch(/\{[a-z]+\}/);
    }
  });
});
