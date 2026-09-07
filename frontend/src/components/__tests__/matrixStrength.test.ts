/**
 * Pure-helper tests for the Proof Matrix's display helpers (task 396, P1).
 *
 * No DOM / RTL — pure functions only (CLAUDE.md §30), mirroring
 * trialPrepHelpers.test.ts. What is worth locking here is the behaviour that
 * only shows up on screen: when the "×N" marker appears at all, and that an
 * unfilled template shows verbatim rather than being repaired.
 *
 * The `tierChipLabel` suite went with the function itself — PROOF_MATRIX_v2 §3
 * removed the tier chip from the drill-down, and the lookup left with its only
 * caller rather than staying as code nothing reaches.
 */
import { describe, expect, it } from "vitest";
import { duplicateMarker, fillCount } from "../matrixStrength";
import { MATRIX_WORDING_FIXTURE as wording } from "../../testFixtures/matrixWording";

describe("fillCount", () => {
  it("puts the served number into the served template", () => {
    expect(fillCount(wording.raw_approved_template, 15)).toBe("· 15 approved");
  });

  it("leaves a template with no placeholder visibly unfilled", () => {
    // A template edited to drop `{count}` is refused by the settings write path,
    // so this state means the store was edited around the API. It must show
    // verbatim and wrong rather than being silently repaired here — that is how
    // a reader finds out.
    expect(fillCount("approved", 15)).toBe("approved");
  });

  it("fills zero, which is a real reading and not an empty one", () => {
    expect(fillCount(wording.raw_approved_template, 0)).toBe("· 0 approved");
  });
});

describe("duplicateMarker", () => {
  it("marks a row that collapsed two statements into one", () => {
    expect(duplicateMarker(2, wording)).toBe("×2");
  });

  it("shows nothing for a row that collapsed nothing", () => {
    // "×1" on every line would be noise, and the count of a group of one is not
    // a fact worth printing.
    expect(duplicateMarker(1, wording)).toBeNull();
  });

  it("shows nothing for a nonsensical count rather than rendering ×0", () => {
    expect(duplicateMarker(0, wording)).toBeNull();
  });
});
