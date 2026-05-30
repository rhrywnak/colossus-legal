/**
 * Pure-helper tests for CaseSummaryCard.
 *
 * Locks — critically — the null-safety of the complaint link (Standing Rule 1:
 * a missing complaint id must produce NO link, never a broken one). No DOM /
 * RTL — pure functions only.
 */
import { describe, expect, it } from "vitest";
import { complaintFileHref } from "../CaseSummaryCard";

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
