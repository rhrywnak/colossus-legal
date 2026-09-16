// =============================================================================
// warRoomCardView.test.ts — the status card's strings and colour flags
// =============================================================================
//
// Pure tests of `warRoomCardView` (CLAUDE.md rule 30). The wording fixture is the
// migration's own values, so every expected string is what DEV will print.

import { describe, expect, it } from "vitest";

import { formatCardDay, warRoomCardView } from "../warRoomCardView";
import { bareProgress, s13, warRoomWording } from "./warRoomFixtures";

function bare() {
  return { ...s13(), theme_statement: null, progress: bareProgress() };
}

describe("warRoomCardView — EVIDENCE", () => {
  it("renders the never-scanned line, in the warning colour, when no scan has run", () => {
    const view = warRoomCardView(bare(), warRoomWording, false);
    expect(view.scanLine).toBe("Scan: never run");
    expect(view.scanWarning).toBe(true);
  });

  it("renders the last scan from its template, not in the warning colour", () => {
    const view = warRoomCardView(s13(), warRoomWording, false);
    expect(view.scanLine).toBe(
      `Scan: Qwen3.8 · ${formatCardDay("2026-09-11T15:00:00Z")} · 43 relevant of 273`,
    );
    expect(view.scanWarning).toBe(false);
  });

  it("renders 0 candidates WITHOUT the warning colour", () => {
    const view = warRoomCardView(s13({ candidates_to_rule: 0 }), warRoomWording, false);
    const candidates = view.evidenceRows.find((r) => r.label === "Candidates to rule");
    expect(candidates).toEqual({ label: "Candidates to rule", value: "0", warning: false });
  });

  it("renders candidates above 0 in the warning colour", () => {
    const view = warRoomCardView(s13(), warRoomWording, false);
    const candidates = view.evidenceRows.find((r) => r.label === "Candidates to rule");
    expect(candidates).toEqual({ label: "Candidates to rule", value: "33", warning: true });
  });

  it("renders Matrix linked as N of M, and an em dash when nothing is stuck (Q4)", () => {
    const linked = warRoomCardView(s13(), warRoomWording, false).evidenceRows[2];
    expect(linked.value).toBe("4 of 99");
    const none = warRoomCardView(bare(), warRoomWording, false).evidenceRows[2];
    expect(none).toEqual({ label: "Matrix linked", value: "—", warning: false });
  });
});

describe("warRoomCardView — PREP & REHEARSAL", () => {
  it("renders the deck and the answered line as the mockup draws them", () => {
    const view = warRoomCardView(s13(), warRoomWording, false);
    expect(view.prepRows.map((r) => [r.label, r.value])).toEqual([
      ["Talking points", "5"],
      ["Watch items", "5"],
      ["Deck", `17 questions · ${formatCardDay("2026-09-14T15:00:00Z")}`],
    ]);
    expect(view.answered).toEqual({
      count: "0 of 17",
      split: "answered · Chuck 0/7 · defense 0/10",
      fraction: 0,
    });
  });

  it("renders a scenario with nothing yet as zeroes and dashes — never a blank", () => {
    const view = warRoomCardView(bare(), warRoomWording, false);
    for (const r of [...view.evidenceRows, ...view.prepRows]) {
      expect(r.value, r.label).not.toBe("");
      expect(r.value, r.label).not.toContain("{");
    }
    expect(view.prepRows[2].value).toBe("—");
    expect(view.answered, "no answered line on an empty deck").toBeNull();
  });

  it("shows the amber changed pill when questions are new or changed for Marie", () => {
    expect(warRoomCardView(s13(), warRoomWording, false).badge).toEqual({
      kind: "changed",
      text: "3 new or changed for Marie",
    });
  });

  it("shows the green up-to-date pill when nothing is owed", () => {
    expect(warRoomCardView(s13({ marie_changed: 0 }), warRoomWording, false).badge).toEqual({
      kind: "up_to_date",
      text: "Up to date",
    });
  });

  it("fills the bar by the answered fraction", () => {
    const view = warRoomCardView(
      s13({
        answered: {
          total: 12,
          of: 42,
          chuck_answered: 12,
          chuck_total: 12,
          defense_answered: 0,
          defense_total: 30,
        },
      }),
      warRoomWording,
      false,
    );
    expect(view.answered?.fraction).toBeCloseTo(12 / 42);
    expect(view.answered?.split).toBe("answered · Chuck 12/12 · defense 0/30");
  });
});

describe("warRoomCardView — identity and actions", () => {
  it("renders no Timeline link for a scenario no subset carries", () => {
    expect(warRoomCardView(s13(), warRoomWording, false).actions.timeline).toBeNull();
    expect(warRoomCardView(s13(), warRoomWording, true).actions.timeline).toBe("Timeline");
  });

  it("carries the four action words from the store", () => {
    expect(warRoomCardView(s13(), warRoomWording, true).actions).toEqual({
      open: "Open scenario",
      practice: "Practice",
      timeline: "Timeline",
      delete: "Delete",
    });
  });

  it("renders no theme line when the theme is absent or blank (Q3)", () => {
    expect(warRoomCardView(bare(), warRoomWording, false).theme).toBeNull();
    expect(
      warRoomCardView({ ...s13(), theme_statement: "   " }, warRoomWording, false).theme,
    ).toBeNull();
  });
});
