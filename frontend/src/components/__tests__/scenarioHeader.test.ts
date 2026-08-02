// =============================================================================
// scenarioHeader.test.ts — the lean header's descriptor (task 1.7B)
// =============================================================================

import { describe, expect, it } from "vitest";

import { directionChip, headerDescriptor } from "../scenarioHeader";

describe("the direction chip", () => {
  it("reads in plain words, not the stored token", () => {
    expect(directionChip("defense").label).toBe("Defensive");
    expect(directionChip("offense").label).toBe("Offensive");
  });

  it("says why it cannot be changed, so nobody clicks it expecting an editor", () => {
    const chip = directionChip("defense");
    expect(chip.title).toContain("cannot be changed");
  });

  it("shows an unrecognised direction verbatim rather than defaulting", () => {
    // Mapping an unknown token to "Defensive" would be the page inventing a
    // posture for a scenario whose stored one it does not understand.
    const chip = directionChip("counter-attack");
    expect(chip.label).toBe("counter-attack");
    expect(chip.title).toContain("does not recognise");
  });
});

describe("the header line", () => {
  const base = {
    code: "S-2",
    name: "Refused to divide property amicably",
    direction: "defense",
    status: "draft" as const,
  };

  it("carries the code, the name and both chips", () => {
    const header = headerDescriptor(base);
    expect(header.code).toBe("S-2");
    expect(header.name).toBe("Refused to divide property amicably");
    expect(header.direction.label).toBe("Defensive");
    expect(header.status.label).toBe("Draft");
  });

  it("reads the status label from the shared vocabulary", () => {
    expect(headerDescriptor({ ...base, status: "ready" }).status.label).toBe("Ready");
    expect(headerDescriptor({ ...base, status: "needs_evidence" }).status.label).toBe(
      "Needs evidence",
    );
  });

  /**
   * THE ONE THAT MATTERS. §2.1 leaves a slot for the readiness verdict and says
   * "show nothing fake". Task 2.4 fills it; until then the header must render
   * NOTHING there — not "Unknown", not a grey placeholder. A verdict is a claim
   * about whether this scenario can be taken into a courtroom.
   */
  it("shows no readiness verdict until 2.4 computes one", () => {
    expect(headerDescriptor(base).readiness).toBeNull();
  });
});
