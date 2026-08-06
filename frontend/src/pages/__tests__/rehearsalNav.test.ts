import { describe, expect, it } from "vitest";

import { positionAt, stepForKey, stepTo } from "../rehearsalNav";

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

describe("positionAt", () => {
  it("picks the sentence the backend composed for that index", () => {
    // The words are the store's now (task 2.11 B2). This module picks; it does
    // not compose — the off-by-one it used to guard is guarded by the backend
    // generating 1..=total, so an index can only look one up.
    const positions = ["Scenario 1 of 3", "Scenario 2 of 3", "Scenario 3 of 3"];
    expect(positionAt(0, positions)).toBe("Scenario 1 of 3");
    expect(positionAt(2, positions)).toBe("Scenario 3 of 3");
  });

  it("clamps a stale index rather than returning nothing", () => {
    // A scenario demoted while the page was open leaves the index past the end.
    // Clamping keeps a sentence on screen; `undefined` would blank the line with
    // nothing saying why.
    const positions = ["Scenario 1 of 2", "Scenario 2 of 2"];
    expect(positionAt(9, positions)).toBe("Scenario 2 of 2");
    expect(positionAt(-3, positions)).toBe("Scenario 1 of 2");
  });

  it("returns null when nothing is ready, so the caller shows its own notice", () => {
    // An empty rehearsal is a real state with its own stored sentence. Inventing
    // a position line for it would be composing prose about nothing.
    expect(positionAt(0, [])).toBeNull();
  });
});
