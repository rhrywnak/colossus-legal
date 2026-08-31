// =============================================================================
// windowStyles.test.ts — the one DECISION hiding in a styles module
// =============================================================================
//
// `windowStyles.ts` is almost entirely transcribed geometry, and geometry does
// not get a test — a screenshot is the check for that, and the T4 report's
// REPRODUCED table is where it lives.
//
// `softTint` is different. It is a function with three branches, one of which
// exists to keep the window rendering when the DATABASE hands it something the
// other two cannot parse. The tag colours come from `chronology_tags`, a table a
// person edits; nothing stops a row holding `red`, or `var(--green)`, or a hex
// with a typo in it. That branch is why the function is exported, and an
// exported-for-testing function with no test is the shape of a promise nobody
// kept.

import { describe, expect, it } from "vitest";

import { softTint } from "../windowStyles";

describe("softTint — the pale ground under a tag pill", () => {
  it("converts a full #rrggbb, which is what the tag table actually holds", () => {
    // #059669 is the `financial` tag's stored colour on DEV, and the mockup's
    // own `--green`. This is the path that runs fifteen times on The $50,000.
    expect(softTint("#059669", 0.14)).toBe("rgba(5, 150, 105, 0.14)");
  });

  it("converts the other four stored tag colours", () => {
    expect(softTint("#2563eb", 0.14)).toBe("rgba(37, 99, 235, 0.14)");
    expect(softTint("#7c3aed", 0.14)).toBe("rgba(124, 58, 237, 0.14)");
    expect(softTint("#64748b", 0.14)).toBe("rgba(100, 116, 139, 0.14)");
    expect(softTint("#d97706", 0.14)).toBe("rgba(217, 119, 6, 0.14)");
  });

  it("expands the three-digit shorthand rather than misreading it", () => {
    // `#abc` is `#aabbcc`, not `#0a0b0c`. Reading it as the latter would tint
    // the pill nearly black and look like a rendering bug rather than a parse.
    expect(softTint("#abc", 0.5)).toBe("rgba(170, 187, 204, 0.5)");
  });

  it("is case-insensitive, because a hand-typed row will not be consistent", () => {
    expect(softTint("#B45309", 0.2)).toBe("rgba(180, 83, 9, 0.2)");
  });

  it("carries the alpha through verbatim", () => {
    expect(softTint("#000000", 0)).toBe("rgba(0, 0, 0, 0)");
    expect(softTint("#ffffff", 1)).toBe("rgba(255, 255, 255, 1)");
  });

  // ── the branch that keeps the window on screen ─────────────────────────────

  it("returns a CSS KEYWORD unchanged instead of producing nonsense", () => {
    // A tag row holding `red` is legal in the database and legal in CSS. The
    // pill loses its pale ground and is flat red — visibly plainer, still a
    // pill, and nothing throws.
    expect(softTint("red", 0.14)).toBe("red");
  });

  it("returns a var() unchanged", () => {
    expect(softTint("var(--state-success-strong)", 0.14)).toBe(
      "var(--state-success-strong)",
    );
  });

  it("returns a MALFORMED hex unchanged rather than parsing half of it", () => {
    // Five digits is the typo that matters: a regex anchored loosely would take
    // the first six characters of `#05966` + whatever followed and tint the
    // pill some unrelated colour, which nobody would ever trace back to a typo.
    expect(softTint("#05966", 0.14)).toBe("#05966");
    expect(softTint("#0596699", 0.14)).toBe("#0596699");
    expect(softTint("#gggggg", 0.14)).toBe("#gggggg");
  });

  it("returns an empty string unchanged", () => {
    // An empty colour column. The pill renders with no background, which is the
    // honest rendering of "this tag has no colour".
    expect(softTint("", 0.14)).toBe("");
  });
});
