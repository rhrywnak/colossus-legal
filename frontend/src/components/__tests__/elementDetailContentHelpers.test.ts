/**
 * Pure-helper tests for ElementDetailContent (the routed Count-detail page's
 * Element body). Locks `sortAllegationsByParagraph`: numeric ascending,
 * non-numeric last, stable on ties, range-by-start, and purity.
 *
 * This is a self-contained copy of the panel's helper (the panel is removed in
 * Home instruction 2 of 2), so it carries its own test rather than relying on
 * the panel's. The underlying `parseLeadingParagraph` primitive is tested in
 * `utils/__tests__/paragraphSort.test.ts`; we exercise it transitively here.
 *
 * No DOM / RTL — pure functions only (project precedent: CountCard helpers).
 */
import { describe, expect, it } from "vitest";
import { sortAllegationsByParagraph } from "../ElementDetailContent";
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
  supporting_evidence: [],
  ...overrides,
});

describe("sortAllegationsByParagraph (ElementDetailContent)", () => {
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

  it("does not mutate the input array (purity)", () => {
    const input = [
      makeAllegation({ allegation_id: "b", paragraph_number: "73" }),
      makeAllegation({ allegation_id: "a", paragraph_number: "10" }),
    ];
    sortAllegationsByParagraph(input);
    expect(input.map((a) => a.allegation_id)).toEqual(["b", "a"]);
  });
});
