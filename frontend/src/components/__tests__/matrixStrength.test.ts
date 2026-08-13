/**
 * Pure-helper tests for the Proof Matrix's display helpers (task 396, P1).
 *
 * No DOM / RTL — pure functions only (CLAUDE.md §30), mirroring
 * trialPrepHelpers.test.ts. What is worth locking here is the behaviour that
 * only shows up on screen: which tokens produce a chip, which produce nothing,
 * and when the "×N" marker appears at all.
 */
import { describe, expect, it, vi } from "vitest";
import {
  duplicateMarker,
  fillCount,
  tierChipLabel,
} from "../matrixStrength";
import type { MatrixWording } from "../../services/causesOfAction";

const wording: MatrixWording = {
  strong_column_label: "Strong support",
  raw_approved_template: "· {count} approved",
  strong_hint: "Sworn admissions by the other side, and the court's own findings.",
  tier_strong_chip: "Their own words",
  tier_hedged_chip: "Qualified",
  tier_other_chip: "Our sworn word",
  duplicate_template: "×{count}",
  ranked_list_note: "Strongest first",
};

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

describe("tierChipLabel", () => {
  it("maps each known token to its stored label", () => {
    expect(tierChipLabel("strong", wording)).toBe("Their own words");
    expect(tierChipLabel("hedged", wording)).toBe("Qualified");
    expect(tierChipLabel("other", wording)).toBe("Our sworn word");
  });

  it("returns nothing for an unmapped pair, silently", () => {
    // `null` is the backend saying "the stored tier map makes no claim about
    // this one". That is a normal state, not a fault: the row still renders and
    // is still counted, it just carries no chip. No warning.
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    expect(tierChipLabel(null, wording)).toBeNull();
    expect(warn).not.toHaveBeenCalled();
    warn.mockRestore();
  });

  it("returns nothing for an unknown token, and says so in the console", () => {
    // A tier a newer backend defines and this build does not. The row must not
    // print the raw token — that would put the database's vocabulary in front of
    // Chuck — but an operator should be able to see the frontend is behind.
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    expect(tierChipLabel("devastating", wording)).toBeNull();
    expect(warn).toHaveBeenCalledTimes(1);
    expect(warn.mock.calls[0][0]).toContain("devastating");
    warn.mockRestore();
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
