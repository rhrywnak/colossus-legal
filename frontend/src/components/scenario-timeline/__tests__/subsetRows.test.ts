// =============================================================================
// subsetRows.test.ts — what one row of the subset window says
// =============================================================================
//
// T4.4 names three of these: the date-split formatter (day / month / year /
// approximate), the divider rules (single year → no phase; cross-phase →
// template filled), and the ⚑. Every assertion is a concrete expected value,
// and the date cases are the fifteen events of "The $50,000" as they sit on DEV
// — not invented shapes.

import { describe, expect, it } from "vitest";

import { splitEventDate } from "../../timeline/timelineFilters";
import type { ChronologyWording, TimelineEvent, TimelinePhase } from "../../../services/caseTimeline";
import type { SubsetEvent } from "../../../services/caseTimelineSubsets";
import {
  crossesPhases,
  dateCaption,
  dividerFor,
  flagCount,
  isDateToConfirm,
  phaseLabel,
  yearOf,
} from "../subsetRows";

/** The T4 rows, keyed as the WIRE carries them — the stored key without its
 *  `chronology_` prefix, which is what `cw` looks up. */
const WORDING: ChronologyWording = {
  subsets_precision_month_label: "month · approx.",
  subsets_precision_year_label: "year · approx.",
  subsets_year_phase_divider_template: "{year} · {phase}",
  subsets_date_to_confirm_badge: "date to confirm",
};

const PHASES: TimelinePhase[] = [
  { id: "estate", label: "PRE-PROBATE", date_range: "2008–2009", color: "#b45309", sort_order: 1 },
  { id: "probate", label: "PROBATE", date_range: "2009–2011", color: "#2563eb", sort_order: 2 },
];

function event(over: Partial<TimelineEvent>): TimelineEvent {
  return {
    id: "e1",
    event_date: "2008-08-18",
    date_precision: "day",
    approximate: false,
    phase: "estate",
    title: "$50,000 Transferred from Emil's Account",
    attributes: {},
    tags: ["financial"],
    links: [],
    note_count: 0,
    created_at: "2026-08-31T00:00:00Z",
    updated_at: "2026-08-31T00:00:00Z",
    ...over,
  };
}

function row(over: Partial<TimelineEvent>, removed = false): SubsetEvent {
  return { event: event(over), subset_note: "", removed };
}

// -----------------------------------------------------------------------------
// The date, split into the two lines the window stacks
// -----------------------------------------------------------------------------

describe("splitEventDate — the mockup's four date shapes", () => {
  it("a plain day is 'Aug 18' over '2008'", () => {
    expect(splitEventDate("2008-08-18", false, "day")).toEqual({ lead: "Aug 18", year: "2008" });
  });

  it("an APPROXIMATE day keeps the day and takes the tilde", () => {
    // The Milster handoff, as the mockup draws it: "~ May 3" over "2009".
    expect(splitEventDate("2009-05-03", true, "day")).toEqual({ lead: "~ May 3", year: "2009" });
  });

  it("a MONTH-precision date prints no day — a day would be fabricated", () => {
    // DEV stores this as 2009-04-01. Printing "Apr 1" would assert a day the
    // source never stated, which is the whole reason the precision column
    // exists. Defect D9 is this same mistake on the picker.
    expect(splitEventDate("2009-04-01", true, "month")).toEqual({ lead: "~ Apr", year: "2009" });
  });

  it("a YEAR-precision date is its own lead and leaves the year line EMPTY", () => {
    // "~ 2009" stacked over "2009" would read as two facts where there is one.
    expect(splitEventDate("2009-10-01", true, "year")).toEqual({ lead: "~ 2009", year: "" });
  });

  it("a date the browser cannot parse is SHOWN, never blanked", () => {
    expect(splitEventDate("not-a-date", false, "day")).toEqual({ lead: "not-a-date", year: "" });
  });

  it("an unknown precision falls through to a full day, like the page formatter", () => {
    expect(splitEventDate("2009-04-21", false, "decade")).toEqual({ lead: "Apr 21", year: "2009" });
  });
});

// -----------------------------------------------------------------------------
// The caption under the date
// -----------------------------------------------------------------------------

describe("dateCaption", () => {
  it("is the bare year for an ordinary day", () => {
    expect(dateCaption(event({}), "2008", WORDING)).toBe("2008");
  });

  it("names month precision beside the year", () => {
    const e = event({ event_date: "2009-04-01", date_precision: "month", approximate: true });
    expect(dateCaption(e, "2009", WORDING)).toBe("2009 · month · approx.");
  });

  it("names year precision ALONE, because the year is already the lead", () => {
    const e = event({ event_date: "2009-10-01", date_precision: "year", approximate: true });
    expect(dateCaption(e, "", WORDING)).toBe("year · approx.");
  });

  it("says nothing about precision on a date nobody marked approximate", () => {
    // A month-precision date that is NOT approximate is an exact statement of a
    // month. "approx." on it would be this surface inventing a doubt.
    const e = event({ event_date: "2009-04-01", date_precision: "month", approximate: false });
    expect(dateCaption(e, "2009", WORDING)).toBe("2009");
  });
});

