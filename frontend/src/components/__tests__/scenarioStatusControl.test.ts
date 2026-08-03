// =============================================================================
// scenarioStatusControl.test.ts — the same-segment no-op rule (task 1.7D)
// =============================================================================
//
// The segmented status control replaced a two-directional button, and with a
// segmented control a human can click the segment they are ALREADY on — something
// the old button made impossible, because it only ever offered the other
// direction.
//
// That new affordance created a new way to be wrong. The ready route writes a row
// to the status-transitions ledger, which is the forensic record of who declared a
// scenario rehearsable and when. A POST fired for a click that changes nothing
// would record a transition that never happened — and a false entry in that ledger
// is worse than a missing feature, because it is evidence about a human's act.
//
// So the guard is a correctness rule, not a visual convention, and it is a pure
// function so it can be tested without component-render infrastructure (which this
// repo deliberately does not have — CLAUDE.md Rule 30).

import { describe, expect, it } from "vitest";

import { shouldApplyStatus } from "../ScenarioStatusControl";

describe("shouldApplyStatus — the same-segment no-op guard", () => {
  it("does nothing when Draft is clicked on a scenario that is already Draft", () => {
    expect(shouldApplyStatus(false, false)).toBe(false);
  });

  it("does nothing when Ready is clicked on a scenario that is already Ready", () => {
    expect(shouldApplyStatus(true, true)).toBe(false);
  });

  it("fires when switching Draft → Ready", () => {
    // The act that puts a scenario in front of a witness. It must reach the server.
    expect(shouldApplyStatus(true, false)).toBe(true);
  });

  it("fires when switching Ready → Draft", () => {
    // Demotion must reach the server too — withdrawing a scenario from rehearsal is
    // as much a recorded human act as declaring it ready, and the ledger needs both
    // ends of the round trip or its history has gaps.
    expect(shouldApplyStatus(false, true)).toBe(true);
  });

  it("is symmetric: only the DIFFERENCE matters, not which way", () => {
    // Stated as a property so the rule cannot drift into "only promotion counts",
    // which is the shape a well-meaning optimisation would take.
    for (const [next, current] of [
      [true, false],
      [false, true],
    ] as const) {
      expect(shouldApplyStatus(next, current)).toBe(true);
    }
    for (const [next, current] of [
      [true, true],
      [false, false],
    ] as const) {
      expect(shouldApplyStatus(next, current)).toBe(false);
    }
  });
});
