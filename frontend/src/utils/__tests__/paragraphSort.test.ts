/**
 * Tests for the shared `parseLeadingParagraph` helper.
 *
 * These tests previously lived inside
 * `frontend/src/components/__tests__/elementDetailPanelHelpers.test.ts`
 * — they moved here when the helper was extracted to `utils/` so both the
 * Element detail panel and the Evidence Explorer page can share one
 * canonical implementation (no test duplication: parse coverage lives
 * with the helper, render-helper coverage stays with the panel).
 *
 * Comparator behavior is covered indirectly by call-site tests; the
 * standalone helper only owns the parse contract.
 */
import { describe, expect, it } from "vitest";
import { parseLeadingParagraph } from "../paragraphSort";

describe("parseLeadingParagraph", () => {
  it("parses a plain integer string", () => {
    expect(parseLeadingParagraph("10")).toBe(10);
    expect(parseLeadingParagraph("73")).toBe(73);
  });

  it("returns the leading int for a range prefix", () => {
    expect(parseLeadingParagraph("16-18")).toBe(16);
    expect(parseLeadingParagraph("20-22")).toBe(20);
  });

  it("returns null for a non-numeric input", () => {
    expect(parseLeadingParagraph("abc")).toBeNull();
  });

  it("returns null for an empty string", () => {
    expect(parseLeadingParagraph("")).toBeNull();
  });

  it("returns null when the string starts with a non-digit", () => {
    expect(parseLeadingParagraph("¶7")).toBeNull();
    expect(parseLeadingParagraph("-3")).toBeNull();
    expect(parseLeadingParagraph(" 5")).toBeNull(); // leading whitespace
  });

  it("parses multi-digit leading ints correctly", () => {
    expect(parseLeadingParagraph("100")).toBe(100);
    expect(parseLeadingParagraph("100-105")).toBe(100);
  });
});
