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

import { eventDate, eventDateCaption, softTint } from "../windowStyles";

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


// -----------------------------------------------------------------------------
// The date column — the one piece of geometry that DID need a test
// -----------------------------------------------------------------------------

describe("the date caption stays inside its column", () => {
  // ⚑ The exception to "geometry does not get a test", and it earned it.
  //
  // The caption declared `white-space: nowrap` and inherited another from
  // `eventDate`. On a plain row that is invisible — "2009" is 30px of text in
  // an 85px box. On a month- or year-precision approximate row it becomes
  // "2009 · month · approx.", which needs 130px, and because the column also
  // has `overflow: visible` the surplus 45px was PAINTED ACROSS the event
  // title. One row of fifteen on "The $50,000", shipped since T4, and it
  // survived a REPRODUCED/DEVIATED pass because the measurement used to check
  // it — `getBoundingClientRect()` — returns the box and not the paint.
  //
  // These assert the three declarations that decide it. They cannot prove the
  // wrap looks right; the measured table and the screenshot in the T6 report
  // are what know that.

  it("lets the caption wrap rather than overflow", () => {
    expect(eventDateCaption.whiteSpace).toBe("normal");
  });

  it("breaks a word that cannot fit even alone", () => {
    // Nothing in the store needs this today — "approx." is 40px in an 85px box
    // — but the caption is two stored rows joined by a dot, and a reworded one
    // is a single migration away. Measured live against a 26-character token:
    // it wraps to three lines instead of overflowing.
    expect(eventDateCaption.overflowWrap).toBe("break-word");
  });

  it("keeps the second line cheap", () => {
    // 1.15 rather than the default: the wrapped row costs 13px, not 20, and
    // rows here already vary by several lines of fact text.
    expect(eventDateCaption.lineHeight).toBe(1.15);
  });

  it("leaves the DAY line above it on ONE line, always", () => {
    // "~ May 3" broken across two lines would be a different and worse defect
    // than the one the wrap fixes. Measured live: every one of the fifteen
    // leads is a single line box, the widest 53px in an 85px column.
    expect(eventDate("#059669", true).whiteSpace).toBe("nowrap");
    expect(eventDate("#059669", false).whiteSpace).toBe("nowrap");
  });
});
