import { describe, expect, it } from "vitest";

import { positionLabel, stepForKey, stepTo } from "../rehearsalNav";

describe("stepForKey", () => {
  it("moves forward on the keys a one-handed reader reaches for", () => {
    for (const key of ["ArrowRight", "ArrowDown", "PageDown", " "]) {
      expect(stepForKey(key)).toBe("next");
    }
  });

  it("moves back on their mirrors", () => {
    for (const key of ["ArrowLeft", "ArrowUp", "PageUp"]) {
      expect(stepForKey(key)).toBe("previous");
    }
  });

  it("ignores everything else, so a stray keystroke never loses the place", () => {
    for (const key of ["a", "Enter", "Escape", "Tab", "Shift"]) {
      expect(stepForKey(key)).toBeNull();
    }
  });
});

describe("stepTo", () => {
  it("advances and retreats one scenario at a time", () => {
    expect(stepTo(0, 3, "next")).toBe(1);
    expect(stepTo(2, 3, "previous")).toBe(1);
  });

  it("stops at the end rather than wrapping", () => {
    // Wrapping to the first scenario reads as a fresh start, and Marie would
    // work the list twice without noticing.
    expect(stepTo(2, 3, "next")).toBe(2);
    expect(stepTo(0, 3, "previous")).toBe(0);
  });

  it("clamps a stale index instead of indexing past the list", () => {
    // A scenario demoted while the page was open shortens the list under a
    // held index. That must land somewhere real, not on `undefined`.
    expect(stepTo(9, 3, null)).toBe(2);
    expect(stepTo(-4, 3, null)).toBe(0);
  });

  it("returns 0 for an empty list rather than -1", () => {
    expect(stepTo(0, 0, "next")).toBe(0);
    expect(stepTo(3, 0, "previous")).toBe(0);
  });
});

describe("positionLabel", () => {
  it("counts from one, the way a human says it", () => {
    expect(positionLabel(0, 5)).toBe("Scenario 1 of 5");
    expect(positionLabel(4, 5)).toBe("Scenario 5 of 5");
  });

  it("says plainly when nothing is ready", () => {
    // An empty rehearsal is a real state — nobody has declared a scenario ready
    // yet — and it must read as that rather than as "Scenario 1 of 0".
    expect(positionLabel(0, 0)).toBe("No scenarios are ready to rehearse");
  });

  it("never reports a position outside the list", () => {
    expect(positionLabel(9, 3)).toBe("Scenario 3 of 3");
  });
});
