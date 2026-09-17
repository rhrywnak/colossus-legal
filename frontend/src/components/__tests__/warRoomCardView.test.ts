// =============================================================================
// warRoomCardView.test.ts — the status card's strings and colour flags
// =============================================================================
//
// Pure tests of `warRoomCardView` (CLAUDE.md rule 30). The wording fixture is the
// migrations' own values, so every expected string is what DEV will print.

import { describe, expect, it } from "vitest";

import { formatCardDay, warRoomCardView } from "../warRoomCardView";
import { bareProgress, s1, s11, warRoomWording } from "./warRoomFixtures";

function bare() {
  return { ...s1(), theme_statement: null, progress: bareProgress() };
}

describe("warRoomCardView — EVIDENCE", () => {
  it("renders Never scanned, in the warning colour, when no scan has run", () => {
    const view = warRoomCardView(bare(), warRoomWording, false);
    expect(view.scanLine).toBe("Never scanned");
    expect(view.scanWarning).toBe(true);
  });

  it("renders the scan line as the DATE ONLY — no model, no relevant count", () => {
    const view = warRoomCardView(s11(), warRoomWording, false);
    expect(view.scanLine).toBe(`Last scan ${formatCardDay("2026-08-29T15:00:00Z")}`);
    expect(view.scanLine).not.toMatch(/relevant|\{/);
    expect(view.scanWarning).toBe(false);
  });

  it("renders candidates above 0 in the warning colour, and 0 without it", () => {
    const owed = warRoomCardView(s11(), warRoomWording, false).evidenceRows[1];
    expect(owed).toEqual({ label: "Candidates to rule", value: "34", warning: true });
    const none = warRoomCardView(s1(), warRoomWording, false).evidenceRows[1];
    expect(none.warning).toBe(false);
  });

  it("renders Matrix linked as N of M, and an em dash when nothing is stuck (Q4)", () => {
    expect(warRoomCardView(s11(), warRoomWording, false).evidenceRows[2].value).toBe("4 of 99");
    expect(warRoomCardView(bare(), warRoomWording, false).evidenceRows[2].value).toBe("—");
  });
});

describe("warRoomCardView — PREP & REHEARSAL", () => {
  it("makes the answered count the headline, with ONE muted line under it", () => {
    const view = warRoomCardView(s1(), warRoomWording, false);
    expect(view.answered).toEqual({
      count: "42 of 42",
      word: "answered",
      meta: `Chuck 12/12 · defense 30/30 · deck 42 q · ${formatCardDay("2026-09-15T15:00:00Z")}`,
      fraction: 1,
    });
  });

  it("renders no headline numbers for a scenario with no deck — the em dash instead", () => {
    const view = warRoomCardView(bare(), warRoomWording, false);
    expect(view.answered).toBeNull();
    expect(view.deckNone).toBe("—");
  });

  it("carries no talking-points or watch-items row any more", () => {
    const view = warRoomCardView(s1(), warRoomWording, false);
    expect(JSON.stringify(view)).not.toMatch(/Talking points|Watch items/);
  });

  it("fills the bar by the answered fraction", () => {
    const view = warRoomCardView(
      s1({ answered: { total: 10, of: 16, chuck_answered: 10, chuck_total: 10, defense_answered: 0, defense_total: 6 } }),
      warRoomWording,
      false,
    );
    expect(view.answered?.fraction).toBeCloseTo(10 / 16);
  });
});

describe("warRoomCardView — badges", () => {
  it("says Not started (gray) at 0 answered — never Up to date", () => {
    const view = warRoomCardView(s11(), warRoomWording, false);
    expect(view.badges).toEqual([{ kind: "not_started", text: "Not started" }]);
  });

  it("shows the reviewer's amber pill — global, naming the reviewer — and suppresses it at 0", () => {
    expect(warRoomCardView(s1(), warRoomWording, false).badges).toEqual([
      { kind: "review", text: "42 answers awaiting Chuck's review" },
    ]);
    const kinds = warRoomCardView(s1({ awaiting_review: 0, marie_changed: 2 }), warRoomWording, false)
      .badges.map((b) => b.kind);
    expect(kinds).toEqual(["marie"]);
  });

  it("prints the reviewer's display name from the payload, not a literal", () => {
    const wording = { ...warRoomWording, reviewer_display_name: "Pat" };
    expect(warRoomCardView(s1(), wording, false).badges[0].text).toBe("42 answers awaiting Pat's review");
  });

  it("shows Marie's amber pill and suppresses it at 0", () => {
    const view = warRoomCardView(s1({ marie_changed: 3, awaiting_review: 0 }), warRoomWording, false);
    expect(view.badges).toEqual([{ kind: "marie", text: "3 new or changed for Marie" }]);
  });

  it("shows both amber pills when both are owed, review first", () => {
    const view = warRoomCardView(s1({ marie_changed: 1 }), warRoomWording, false);
    expect(view.badges.map((b) => b.kind)).toEqual(["review", "marie"]);
  });

  it("shows green Up to date only when started and nothing pending", () => {
    const view = warRoomCardView(s1({ awaiting_review: 0 }), warRoomWording, false);
    expect(view.badges).toEqual([{ kind: "up_to_date", text: "Up to date" }]);
  });
});

describe("warRoomCardView — identity and actions", () => {
  it("renders no Timeline link for a scenario no subset carries", () => {
    expect(warRoomCardView(s1(), warRoomWording, false).actions.timeline).toBeNull();
    expect(warRoomCardView(s1(), warRoomWording, true).actions.timeline).toBe("Timeline");
  });

  it("carries the three action words from the store — no Open scenario action", () => {
    expect(warRoomCardView(s1(), warRoomWording, true).actions).toEqual({
      practice: "Practice →",
      timeline: "Timeline",
      delete: "Delete",
    });
  });

  it("renders no theme line when the theme is absent or blank (Q3)", () => {
    expect(warRoomCardView(bare(), warRoomWording, false).theme).toBeNull();
    expect(warRoomCardView({ ...s1(), theme_statement: "   " }, warRoomWording, false).theme).toBeNull();
  });
});
