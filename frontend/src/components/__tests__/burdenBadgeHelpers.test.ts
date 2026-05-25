/**
 * Pure-helper tests for BurdenBadge.
 *
 * Locks the two behind-the-pill contracts: variant selection (which token set
 * a burden value maps to) and display formatting (snake_case → readable). Per
 * §11 test-auditor: happy path, the warning/neutral boundary, and the defensive
 * default for unknown values are all covered. No DOM / RTL — pure functions.
 */
import { describe, expect, it } from "vitest";
import { burdenVariant, formatBurden } from "../BurdenBadge";

describe("burdenVariant", () => {
  it("maps preponderance to neutral", () => {
    expect(burdenVariant("preponderance")).toBe("neutral");
  });

  it("maps clear_and_convincing (backend snake_case) to warning", () => {
    expect(burdenVariant("clear_and_convincing")).toBe("warning");
  });

  it("is robust to spaces and casing", () => {
    expect(burdenVariant("Clear And Convincing")).toBe("warning");
    expect(burdenVariant("clear and convincing")).toBe("warning");
  });

  it("falls back to neutral for unrecognized values (defensive)", () => {
    expect(burdenVariant("beyond_a_reasonable_doubt")).toBe("neutral");
    expect(burdenVariant("")).toBe("neutral");
  });
});

describe("formatBurden", () => {
  it("capitalizes a single-word burden", () => {
    expect(formatBurden("preponderance")).toBe("Preponderance");
  });

  it("turns snake_case into a readable phrase (only first word capitalized)", () => {
    expect(formatBurden("clear_and_convincing")).toBe("Clear and convincing");
  });

  it("preserves already-formatted input", () => {
    expect(formatBurden("Clear and convincing")).toBe("Clear and convincing");
  });
});
