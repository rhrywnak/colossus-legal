/**
 * Pure-helper tests for CaseSummaryCard.
 *
 * Locks the venue line wording and — critically — the null-safety of the
 * complaint link (Standing Rule 1: a missing complaint id must produce NO link,
 * never a broken one). No DOM / RTL — pure functions only.
 */
import { describe, expect, it } from "vitest";
import { complaintFileHref, formatVenueLine } from "../CaseSummaryCard";
import type { CaseSummaryDoc } from "../../services/caseSummaryDoc";

const doc: CaseSummaryDoc = {
  summary: "Marie Awad alleges breach of fiduciary duty.",
  venue: "Bay County Circuit Court",
  filed: "November 1, 2013",
  status: "Active (transferred from Macomb County Circuit Court)",
};

describe("formatVenueLine", () => {
  it("composes venue · Filed {date} · status", () => {
    expect(formatVenueLine(doc)).toBe(
      "Bay County Circuit Court · Filed November 1, 2013 · Active (transferred from Macomb County Circuit Court)",
    );
  });
});

describe("complaintFileHref", () => {
  it("returns the file route URL when an id is present", () => {
    const href = complaintFileHref("doc-awad-v-catholic-family-complaint-11-1-13");
    expect(href).not.toBeNull();
    expect(href).toMatch(
      /\/api\/documents\/doc-awad-v-catholic-family-complaint-11-1-13\/file$/,
    );
  });

  it("URL-encodes the id", () => {
    const href = complaintFileHref("a b/c");
    expect(href).toMatch(/\/api\/documents\/a%20b%2Fc\/file$/);
  });

  it("returns null when the id is null", () => {
    expect(complaintFileHref(null)).toBeNull();
  });

  it("returns null when the id is blank/whitespace", () => {
    expect(complaintFileHref("   ")).toBeNull();
  });
});
