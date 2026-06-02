/**
 * Pure-helper tests for ElementAllegationList — the source-PDF locator helpers.
 * Locks the click-through URL construction (reusing the app's existing
 * /api/documents/:id/file#page=N pattern) and the human locator label across
 * the present/absent-page and missing-title branches.
 *
 * No DOM / RTL — pure functions only (project precedent: CountCard helpers).
 */
import { describe, expect, it } from "vitest";
import { locatorLabel, pdfHref } from "../ElementAllegationList";
import { API_BASE_URL } from "../../services/api";
import type { SupportingEvidence } from "../../services/elementDetailService";

const makeEvidence = (
  overrides: Partial<SupportingEvidence> = {},
): SupportingEvidence => ({
  id: "evidence-074",
  verbatim_quote: "That is my recollection.",
  page_number: 22,
  paragraph: "Q74",
  page_note: null,
  source_document_id: "doc-phillips",
  source_document_title: "Phillips Discovery Response",
  ...overrides,
});

describe("pdfHref", () => {
  it("appends a #page fragment when a page number is given", () => {
    expect(pdfHref("doc-phillips", 22)).toBe(
      `${API_BASE_URL}/api/documents/doc-phillips/file#page=22`,
    );
  });

  it("omits the fragment when the page is null", () => {
    expect(pdfHref("doc-phillips", null)).toBe(
      `${API_BASE_URL}/api/documents/doc-phillips/file`,
    );
  });

  it("URL-encodes the document id", () => {
    expect(pdfHref("doc a/b", 3)).toBe(
      `${API_BASE_URL}/api/documents/doc%20a%2Fb/file#page=3`,
    );
  });
});

describe("locatorLabel", () => {
  it("includes the page when present: '{title} · p. {n}'", () => {
    expect(locatorLabel(makeEvidence())).toBe("Phillips Discovery Response · p. 22");
  });

  it("shows the title alone when there is no page", () => {
    expect(locatorLabel(makeEvidence({ page_number: null }))).toBe(
      "Phillips Discovery Response",
    );
  });

  it("falls back to 'Source document' when the title is null", () => {
    expect(
      locatorLabel(makeEvidence({ source_document_title: null, page_number: 5 })),
    ).toBe("Source document · p. 5");
  });
});
