/**
 * Pure-helper tests for ElementDetailPanel.
 *
 * Locks the panel's three pure logic surfaces:
 *  - parseLeadingParagraph: numeric prefix extraction with range support
 *    ("16-18" → 16) and the documented null fallback for non-numeric input.
 *  - sortAllegationsByParagraph: numeric ascending sort, non-numeric last,
 *    stable preservation of original order for ties.
 *  - formatElementHeader: composes "Element {N}.{M} — {name}" when count
 *    metadata is present; falls back to "Element — {name}" otherwise.
 *
 * No DOM / RTL — pure functions only. Matches the project precedent
 * (CountCard pure helpers, frontend has no jsdom setup per CLAUDE.md §30).
 */
import { describe, expect, it } from "vitest";
import {
  formatElementHeader,
  parseLeadingParagraph,
  sortAllegationsByParagraph,
} from "../ElementDetailPanel";
import type { AllegationSummary } from "../../services/elementDetailService";

const makeAllegation = (
  overrides: Partial<AllegationSummary> = {},
): AllegationSummary => ({
  allegation_id: "a-1",
  paragraph_number: "10",
  summary: null,
  title: null,
  verbatim_quote: null,
  source_section: "Common",
  ...overrides,
});

describe("parseLeadingParagraph", () => {
  it("parses a plain integer string", () => {
    expect(parseLeadingParagraph("10")).toBe(10);
    expect(parseLeadingParagraph("73")).toBe(73);
  });

  it("returns the leading int for a range prefix", () => {
    expect(parseLeadingParagraph("16-18")).toBe(16);
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
  });
});

describe("sortAllegationsByParagraph", () => {
  it("sorts numerically, not lexicographically", () => {
    const input = [
      makeAllegation({ allegation_id: "a", paragraph_number: "100" }),
      makeAllegation({ allegation_id: "b", paragraph_number: "9" }),
      makeAllegation({ allegation_id: "c", paragraph_number: "73" }),
    ];
    expect(sortAllegationsByParagraph(input).map((a) => a.allegation_id)).toEqual([
      "b",
      "c",
      "a",
    ]);
  });

  it("places non-numeric paragraph_numbers last", () => {
    const input = [
      makeAllegation({ allegation_id: "x", paragraph_number: "abc" }),
      makeAllegation({ allegation_id: "a", paragraph_number: "5" }),
    ];
    expect(sortAllegationsByParagraph(input).map((a) => a.allegation_id)).toEqual([
      "a",
      "x",
    ]);
  });

  it("preserves original order among non-numeric entries (stable)", () => {
    const input = [
      makeAllegation({ allegation_id: "x", paragraph_number: "abc" }),
      makeAllegation({ allegation_id: "y", paragraph_number: "def" }),
    ];
    expect(sortAllegationsByParagraph(input).map((a) => a.allegation_id)).toEqual([
      "x",
      "y",
    ]);
  });

  it("preserves original order among identical numeric entries (stable)", () => {
    const input = [
      makeAllegation({ allegation_id: "first", paragraph_number: "10" }),
      makeAllegation({ allegation_id: "second", paragraph_number: "10" }),
    ];
    expect(sortAllegationsByParagraph(input).map((a) => a.allegation_id)).toEqual([
      "first",
      "second",
    ]);
  });

  it("does not mutate the input array (purity)", () => {
    const input = [
      makeAllegation({ allegation_id: "b", paragraph_number: "73" }),
      makeAllegation({ allegation_id: "a", paragraph_number: "10" }),
    ];
    sortAllegationsByParagraph(input);
    expect(input.map((a) => a.allegation_id)).toEqual(["b", "a"]);
  });

  it("sorts ranges by their starting paragraph", () => {
    const input = [
      makeAllegation({ allegation_id: "later", paragraph_number: "20-22" }),
      makeAllegation({ allegation_id: "early", paragraph_number: "16-18" }),
    ];
    expect(sortAllegationsByParagraph(input).map((a) => a.allegation_id)).toEqual([
      "early",
      "later",
    ]);
  });
});

describe("formatElementHeader", () => {
  it("composes the full label when count + order are present", () => {
    expect(formatElementHeader("Fiduciary duty", 1, 1)).toBe(
      "Element 1.1 — Fiduciary duty",
    );
    expect(formatElementHeader("Breach", 2, 3)).toBe("Element 2.3 — Breach");
  });

  it("falls back when count_number is null", () => {
    expect(formatElementHeader("Orphan", null, 1)).toBe("Element — Orphan");
  });

  it("falls back when order_in_count is null", () => {
    expect(formatElementHeader("Orphan", 1, null)).toBe("Element — Orphan");
  });

  it("falls back when both are null", () => {
    expect(formatElementHeader("Orphan", null, null)).toBe("Element — Orphan");
  });
});
