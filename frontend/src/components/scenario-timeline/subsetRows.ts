// =============================================================================
// subsetRows.ts — every decision one row of the subset window makes
// =============================================================================
//
// TIMELINE_SUBSET_MOCKUP_v2_2026-08-31.html Screen 2, approved as drawn (design
// §11 item 2). The sibling of `subsetWindow.ts`: that module decides where the
// window IS, this one decides what a row inside it SAYS. Split for the reason
// this project splits every such pair — there is no component-testing tier
// here, so a decision made inside a `.map()` is a decision no test can reach.
//
// Three decisions live here, and each one shipped wrong at least once before:
//
//  1. WHERE A DIVIDER GOES, and whether it names a phase (T3 drew phase-only
//     dividers; the year — the thing a story told in dates is organised by —
//     was nowhere on the rule).
//  2. WHICH ROWS CARRY THE ⚑, which is a question about data and not about
//     string-matching a fact.
//  3. HOW MANY of them there are, for the footer.

import type { ChronologyWording, TimelineEvent, TimelinePhase } from "../../services/caseTimeline";
import { cw, fill } from "../../services/caseTimeline";
import type { SubsetEvent } from "../../services/caseTimelineSubsets";

/**
 * Does this row carry the amber "date to confirm" badge?
 *
 * ## ⚑ `approximate` ALONE, and this is a recorded decision rather than a guess
 *
 * The mockup badges the Milster handoff because Roman entered that date from
 * recollection. NOTHING IN THE DATA SAYS SO. Measured against DEV on
 * 2026-08-31: no `chronology_events` row carries "to confirm" in its fact, no
 * `chronology_event_links` row carries it in a label or a pinpoint, and
 * `attributes` holds only `tags` and legacy `source`/`source_id`. There is no
 * flag to read.
 *
 * The instruction's own fallback is what this implements, and it is the right
 * one: string-matching "to confirm" inside a fact would make a BADGE — a claim
 * about the reliability of a date, in a story that gets quoted into a brief —
 * depend on prose somebody might reword tomorrow. A badge that appears and
 * disappears when an author fixes a typo is worse than no badge.
 *
 * So the badge marks what the data actually knows: the date is approximate.
 * On "The $50,000" that is exactly the two rows the mockup marks. A first-class
 * "date to confirm" flag on the event is the proper fix and is filed under
 * NEEDS A RULING in the T4 report; when it exists, this function is the ONE
 * place that changes.
 */
export function isDateToConfirm(event: TimelineEvent): boolean {
  return event.approximate;
}

/**
 * How many rows carry the ⚑ — the footer's second number.
 *
 * Counts LIVE rows only. A row whose event was soft-deleted off the chronology
 * is already marked as a gap and is not a date anybody can go and confirm, so
 * counting it here would inflate a number the reader is being asked to act on.
 */
export function flagCount(rows: SubsetEvent[]): number {
  return rows.filter((row) => !row.removed && isDateToConfirm(row.event)).length;
}

/**
 * The footer's right-hand line — "15 events", or "15 events · 2 ⚑".
 *
 * ## ⚑ WHY THE WORDS ARE A ROW AND THE GLYPH IS NOT
 *
 * `subsets_window_footer_events_template` carries "{count} events" because
 * "events" is a word: an editor might make it "dates", a translator would
 * certainly change it. The " · {n} ⚑" that may follow is a middle dot, a
 * number and a glyph — there is nothing in it to edit and nothing to
 * translate. That is the same split the title bar already makes, where ⧉ ⇲ –
 * and × live in code and their accessible NAMES are stored rows.
 *
 * ## The suffix is DROPPED at zero, not rendered as "· 0 ⚑"
 *
 * A zero here is not information. "15 events · 0 ⚑" invites the reader to work
 * out what the symbol would have meant if there had been any, on every story
 * that has none — which is most of them. When there is nothing to flag the
 * footer simply says how many events there are.
 *
 * `count` is every reference the subset holds, gaps included — the SAME number
 * the title bar shows. Two different counts of one story on one window is how a
 * reader stops trusting either.
 */
