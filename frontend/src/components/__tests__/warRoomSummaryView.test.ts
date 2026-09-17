// =============================================================================
// warRoomSummaryView.test.ts — the summary card's numbers, words and flags
// =============================================================================
//
// Pure tests (CLAUDE.md rule 30). The wording fixture is the migrations' own
// values, so every expected string is what DEV will print.

import { describe, expect, it } from "vitest";

import { candidatesCell, reviewCell, unansweredCell, warRoomSummaryView } from "../warRoomSummaryView";
import { formatCardDay } from "../warRoomCardView";
import { bareProgress, s1, warRoomWording } from "./warRoomFixtures";
import type { ScenarioProgress, ScenarioSummary } from "../../pages/trialPrepData";

/** A scenario with just the numbers a test cares about. */
function sc(code: string, over: Partial<ScenarioProgress>): ScenarioSummary {
  return { ...s1(), id: code, code, progress: { ...bareProgress(), ...over } };
}
const ans = (total: number, of: number) => ({ ...bareProgress().answered, total, of });

const METRICS = { scenarios: 11, ready: 11, drafted_or_review: 0 };

describe("the top row", () => {
  it("carries the three metrics and the answered figure with its percentage", () => {
    const view = warRoomSummaryView(
      { metrics: METRICS, scenarios: [sc("S-1", { answered: ans(42, 42) }), sc("S-2", { answered: ans(66, 143) })] },
      warRoomWording,
    );
    expect(view.metrics).toEqual([
      { value: 11, label: "Scenarios" },
      { value: 11, label: "Ready" },
      { value: 0, label: "Draft" },
    ]);
    expect(view.answered).toBe(108);
    expect(view.answeredRest).toBe("of 185 · 58%");
    expect(view.fraction).toBeCloseTo(108 / 185);
  });

  it("is 0% with no questions — never NaN", () => {
    const view = warRoomSummaryView({ metrics: METRICS, scenarios: [] }, warRoomWording);
    expect(view.answeredRest).toBe("of 0 · 0%");
    expect(view.fraction).toBe(0);
  });
});

describe("Marie's cell — unanswered questions", () => {
  it("counts across scenarios and lists up to three untouched codes", () => {
    const cell = unansweredCell(
      [
        sc("S-1", { answered: ans(42, 42) }),
        sc("S-5", { answered: ans(4, 15) }),
        sc("S-11", { answered: ans(0, 12) }),
        sc("S-12", { answered: ans(0, 8) }),
        sc("S-13", { answered: ans(0, 8) }),
        sc("S-14", { answered: ans(0, 8) }),
        sc("S-15", { answered: ans(0, 0) }), // no deck: never "untouched"
      ],
      warRoomWording,
    );
    expect(cell.count).toBe(11 + 12 + 8 + 8 + 8);
    expect(cell.context).toBe("across 5 scenarios · S-11, S-12, S-13 untouched");
    expect(cell.warning).toBe(false);
    expect(cell.chip).toBe("Marie");
  });

  it("drops the untouched clause when every deck has an answer, and is singular at one", () => {
    const cell = unansweredCell([sc("S-5", { answered: ans(4, 15) })], warRoomWording);
    expect(cell.context).toBe("across 1 scenario");
  });

  it("appends the SUM of the cards' new-or-changed pills, and suppresses it at 0", () => {
    const cell = unansweredCell(
      [
        sc("S-5", { answered: ans(4, 15), marie_changed: 2 }),
        sc("S-7", { answered: ans(1, 16), marie_changed: 3 }),
      ],
      warRoomWording,
    );
    expect(cell.context).toBe("across 2 scenarios · 5 new or changed");
    expect(unansweredCell([sc("S-5", { answered: ans(4, 15) })], warRoomWording).context).toBe(
      "across 1 scenario",
    );
  });

  it("says every question is answered at zero", () => {
    expect(unansweredCell([sc("S-1", { answered: ans(42, 42) })], warRoomWording).context).toBe(
      "every question answered",
    );
  });
});

describe("the reviewer's cell — answers requiring review", () => {
  it("names the oldest waiting day and the largest pile, in warning ink, chip from the settings row", () => {
    const cell = reviewCell(
      [
        sc("S-1", { awaiting_review: 42, oldest_awaiting_review: "2026-09-15T15:00:00Z" }),
        sc("S-5", { awaiting_review: 4, oldest_awaiting_review: "2026-08-19T15:00:00Z" }),
        sc("S-7", { awaiting_review: 0, oldest_awaiting_review: null }),
      ],
      warRoomWording,
    );
    expect(cell.count).toBe(46);
    expect(cell.context).toBe(`oldest waiting since ${formatCardDay("2026-08-19T15:00:00Z")} · S-1 has 42`);
    expect(cell.warning).toBe(true);
    expect(cell.chip).toBe("Chuck");
  });

  it("says nothing waiting at zero, in ink", () => {
    const cell = reviewCell([sc("S-1", {})], warRoomWording);
    expect([cell.context, cell.warning]).toEqual(["nothing waiting", false]);
  });
});

describe("Roman's cell — candidates to rule", () => {
  it("lists the four largest piles, ties sharing one pile", () => {
    const cell = candidatesCell(
      [
        sc("S-7", { candidates_to_rule: 34 }),
        sc("S-11", { candidates_to_rule: 34 }),
        sc("S-12", { candidates_to_rule: 72 }),
        sc("S-13", { candidates_to_rule: 33 }),
        sc("S-14", { candidates_to_rule: 97 }),
        sc("S-2", { candidates_to_rule: 5 }),
        sc("S-1", { candidates_to_rule: 0 }),
      ],
      warRoomWording,
    );
    expect(cell.count).toBe(34 + 34 + 72 + 33 + 97 + 5);
    expect(cell.context).toBe("S-14 97 · S-12 72 · S-7 & S-11 34 · S-13 33");
    expect(cell.warning).toBe(true);
  });

  it("says nothing to rule at zero", () => {
    expect(candidatesCell([sc("S-1", {})], warRoomWording).context).toBe("nothing to rule");
  });
});

describe("every line is filled", () => {
  it("leaves no brace in any rendered string", () => {
    const view = warRoomSummaryView(
      {
        metrics: METRICS,
        scenarios: [
          sc("S-1", { answered: ans(1, 3), awaiting_review: 1, oldest_awaiting_review: "2026-09-15T15:00:00Z", candidates_to_rule: 2 }),
          sc("S-2", { answered: ans(0, 2) }),
        ],
      },
      warRoomWording,
    );
    const strings = [view.answeredRest, ...view.cells.flatMap((c) => [c.label, c.chip, c.context])];
    for (const line of strings) expect(line, line).not.toContain("{");
    expect(view.cells.map((c) => c.owner)).toEqual(["marie", "reviewer", "roman"]);
  });
});
