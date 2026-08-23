// The two dates on Chuck's sheets.
//
// ## Why these assert STRUCTURE and not exact strings
//
// Both helpers format through the browser's own locale, so an exact-string
// assertion would pass on the machine that wrote it and fail on a build agent in
// another region — a test that measures the runner rather than the code. What is
// worth pinning is the DECISION: that an unparseable date withdraws the line
// instead of printing "Invalid Date" onto paper.

import { describe, expect, it } from "vitest";

import { asPrintedAt, asSheetDate } from "../practicePrintFormat";

describe("the deck's own date", () => {
  it("formats a real timestamp into something a person reads", () => {
    const shown = asSheetDate("2026-08-19T14:51:20Z");
    expect(shown).not.toBeNull();
    expect(shown).toContain("2026");
    // Day and month are present in some locale order; the year alone is not a date.
    expect(shown!.length).toBeGreaterThan(4);
  });

  it("withdraws the line when there is no date at all", () => {
    // An unseeded deck has no questions and therefore no MAX(updated_at).
    expect(asSheetDate(null)).toBeNull();
  });

  it("withdraws the line rather than printing “Invalid Date” — the guard", () => {
    // `new Date("nonsense")` is a Date whose toLocaleDateString is the STRING
    // "Invalid Date". Without this guard that phrase prints in the sheet header,
    // on paper, and reads as though the deck itself were broken.
    for (const rubbish of ["nonsense", "", "2026-13-45", "not-a-date"]) {
      expect(asSheetDate(rubbish), `"${rubbish}" should withdraw the line`).toBeNull();
    }
  });

  it("never returns the literal “Invalid Date” for any input it accepts", () => {
    // ANTI-VACUITY for the test above: a guard that returned the string would
    // still pass three `toBeNull` checks if it only caught the cases listed.
    for (const input of [null, "nonsense", "2026-08-19T14:51:20Z", "2026-02-30"]) {
      expect(asSheetDate(input)).not.toBe("Invalid Date");
    }
  });
});

describe("when the copy was printed", () => {
  it("carries the date AND the time", () => {
    const at = new Date("2026-08-22T09:12:00");
    const shown = asPrintedAt(at);
    expect(shown).toContain("2026");
    // The time is the half that distinguishes two copies taken the same day —
    // which is the case that matters, because Chuck reprints after editing.
    expect(shown).toMatch(/\d{1,2}:\d{2}/);
  });

  it("distinguishes two copies taken minutes apart", () => {
    const morning = asPrintedAt(new Date("2026-08-22T09:12:00"));
    const later = asPrintedAt(new Date("2026-08-22T14:47:00"));
    expect(morning).not.toBe(later);
  });
});
