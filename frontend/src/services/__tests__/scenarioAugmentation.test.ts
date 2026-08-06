/**
 * Tests for `scenarioAugmentation`'s pure helpers — task 2.11 C, ruling C4b.
 *
 * The two substitutions these sections perform. Both are the same shape as
 * `rehearsalSections.fillCode` and both fail the same silent way: a template
 * whose slot is never filled prints "{cap}" to a human, and one filled with the
 * wrong number states a limit the product does not enforce.
 */
import { describe, expect, it } from "vitest";

import { fillCap, fillN } from "../scenarioAugmentation";

describe("fillCap", () => {
  it("puts the served cap into the stored sentence", () => {
    expect(fillCap("her own words · up to {cap}", 3)).toBe("her own words · up to 3");
  });

  it("states the limit in the refusal too, so a disabled control says why", () => {
    // A control that refuses without naming the ceiling reads as a broken one.
    expect(fillCap("That is already {cap} points — the most a witness can hold.", 3)).toBe(
      "That is already 3 points — the most a witness can hold.",
    );
  });

  it("leaves a template with no slot untouched rather than appending", () => {
    // The settings write path refuses a value that drops {cap}, so this state
    // means the store was edited around the API. The honest render is the
    // sentence as written — not one with a number bolted onto the end.
    expect(fillCap("her own words", 3)).toBe("her own words");
  });

  it("renders a cap of zero rather than treating it as absent", () => {
    // `0` is falsy in JavaScript, which is how a substitution written with `||`
    // silently produces "up to " — a limit stated and then withheld.
    expect(fillCap("up to {cap}", 0)).toBe("up to 0");
  });
});

describe("fillN", () => {
  it("names each editing box by its own row number", () => {
    // Without {n} every box in the list announces itself identically to a screen
    // reader, which is the same as none of them being labelled.
    expect(fillN("Talking point {n}", 2)).toBe("Talking point 2");
  });

  it("is 1-based, matching the pill and the write route's segment", () => {
    // The number a human reads, the number the aria-label says, and the number
    // `PUT …/talking-points/:position` matches must be one number — or editing
    // point 2 lands on point 1.
    expect(fillN("Talking point {n}", 1)).toBe("Talking point 1");
  });
});