export function footerLine(rows: SubsetEvent[], wording: ChronologyWording): string {
  const events = fill(cw(wording, "subsets_window_footer_events_template"), {
    count: rows.length,
  });
  const flags = flagCount(rows);
  return flags === 0 ? events : `${events} · ${flags} ⚑`;
}

/** The four-digit year an event belongs to, straight off the stored ISO date. */
export function yearOf(event: TimelineEvent): string {
  return event.event_date.slice(0, 4);
}

/**
 * Does this story cross a phase boundary?
 *
 * The question the divider's SHAPE depends on — see [`dividerFor`]. Asked over
 * the whole subset once rather than per row, because "this story crosses
 * phases" is a fact about the story.
 */
export function crossesPhases(rows: SubsetEvent[]): boolean {
  const seen = new Set(rows.map((row) => row.event.phase));
  return seen.size > 1;
}

/**
 * The divider above one row, or `null` when that row needs none.
 *
 * ## ⚑ The rule, and the two ways of reading it that this settles
 *
 * Mockup Screen 2 draws a rule between YEAR changes and nothing else; design
 * §11 adds "when a story crosses phases the divider says both". Read strictly
 * per-divider, that leaves a hole: a story running pre-probate → probate inside
 * a single calendar year changes phase at a row where the year does NOT change,
 * so the phase boundary would pass with no rule at all and the reader would be
 * told nothing. T3's phase-only divider had the mirror-image hole — it never
 * said which year.
 *
 * So: a divider appears when the year changes OR the phase changes, and it
 * carries the phase for every rule in a story that crosses phases. A
 * single-phase story — which is what the mockup draws — gets bare years, "2008"
 * then "2009", exactly as drawn. There is no phase-only divider left anywhere:
 * every rule names a year, because a story told in dates is organised by them.
 *
 * `previous` is `null` for the first row, which always gets a divider.
 */
export function dividerFor(
  event: TimelineEvent,
  previous: TimelineEvent | null,
  storyCrossesPhases: boolean,
  phases: TimelinePhase[],
  wording: ChronologyWording,
): string | null {
  const year = yearOf(event);
  const changed =
    previous === null || yearOf(previous) !== year || previous.phase !== event.phase;
  if (!changed) return null;
  if (!storyCrossesPhases) return year;

  // Lower-cased as the mockup writes it. The divider's own style then upper-
  // cases it for display, so this normalises "PRE-PROBATE" and a hand-typed
  // "Probate" to the same thing before either reaches the screen.
  const label = phaseLabel(phases, event.phase).toLocaleLowerCase("en-US");
  return fill(cw(wording, "subsets_year_phase_divider_template"), { year, phase: label });
}

/**
 * One phase's label, or the slug when the payload has no row for it.
 *
 * The slug and not a blank: an event filed under a phase the payload does not
 * carry is exactly the case the timeline page's own unknown-phase row exists to
 * make visible, and a divider reading "2009 · " would hide it.
 */
export function phaseLabel(phases: TimelinePhase[], id: string): string {
  return phases.find((phase) => phase.id === id)?.label ?? id;
}

/**
 * The caption under the date — "2009 · month · approx.", "2009", or "".
 *
 * The year, the precision, or both, joined by the middle dot this app already
 * uses as inline furniture (`SubsetModal`, `ScenarioTimelineRow`). The dot is
 * punctuation and not a word: it carries no meaning a translator would change,
 * and the two things it joins are BOTH stored strings.
 *
 * Empty when there is nothing to say — a year-precision date whose year is
 * already the lead line, on an event nobody marked approximate.
 */
export function dateCaption(
  event: TimelineEvent,
  year: string,
  wording: ChronologyWording,
): string {
  const parts: string[] = [];
  if (year !== "") parts.push(year);
  if (event.approximate && event.date_precision === "month") {
    parts.push(cw(wording, "subsets_precision_month_label"));
  }
  if (event.approximate && event.date_precision === "year") {
    parts.push(cw(wording, "subsets_precision_year_label"));
  }
  return parts.join(" · ");
}
