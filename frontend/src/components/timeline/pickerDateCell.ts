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
import { isDateToConfirm } from "../scenario-timeline/subsetRows";
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
 * ## The caption, case by case (mockup Screen 3, rows 8, 14 and the last)
 *
 * | precision | approximate | text            | caption          |
 * |-----------|-------------|-----------------|------------------|
 * | `day`     | no          | `Aug 18, 2008`  | `""`             |
 * | `month`   | yes         | `~ Apr 2009`    | `month · approx.`|
 * | `year`    | yes         | `~ 2009`        | `year · approx.` |
 * | `day`     | yes         | `~ May 3, 2009` | `⚑ to confirm`   |
 *
 * The three captions are STORED ROWS, seeded by T4 and already spoken by the
 * floating window: this is the second surface to read them, which is the point
 * of a store. The ⚑ is a glyph and stays in code, the same split the window's
 * ⧉ ⇲ – × and the order arrows' ▲▼ already make.
 *
 * ## ⚑ The flag decision is T4's, deliberately not re-made here
 *
 * `isDateToConfirm` is imported rather than re-derived because there is exactly
 * one answer to "is this date one somebody must go and confirm", and T4 recorded
 * it with its reasoning: NOTHING in the data carries a "to confirm" flag, so the
 * badge marks `approximate`, which is what the data does know. Two surfaces
 * badging on two rules would be a story that says one thing on the timeline and
 * another in the picker.
 *
 * A day-precision approximate date therefore gets the flag caption and a
 * month/year one gets its precision instead — the same three rows the mockup
 * draws, and the reason is that "month · approx." already SAYS the date is
 * unsettled, so "⚑ to confirm" under it would be the same statement twice.
 */
export function dateCell(event: TimelineEvent, wording: ChronologyWording): DateCell {
  const text = formatEventDate(event.event_date, event.approximate, event.date_precision);
  return {
    text,
    caption: captionFor(event, wording),
    approximate: event.approximate,
  };
}

/** The small line: the precision, the flag, or nothing. */
function captionFor(event: TimelineEvent, wording: ChronologyWording): string {
  if (!event.approximate) return "";
  if (event.date_precision === "month") return cw(wording, "subsets_precision_month_label");
  if (event.date_precision === "year") return cw(wording, "subsets_precision_year_label");
  // Day precision (and any precision the payload invents — the same
  // fall-through `formatEventDate` makes) with the date marked approximate.
  if (!isDateToConfirm(event)) return "";
  return `⚑ ${cw(wording, "subsets_date_to_confirm_badge")}`;
}
