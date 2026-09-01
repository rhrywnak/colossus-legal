// =============================================================================
// pickerDateCell.ts — how one date reads in the picker (mockup Screen 3 `.pk .d`)
// =============================================================================
//
// T6.2, defects D1 and D9. The picker used to render `event.event_date` — the
// raw ISO string, in muted 11.5 px — beside a title in full ink, which is the
// wrong emphasis for a screen whose whole job is putting events in date order.
// Worse, it printed "2009-04-01" for an event whose source said only "April
// 2009": a day the record does not contain, invented by a formatter.
//
// ## ⚑ THE FORMAT IS THE TIMELINE PAGE'S OWN, NOT A SECOND OPINION
//
// The text comes from `formatEventDate` in `timelineFilters.ts` — the same
// function the timeline page and the event card already call. Writing "Aug 18,
// 2008" here with a fresh `toLocaleDateString` would have been a second opinion
// about what a month-precision date says, and the two would have drifted the
// first time somebody changed a locale option in one of them. This module owns
// only what the picker adds: the CAPTION under the date, and whether the date
// is amber.
//
// ## Why this is a module and not four lines inside the row
//
// This project has no component-testing tier (CLAUDE.md rule 30). A decision
// made inside `renderRow` is a decision no test can reach, and "which caption
// does a year-precision approximate date get" is exactly the kind of decision
// that is quietly wrong for a month before anybody notices.

import type { ChronologyWording, TimelineEvent } from "../../services/caseTimeline";
import { cw } from "../../services/caseTimeline";
import { formatEventDate } from "./timelineFilters";

/** The two lines of the date cell, and whether they are drawn amber. */
export type DateCell = {
  /** The big line — "Aug 18, 2008", "~ Apr 2009", "~ 2009". */
  text: string;
  /** The small line under it, or "" when there is nothing to add. */
  caption: string;
  /** Amber, and it is a claim about the DATE rather than about the event. */
  approximate: boolean;
};

/**
 * How one event's date reads in a picker row.
 *
 * ## The caption, case by case (mockup Screen 3, as amended by T6 round two)
 *
 * | precision | approximate | text            | caption          |
 * |-----------|-------------|-----------------|------------------|
 * | `day`     | no          | `Aug 18, 2008`  | `""`             |
 * | `month`   | yes         | `~ Apr 2009`    | `month · approx.`|
 * | `year`    | yes         | `~ 2009`        | `year · approx.` |
 * | `day`     | yes         | `~ May 3, 2009` | `""`  ← was `⚑`  |
 *
 * Both captions are STORED ROWS, seeded by T4 and already spoken by the
 * floating window: this is the second surface to read them, which is the point
 * of a store.
 *
 * ## ⚑ The ⚑ "date to confirm" caption was a third case here, and is retired
 *
 * T6.2 shipped a day-precision approximate date with "⚑ date to confirm" under
 * it, reusing T4's flag. Roman removed it on 2026-08-31, reversing his own T4
 * ruling: the flag could only read `approximate`, so it claimed four of the
 * case's thirty-one events needed a date confirmed — two of which nobody has
 * ever flagged — and spreading the ⚑ that thinly destroyed the signal it
 * exists to carry.
 *
 * The full reasoning, and the predicate that would bring the badge back if a
 * real "date to confirm" column ever lands on `chronology_events`, live
 * together at `isDateToConfirm` in `subsetRows.ts`. That function is
 * deliberately kept and deliberately unread; this module no longer imports it,
 * so the two surfaces would get the badge back together or not at all.
 */
export function dateCell(event: TimelineEvent, wording: ChronologyWording): DateCell {
  const text = formatEventDate(event.event_date, event.approximate, event.date_precision);
  return {
    text,
    caption: captionFor(event, wording),
    approximate: event.approximate,
  };
}

/**
 * The small line: the precision, or nothing.
 *
 * A date is captioned only when it is APPROXIMATE and the source stated less
 * than a day — "month · approx." says the record gives a month and no more, so
 * "Apr 1, 2009" would be a fabricated day. That is the class of mistake the
 * precision vocabulary exists to prevent, and this caption is where a reader is
 * told.
 *
 * An approximate DAY-precision date gets no caption. It is still drawn amber
 * and still reads "~ May 3, 2009", which says everything the record supports:
 * a day IS stated and it is approximate. Until T6 round two it also wore
 * "⚑ date to confirm", a claim nothing in the data could back — see the module
 * header, and `isDateToConfirm` in `subsetRows.ts` for the function that would
 * bring it back if a real column ever lands.
 */
function captionFor(event: TimelineEvent, wording: ChronologyWording): string {
  if (!event.approximate) return "";
  if (event.date_precision === "month") return cw(wording, "subsets_precision_month_label");
  if (event.date_precision === "year") return cw(wording, "subsets_precision_year_label");
  // Day precision, and any precision the payload invents — the same
  // fall-through `formatEventDate` makes, so the two never disagree.
  return "";
}
