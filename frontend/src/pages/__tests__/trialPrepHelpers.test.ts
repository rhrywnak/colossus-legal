/**
 * Pure-helper tests for the Trial Prep pages.
 *
 * Locks the view-shaping contracts: chronological timeline ordering (nulls
 * last, non-mutating), the grounded/anticipated split, the
 * repeat-after-rebuttal flag, and status styling.
 *
 * The pattern-flag pill's three cases were tested here until .396, when ruling
 * R2 §3 killed the chip — it read "pattern analysis pending" on every card in
 * every state, because the field behind it is hardcoded null. The helper went
 * with the chip rather than staying covered and unrendered. No DOM / RTL — pure functions only (CLAUDE.md §30),
 * mirroring proofReviewHelpers.test.ts.
 */
import { describe, expect, it } from "vitest";
import {
  isAnticipated,
  showsRepeatFlag,
  sortTimelineByDate,
  statusMeta,
} from "../trialPrepHelpers";
import type {
  ExchangeTurn,
  ScenarioStatus,
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
