/**
 * Pure-helper tests for the Trial Prep pages.
 *
 * Locks the view-shaping contracts: the pattern-flag pill (null vs 0 vs >0),
 * the scenario meta line, chronological timeline ordering (nulls last,
 * non-mutating), the grounded/anticipated split, the repeat-after-rebuttal flag,
 * and status styling. No DOM / RTL — pure functions only (CLAUDE.md §30),
 * mirroring proofReviewHelpers.test.ts.
 */
import { describe, expect, it } from "vitest";
import {
  isAnticipated,
  patternFlagText,
  scenarioMetaLine,
  showsRepeatFlag,
  sortTimelineByDate,
  statusMeta,
} from "../trialPrepHelpers";
import type {
  ExchangeTurn,
  ScenarioStatus,
  ScenarioSummary,
} from "../trialPrepData";

const makeTurn = (overrides: Partial<ExchangeTurn> = {}): ExchangeTurn => ({
  kind: "accusation",
  grounded: true,
  speaker: "George Phillips",
  date: "2025-03-01",
  text: "An accusation.",
  relationship_type: "characterizes",
  source_document: "doc-x",
  page_number: 4,
  paragraph: "¶4",
  repeated_after_rebuttal: false,
  ...overrides,
});

const makeSummary = (
  overrides: Partial<ScenarioSummary> = {},
): ScenarioSummary => ({
  id: "s1",
  attack: "An attack",
  status: "draft",
  instance_count: 4,
  response_count: 2,
  speakers: ["George Phillips", "CFS"],
  baseless_repeat_count: 0,
  ...overrides,
});

describe("patternFlagText", () => {
  it("null → 'pattern analysis pending', muted (distinct from clean)", () => {
    expect(patternFlagText(null)).toEqual({
      text: "pattern analysis pending",
      muted: true,
    });
  });

  it("0 → 'no baseless repeat yet', muted", () => {
    expect(patternFlagText(0)).toEqual({
      text: "no baseless repeat yet",
      muted: true,
    });
  });

  it("> 0 → emphasized 'repeated N× after rebuttal'", () => {
    expect(patternFlagText(3)).toEqual({
      text: "repeated 3× after rebuttal",
      muted: false,
    });
  });
});

describe("scenarioMetaLine", () => {
  it("formats 'N instances · speakers · N responses'", () => {
    expect(scenarioMetaLine(makeSummary())).toBe(
      "4 instances · George Phillips, CFS · 2 responses",
    );
  });

  it("handles no speakers without producing an empty segment", () => {
    expect(scenarioMetaLine(makeSummary({ speakers: [] }))).toBe(
      "4 instances · no speakers yet · 2 responses",
    );
  });
});

describe("sortTimelineByDate", () => {
  it("orders by date ascending", () => {
    const turns = [
      makeTurn({ date: "2025-05-19", text: "c" }),
      makeTurn({ date: "2025-01-15", text: "a" }),
      makeTurn({ date: "2025-04-02", text: "b" }),
    ];
    expect(sortTimelineByDate(turns).map((t) => t.text)).toEqual(["a", "b", "c"]);
  });

  it("sorts null-date (anticipated) turns last", () => {
    const turns = [
      makeTurn({ date: null, grounded: false, text: "projected" }),
      makeTurn({ date: "2025-01-15", text: "recorded" }),
    ];
    expect(sortTimelineByDate(turns).map((t) => t.text)).toEqual([
      "recorded",
      "projected",
    ]);
  });

  it("does not mutate the input array (purity)", () => {
    const turns = [
      makeTurn({ date: "2025-05-19", text: "c" }),
      makeTurn({ date: "2025-01-15", text: "a" }),
    ];
    sortTimelineByDate(turns);
    expect(turns.map((t) => t.text)).toEqual(["c", "a"]);
  });

  it("keeps both turns when both dates are null (multiple anticipated turns)", () => {
    // A scenario may carry more than one projected/defense_counter turn; the
    // both-null comparator branch must keep both, never drop or throw.
    const turns = [
      makeTurn({ date: null, grounded: false, text: "p1" }),
      makeTurn({ date: null, grounded: false, text: "p2" }),
    ];
    const out = sortTimelineByDate(turns);
    expect(out).toHaveLength(2);
    expect(out.map((t) => t.text).sort()).toEqual(["p1", "p2"]);
  });
});

describe("isAnticipated", () => {
  it("is true for a non-grounded turn", () => {
    expect(isAnticipated(makeTurn({ grounded: false }))).toBe(true);
  });
  it("is false for a grounded turn", () => {
    expect(isAnticipated(makeTurn({ grounded: true }))).toBe(false);
  });
});

describe("showsRepeatFlag", () => {
  it("is true only for an accusation_repeat that postdates a rebuttal", () => {
    expect(
      showsRepeatFlag(
        makeTurn({ kind: "accusation_repeat", repeated_after_rebuttal: true }),
      ),
    ).toBe(true);
  });

  it("is false for a repeat that does not postdate a rebuttal", () => {
    expect(
      showsRepeatFlag(
        makeTurn({ kind: "accusation_repeat", repeated_after_rebuttal: false }),
      ),
    ).toBe(false);
  });

  it("is false for a plain accusation even if the flag is set", () => {
    expect(
      showsRepeatFlag(
        makeTurn({ kind: "accusation", repeated_after_rebuttal: true }),
      ),
    ).toBe(false);
  });
});

describe("statusMeta", () => {
  it("maps each status to its exact label and token color", () => {
    // Lock the full contract per status so a label typo or a token rename is
    // caught (the dashboard dot and the detail header both read these).
    expect(statusMeta("draft")).toEqual({
      label: "Draft",
      color: "var(--text-muted)",
    });
    expect(statusMeta("needs_evidence")).toEqual({
      label: "Needs evidence",
      color: "var(--state-warning-strong)",
    });
    expect(statusMeta("ready")).toEqual({
      label: "Ready",
      color: "var(--state-success-strong)",
    });
  });

  it("returns a design-token color for every status", () => {
    const statuses: ScenarioStatus[] = ["draft", "needs_evidence", "ready"];
    for (const s of statuses) {
      expect(statusMeta(s).color).toMatch(/^var\(--/);
    }
  });
});