// -----------------------------------------------------------------------------
// The ⚑
// -----------------------------------------------------------------------------

describe("the date-to-confirm flag", () => {
  it("marks an approximate date and nothing else", () => {
    expect(isDateToConfirm(event({ approximate: true }))).toBe(true);
    expect(isDateToConfirm(event({ approximate: false }))).toBe(false);
  });

  it("does NOT read the fact — a badge must not ride on prose", () => {
    // The recorded decision: string-matching "to confirm" inside a fact would
    // make a claim about a date's reliability depend on wording somebody could
    // reword tomorrow. Both of these are unflagged because neither is
    // approximate, whatever their text says.
    const prose = event({ approximate: false, fact: "on a date still to be confirmed ⚑" });
    expect(isDateToConfirm(prose)).toBe(false);
  });

  it("counts the flagged LIVE rows — the mockup's '2 ⚑' on The $50,000", () => {
    const rows = [
      row({ id: "a" }),
      row({ id: "b", event_date: "2009-04-01", date_precision: "month", approximate: true }),
      row({ id: "c" }),
      row({ id: "d", event_date: "2009-05-03", approximate: true }),
    ];
    expect(flagCount(rows)).toBe(2);
  });

  it("does not count a flagged row whose event is gone from the chronology", () => {
    // A gap is already marked as a gap, and its date is not one anybody can go
    // and confirm. Counting it would inflate a number the reader acts on.
    const rows = [row({ id: "a", approximate: true }, true), row({ id: "b", approximate: true })];
    expect(flagCount(rows)).toBe(1);
  });
});

// -----------------------------------------------------------------------------
// The dividers
// -----------------------------------------------------------------------------

describe("the year divider", () => {
  it("names the bare year in a ONE-PHASE story — the mockup's 2008 / 2009", () => {
    const rows = [row({ id: "a", event_date: "2008-08-18" }), row({ id: "b", event_date: "2009-03-16" })];
    expect(crossesPhases(rows)).toBe(false);
    expect(dividerFor(rows[0].event, null, false, PHASES, WORDING)).toBe("2008");
    expect(dividerFor(rows[1].event, rows[0].event, false, PHASES, WORDING)).toBe("2009");
  });

  it("gives the FIRST row a divider — a list must not start mid-year", () => {
    expect(dividerFor(event({}), null, false, PHASES, WORDING)).toBe("2008");
  });

  it("gives no divider to a row in the same year and phase as the one above", () => {
    const a = event({ id: "a", event_date: "2009-04-21" });
    const b = event({ id: "b", event_date: "2009-04-21" });
    expect(dividerFor(b, a, false, PHASES, WORDING)).toBeNull();
  });

  it("fills the template when the story crosses phases — '2009 · probate'", () => {
    const rows = [
      row({ id: "a", event_date: "2009-05-04", phase: "estate" }),
      row({ id: "b", event_date: "2009-07-23", phase: "probate" }),
    ];
    expect(crossesPhases(rows)).toBe(true);
    expect(dividerFor(rows[1].event, rows[0].event, true, PHASES, WORDING)).toBe("2009 · probate");
  });

  it("draws a rule at a phase change INSIDE one year — the hole a year-only rule left", () => {
    // Estate → probate on 2009-07-23. The year does not change, so a strict
    // year-only divider would let the phase boundary pass unmarked.
    const a = event({ id: "a", event_date: "2009-05-04", phase: "estate" });
    const b = event({ id: "b", event_date: "2009-07-23", phase: "probate" });
    expect(dividerFor(b, a, true, PHASES, WORDING)).toBe("2009 · probate");
  });

  it("names the SLUG for a phase the payload does not carry, never a blank", () => {
    const a = event({ id: "a", event_date: "2009-05-04", phase: "estate" });
    const b = event({ id: "b", event_date: "2011-01-04", phase: "appeals" });
    expect(phaseLabel(PHASES, "appeals")).toBe("appeals");
    expect(dividerFor(b, a, true, PHASES, WORDING)).toBe("2011 · appeals");
  });

  it("reads the year off the stored ISO date, not off a locale format", () => {
    expect(yearOf(event({ event_date: "2008-08-18" }))).toBe("2008");
  });
});
