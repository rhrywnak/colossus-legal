// =============================================================================
// filteredCounter.test.ts — the §9 honesty rule, owned once (task 1.7E, R1)
// =============================================================================
//
// The rule these assert is the one lifted out of `BiasExplorerFilters`: the
// wording follows the human's INTENT, never the arithmetic accident that the
// filtered count happens to equal the total. The Bias Explorer's own tests still
// assert the same sentences through its wrapper, which is the point — one rule,
// two callers, both covered.

import { describe, expect, it } from "vitest";

import { filteredCounterLine } from "../shared/filteredCounter";

const CANDIDATES = { singular: "candidate", plural: "candidates" };

describe("filteredCounterLine", () => {
  it("says 'Filtered: X of Y' while a filter is active", () => {
    expect(filteredCounterLine(24, 148, true, CANDIDATES)).toBe(
      "Filtered: 24 of 148 candidates",
    );
  });

  it("says 'Showing all Y' when nothing is filtered", () => {
    expect(filteredCounterLine(148, 148, false, CANDIDATES)).toBe("Showing all 148 candidates");
  });

  it("drives the wording by intent, not by numerical equality", () => {
    // THE RULE. A filter that happens to match everything must still read as a
    // filter — otherwise the line tells the reader the filter did nothing, when in
    // fact it selected all of it.
    expect(filteredCounterLine(148, 148, true, CANDIDATES)).toBe(
      "Filtered: 148 of 148 candidates",
    );
  });

  it("honours the caller's singular when the total is one", () => {
    expect(filteredCounterLine(1, 1, false, CANDIDATES)).toBe("Showing all 1 candidate");
    expect(filteredCounterLine(0, 1, true, CANDIDATES)).toBe("Filtered: 0 of 1 candidate");
  });

  it("uses the noun it is given rather than one of its own", () => {
    // The helper owns the RULE; the caller owns the WORD. A second surface with a
    // different noun must not have to touch this module.
    expect(filteredCounterLine(3, 9, true, { singular: "instance", plural: "instances" })).toBe(
      "Filtered: 3 of 9 instances",
    );
  });

  it("pluralises on the TOTAL, not on the filtered count", () => {
    // "Filtered: 1 of 9 candidates" — the noun belongs to the denominator, which is
    // what the sentence is about.
    expect(filteredCounterLine(1, 9, true, CANDIDATES)).toBe("Filtered: 1 of 9 candidates");
  });
});
