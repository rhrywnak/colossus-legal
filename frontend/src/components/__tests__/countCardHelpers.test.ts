/**
 * Pure-helper tests for CountCard.
 *
 * Locks the two display-name and authority-text fallback contracts that
 * back the Element rows on the Home page. The rule (§11 test auditor):
 * every exported pure helper has a unit test for happy path, fallback
 * path, and the boundary between them.
 *
 * No DOM / RTL — pattern matches `configurationPanelHelpers.test.ts`.
 */
import { describe, expect, it } from "vitest";
import {
  resolveAuthorityText,
  resolveElementDisplayName,
} from "../CountCard";
import type { ElementInfo } from "../../services/caseSummary";

const makeElement = (overrides: Partial<ElementInfo> = {}): ElementInfo => ({
  id: "el-1",
  element_name: "breach_of_duty",
  title: "Breach of Duty",
  order_in_count: 1,
  allegation_count: 0,
  ...overrides,
});

// Placeholder text is the contract — repeated here verbatim so a change to
// the prod string deliberately breaks this test.
const PLACEHOLDER =
  "Authority pending review of canonical Element library.";

describe("resolveElementDisplayName", () => {
  it("returns title when title is non-empty (happy path)", () => {
    const element = makeElement({ title: "Breach of Duty" });
    expect(resolveElementDisplayName(element, 0)).toBe("Breach of Duty");
  });

  it("falls back to element_name when title is whitespace-only", () => {
    const element = makeElement({ title: "   ", element_name: "breach_of_duty" });
    expect(resolveElementDisplayName(element, 0)).toBe("breach_of_duty");
  });

  it("falls back to element_name when title is empty string", () => {
    const element = makeElement({ title: "", element_name: "damages" });
    expect(resolveElementDisplayName(element, 0)).toBe("damages");
  });

  it("falls back to positional label when both title and element_name are blank", () => {
    const element = makeElement({ title: "", element_name: "" });
    expect(resolveElementDisplayName(element, 0)).toBe("Element 1");
    expect(resolveElementDisplayName(element, 2)).toBe("Element 3");
  });

  it("treats whitespace-only element_name as blank for positional fallback", () => {
    const element = makeElement({ title: "", element_name: "   " });
    expect(resolveElementDisplayName(element, 4)).toBe("Element 5");
  });
});

describe("resolveAuthorityText", () => {
  it("returns the controlling authority when present (happy path)", () => {
    expect(resolveAuthorityText("M Civ JI 27.05")).toBe("M Civ JI 27.05");
  });

  it("trims surrounding whitespace from a present authority", () => {
    expect(resolveAuthorityText("  Smith v. Jones  ")).toBe("Smith v. Jones");
  });

  it("returns the placeholder when the field is undefined", () => {
    expect(resolveAuthorityText(undefined)).toBe(PLACEHOLDER);
  });

  it("returns the placeholder when the field is an empty string", () => {
    expect(resolveAuthorityText("")).toBe(PLACEHOLDER);
  });

  it("returns the placeholder when the field is whitespace-only", () => {
    expect(resolveAuthorityText("   ")).toBe(PLACEHOLDER);
  });
});
