/**
 * Pure-helper tests for ElementDetailPanel.
 *
 * Locks the panel's two render-helper surfaces:
 *  - sortAllegationsByParagraph: numeric ascending sort, non-numeric last,
 *    stable preservation of original order for ties.
 *  - formatElementHeader: composes "Element {N}.{M} — {name}" when count
 *    metadata is present; falls back to "Element — {name}" otherwise.
 *
 * The underlying `parseLeadingParagraph` primitive moved to
 * `utils/paragraphSort.ts` and is tested there (see
 * `utils/__tests__/paragraphSort.test.ts`). The sort tests below still
 * exercise it transitively, which is the right level of coverage for a
 * helper that *composes* `parseLeadingParagraph` — we don't re-test the
 * primitive here.
 *
 * No DOM / RTL — pure functions only. Matches the project precedent
 * (CountCard pure helpers, frontend has no jsdom setup per CLAUDE.md §30).
 */
import { describe, expect, it } from "vitest";
import {
  formatElementHeader,
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
