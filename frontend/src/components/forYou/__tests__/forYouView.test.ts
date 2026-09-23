// =============================================================================
// forYouView.test.ts — the "For you" page's two pure decisions
// =============================================================================
//
// CC_TASK_FOR_YOU_v1 L1. There is no component-test rig in this project
// (CLAUDE.md rule 30), so the judgements worth holding down were written as
// pure functions expressly to be testable here: how rows fall under the three
// day headings, and when the menu badge is drawn at all.

import { describe, expect, it } from "vitest";

import { badgeLabel, groupByDay } from "../forYouView";
import { rowHref } from "../ForYouRow";
import type { ForYouDay, ForYouRow } from "../../../services/forYou";

function row(day: ForYouDay, id: string): ForYouRow {
  return {
    kind: "note",
    item_id: id,
    scenario_id: "s1",
    question_id: "q1",
    deck_line: "S-11 · The $50,000",
    body: "look at this one again",
    byline: "Chuck · on the question",
    when: "11:42 am",
    day,
    read: false,
  };
}

describe("grouping rows under the day headings", () => {
  it("keeps the server's order inside each group, and the headings in order", () => {
    // The server has already sorted newest first and said which day each row
    // belongs to; this only gathers them, so the order inside a group must be
    // exactly the order it was given.
    const groups = groupByDay([
      row("today", "a"),
      row("earlier", "b"),
      row("today", "c"),
      row("yesterday", "d"),
    ]);
    expect(groups.map((g) => g.day)).toEqual(["today", "yesterday", "earlier"]);
    expect(groups[0].rows.map((r) => r.item_id)).toEqual(["a", "c"]);
    expect(groups[1].rows.map((r) => r.item_id)).toEqual(["d"]);
    expect(groups[2].rows.map((r) => r.item_id)).toEqual(["b"]);
  });

  it("drops a day with nothing in it rather than rendering an empty heading", () => {
    // A "YESTERDAY" with no rows under it reads as a list that failed to load.
    const groups = groupByDay([row("earlier", "a")]);
    expect(groups.map((g) => g.day)).toEqual(["earlier"]);
  });

  it("returns nothing at all for an empty list", () => {
    // The page's own empty state is what shows then — not three bare headings.
    expect(groupByDay([])).toEqual([]);
  });
});

describe("the menu badge", () => {
  it("is ABSENT at zero, never a 0", () => {
    // A "0" beside a menu item is a thing to read and dismiss on every page
    // load. Nothing waiting should look like nothing.
    expect(badgeLabel(0)).toBeNull();
  });

  it("shows the count when something is waiting", () => {
    expect(badgeLabel(1)).toBe("1");
    expect(badgeLabel(17)).toBe("17");
  });

  it("is absent when the count is unknown or nonsense", () => {
    // `null` is what the context holds when the read FAILED. A badge is a claim
    // that work is waiting, and a claim we cannot support must not be made —
    // the failure is in the console and on the page itself, not in the menu.
    expect(badgeLabel(null)).toBeNull();
    expect(badgeLabel(undefined)).toBeNull();
    expect(badgeLabel(Number.NaN)).toBeNull();
    expect(badgeLabel(Number.POSITIVE_INFINITY)).toBeNull();
    expect(badgeLabel(-3)).toBeNull();
  });
});

describe("where a row opens", () => {
  const SLUG = "awad_v_catholic_family_service";

  it("opens the question, carrying from=for-you", () => {
    // The read-clear hangs off that query (ruling Q4): the question page fires
    // it only when the address says the reader arrived from this list.
    const href = rowHref(row("today", "a"), SLUG);
    expect(href).toBe(
      `/cases/${SLUG}/trial-prep/practice/s1/question/q1?from=for-you`,
    );
  });

  it("opens the DECK for a note about a whole scenario, and clears nothing", () => {
    // There is no question to mark read, so the row must not carry a query that
    // claims otherwise. It stays on the list until L2's sweep.
    const scenarioNote = { ...row("today", "a"), question_id: undefined };
    expect(rowHref(scenarioNote, SLUG)).toBe(
      `/cases/${SLUG}/trial-prep/practice/s1`,
    );
    expect(rowHref(scenarioNote, SLUG)).not.toContain("from=");
  });

  it("escapes every segment it is given", () => {
    // A slug or an id carrying a `/` would otherwise become an extra path
    // segment and land on a different route — the .382 defect's whole shape.
    const odd = { ...row("today", "a"), scenario_id: "s/1", question_id: "q/1" };
    const href = rowHref(odd, "awad v cfs/2");
    expect(href).toContain("awad%20v%20cfs%2F2");
    expect(href).toContain("s%2F1");
    expect(href).toContain("q%2F1");
  });
});
