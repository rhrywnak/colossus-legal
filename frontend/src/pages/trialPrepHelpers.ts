// =============================================================================
// trialPrepHelpers.ts — pure view-shaping for the Trial Prep pages
// -----------------------------------------------------------------------------
// All shaping the dashboard / scenario pages need (pattern-flag pill text, the
// scenario meta line, chronological timeline ordering, the grounded vs
// anticipated split, status dot styling) lives here as PURE functions: same
// input → same output, no DOM, no React. That keeps the pages thin renderers and
// lets vitest exercise the logic without jsdom/RTL (CLAUDE.md §30), mirroring
// proofReviewHelpers.ts.
//
// NONE of this invents numbers — the metrics object on the payload is rendered
// verbatim; these helpers only format strings and order/partition arrays the
// payload already contains (Charter §8 honesty rule).
// =============================================================================

import type {
  ExchangeTurn,
  ScenarioStatus,
  ScenarioSummary,
} from "./trialPrepData";

/** The pattern-flag pill text + whether it renders muted. */
export interface PatternFlag {
  text: string;
  muted: boolean;
}

/**
 * Derive the scenario card's pattern-flag pill from `baseless_repeat_count`:
 * - `null`  → "pattern analysis pending" (muted) — the cross-document pass has
 *   not run yet, so absence of a flag is NOT the same as "no repeat".
 * - `0`     → "no baseless repeat yet" (muted) — analysed, nothing found.
 * - `> 0`   → "repeated N× after rebuttal" (emphasized) — the Count IV signal.
 *
 * Keeping null and 0 distinct is the honesty point (Standing Rule 1): "pending"
 * must never read as "clean".
 */
export function patternFlagText(baselessRepeatCount: number | null): PatternFlag {
  if (baselessRepeatCount === null) {
    return { text: "pattern analysis pending", muted: true };
  }
  if (baselessRepeatCount === 0) {
    return { text: "no baseless repeat yet", muted: true };
  }
  return { text: `repeated ${baselessRepeatCount}× after rebuttal`, muted: false };
}

/**
 * The card's "N instances · speakers · N responses" summary line. Counts come
 * straight from the payload; speakers are joined with a comma. Pure string
 * formatting — no derivation of the counts themselves.
 */
export function scenarioMetaLine(scenario: ScenarioSummary): string {
  const speakers = scenario.speakers.length
    ? scenario.speakers.join(", ")
    : "no speakers yet";
  return `${scenario.instance_count} instances · ${speakers} · ${scenario.response_count} responses`;
}

/**
 * Order an exchange timeline chronologically by `date` ascending. Turns with a
 * `null` date (anticipated/projected moves, which have no record date) sort
 * LAST, after all dated turns. Pure: returns a new array, never mutates input.
 *
 * ## React/TS Learning: a stable, non-mutating sort
 *
 * `Array.prototype.sort` mutates in place and is not guaranteed stable across
 * engines for equal keys, so we copy first (`[...turns]`) and give a total
 * comparator (date asc, nulls last) — the page can re-render without the source
 * array drifting.
 */
export function sortTimelineByDate(turns: ExchangeTurn[]): ExchangeTurn[] {
  return [...turns].sort((a, b) => {
    if (a.date === null && b.date === null) return 0;
    if (a.date === null) return 1; // nulls last
    if (b.date === null) return -1;
    return a.date.localeCompare(b.date);
  });
}

/**
 * Whether a turn is *anticipated* (projected, not from the record) rather than
 * grounded. The single source of truth is `grounded` — an anticipated turn has
 * no citation and must render with the "anticipated — not in record" marker and
 * NO source link (the hard grounded-vs-anticipated rule).
 */
export function isAnticipated(turn: ExchangeTurn): boolean {
  return !turn.grounded;
}

/**
 * Whether a turn should display the "repeated after rebuttal" flag: it must be
 * an `accusation_repeat` AND carry `repeated_after_rebuttal`. (A plain
 * accusation, or a repeat that does not postdate a proven rebuttal, gets no
 * flag.)
 */
export function showsRepeatFlag(turn: ExchangeTurn): boolean {
  return turn.kind === "accusation_repeat" && turn.repeated_after_rebuttal;
}

/** Status dot label + token color for a scenario. */
export interface StatusMeta {
  label: string;
  color: string;
}

/**
 * Map a scenario status to a human label and a design-token color for the dot.
 * Centralized so the dashboard card and the detail header agree.
 */
export function statusMeta(status: ScenarioStatus): StatusMeta {
  switch (status) {
    case "ready":
      return { label: "Ready", color: "var(--state-success-strong)" };
    case "review":
      return { label: "In review", color: "var(--accent-primary)" };
    case "drafted":
      return { label: "Drafted", color: "var(--text-muted)" };
    case "needs_response":
      return { label: "Needs response", color: "var(--state-warning-strong)" };
  }
}
