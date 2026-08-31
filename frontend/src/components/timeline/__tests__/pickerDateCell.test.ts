// =============================================================================
// pickerDateCell.test.ts — the picker's date cell, case by case
// =============================================================================
//
// T6.5's third named suite. Every row of the table in `pickerDateCell.ts`'s doc
// comment is asserted here, plus the two edges the mockup does not draw: a date
// the browser cannot parse, and a precision string the payload invented.

import { describe, expect, it } from "vitest";

import type { ChronologyWording, TimelineEvent } from "../../../services/caseTimeline";
import { dateCell } from "../pickerDateCell";

/**
 * Only the four rows this module reads.
 *
 * `cw` throws by name on a missing key, so a fixture carrying the whole
 * chronology block would hide a module that started reading a fifth one. Four
 * keys means the test fails loudly the moment this module's appetite changes.
 */
const wording = {
  subsets_precision_month_label: "month · approx.",
  subsets_precision_year_label: "year · approx.",
  subsets_date_to_confirm_badge: "date to confirm",
} as unknown as ChronologyWording;

function event(over: Partial<TimelineEvent>): TimelineEvent {
  return {
    id: "e1",
    event_date: "2008-08-18",
    date_precision: "day",
    approximate: false,
    title: "an event",
    fact: "",
    phase: "pre-probate",
    attributes: {},
    links: [],
    ...over,
  } as TimelineEvent;
}

describe("dateCell — the mockup's four rows", () => {
  it("day precision, exact: the date alone, in ink", () => {
    const cell = dateCell(event({ event_date: "2008-08-18" }), wording);
    expect(cell.text).toBe("Aug 18, 2008");
    expect(cell.caption).toBe("");
    expect(cell.approximate).toBe(false);
  });

  it("month precision, approximate: no invented day, and the precision beneath", () => {
    // The source said "April 2009". "Apr 1, 2009" would be a day the record
    // does not contain — the class of mistake the precision vocabulary exists
    // to prevent, and the reason the caption is drawn at all.
    const cell = dateCell(
      event({ event_date: "2009-04-01", date_precision: "month", approximate: true }),
      wording,
    );
    expect(cell.text).toBe("~ Apr 2009");
    expect(cell.caption).toBe("month · approx.");
    expect(cell.approximate).toBe(true);
  });

  it("year precision, approximate: the year alone, and the precision beneath", () => {
    const cell = dateCell(
      event({ event_date: "2009-06-15", date_precision: "year", approximate: true }),
      wording,
    );
    expect(cell.text).toBe("~ 2009");
    expect(cell.caption).toBe("year · approx.");
  });

  it("day precision, approximate: the full date, and the ⚑ beneath", () => {
    // The Milster handoff. A day IS stated, and what is unsettled is whether it
    // is the right one — so the caption is the flag rather than a precision.
    const cell = dateCell(
      event({ event_date: "2009-05-03", date_precision: "day", approximate: true }),
      wording,
    );
    expect(cell.text).toBe("~ May 3, 2009");
    expect(cell.caption).toBe("⚑ date to confirm");
    expect(cell.approximate).toBe(true);
  });
});

describe("dateCell — the glyph and the word", () => {
  it("keeps the ⚑ in code and the words in the store", () => {
    const cell = dateCell(
      event({ date_precision: "day", approximate: true }),
      { ...wording, subsets_date_to_confirm_badge: "check this" } as ChronologyWording,
    );
    // Reworded in the store, the glyph is untouched: that is the whole split.
    expect(cell.caption).toBe("⚑ check this");
  });

  it("draws no caption at all on an exact date, whatever its precision", () => {
    // A month-precision date nobody marked approximate is a date the source
    // stated as a month. There is nothing to warn about, so there is no line.
    expect(dateCell(event({ date_precision: "month" }), wording).caption).toBe("");
    expect(dateCell(event({ date_precision: "year" }), wording).caption).toBe("");
  });
});

describe("dateCell — the edges the mockup does not draw", () => {
  it("shows an unparseable date rather than blanking it", () => {
    // The same degradation `formatEventDate` makes. A date the browser cannot
    // parse is still a date somebody typed, and hiding it would hide the fault.
    const cell = dateCell(event({ event_date: "not-a-date" }), wording);
    expect(cell.text).toBe("not-a-date");
    expect(cell.caption).toBe("");
  });

  it("treats an unrecognised precision as a day, in both lines", () => {
    // `formatEventDate` falls through to the full date for an unknown
    // precision. The caption falls through with it — one surface silently
    // disagreeing with the other is the drift this module exists to prevent.
    const cell = dateCell(
      event({ event_date: "2009-05-03", date_precision: "fortnight", approximate: true }),
      wording,
    );
    expect(cell.text).toBe("~ May 3, 2009");
    expect(cell.caption).toBe("⚑ date to confirm");
  });
});
