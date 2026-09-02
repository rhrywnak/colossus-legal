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
 * Is this a date somebody must go and confirm?
 *
 * ## ⚑ NOTHING RENDERS THIS TODAY, AND THAT IS DELIBERATE
 *
 * It had one caller — the window's ⚑ badge, and later the picker's — and Roman
 * retired both on 2026-08-31, reversing the T4 ruling that created them. The
 * reason is worth keeping beside the function, because the function is the
 * thing that would have to change:
 *
 * The badge was to mark a date entered from RECOLLECTION — the Milster handoff.
 * Nothing in the data records that. Measured against DEV on 2026-08-31: no
 * `chronology_events` row carries "to confirm" in its fact, no
 * `chronology_event_links` row carries it in a label or a pinpoint, and
 * `attributes` holds only `tags` and legacy `source`/`source_id`. So T4 badged
 * the nearest thing the data does know, `approximate`, and on "The $50,000"
 * that happened to be exactly the two rows the mockup marks.
 *
 * T6.2 put the same badge on the picker, which draws the WHOLE chronology, and
 * the cost showed: four of thirty-one events wore "date to confirm", two of
 * which nobody has ever flagged. A badge that makes a false claim about the
 * record is worse than no badge.
 *
 * It is KEPT, unrendered and tested, on Roman's instruction: when a real
 * "date to confirm" column lands on `chronology_events`, this is the ONE place
 * that changes, and the two surfaces get it back together or not at all. A
 * function nobody calls is normally something to delete — this one is a
 * recorded decision with a named successor, which is the exception.
 */
export function isDateToConfirm(event: TimelineEvent): boolean {
  return event.approximate;
}

/**
 * The footer's right-hand line — "15 events".
 *
 * ## The "· n ⚑" half is GONE (Roman's ruling, 2026-08-31)
 *
 * T4 built this to read "15 events · 2 ⚑". The second number counted rows whose
 * date was `approximate`, wearing a glyph that claims somebody must confirm
 * them — see [`isDateToConfirm`] for why that claim was not one the data could
 * make. The count went with the badge.
 *
 * ## Why the words are still a row and the count still is not
 *
 * `subsets_window_footer_events_template` carries "{count} events" because
 * "events" is a word: an editor might make it "dates", a translator would
 * certainly change it. The number is a number.
 *
 * `count` is every reference the subset holds, gaps included — the SAME number
 * the title bar shows. Two different counts of one story on one window is how a
 * reader stops trusting either.
 */
export function footerLine(rows: SubsetEvent[], wording: ChronologyWording): string {
  return fill(cw(wording, "subsets_window_footer_events_template"), { count: rows.length });
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
 *
 * ## ⚑ A LINE MAY BEGIN WITH "·" AND MAY NEVER END WITH ONE
 *
 * The date column is 96 px and this caption WRAPS — `eventDateCaption` sets
 * `whiteSpace: normal` deliberately, since the T6 overflow fix. With ordinary
 * spaces either side of the dot the browser is free to break after it, and
 * "2009 · month · approx." came back on DEV as a line ending in a bare
 * "month ·" — a dot introducing a word that is not there yet.
 *
 * So every separator is rebuilt with a NO-BREAK space AFTER the dot and an
 * ordinary one before it: the break can still happen before the dot, and the
 * dot then travels to the next line with the word it introduces.
 *
 * ⚑ Done on the JOINED string, not on the join alone. The stored precision
 * labels carry their own " · " inside them ("month · approx."), and those dots
 * wrap on exactly the same rule — fixing this here covers them without editing
 * two rows in the wording store, where the character would be invisible to
 * anybody reading the value.
 */
/**
 * The caption's separator: space, middle dot, NO-BREAK space (U+00A0).
 *
 * STRUCTURAL: a typographic rule about where a line may break, not a setting.
 * A deployment that could change it could put a bare "·" at the end of a line,
 * which is the defect this constant exists to close.
 */
const SEPARATOR = " ·\u00a0";

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
  return parts.join(SEPARATOR).replaceAll(" · ", SEPARATOR);
}
