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
} from "./trialPrepData";

// `PatternFlag` / `patternFlagText` were RETIRED here (ruling R2 §3, built .396).
//
// They produced the "pattern analysis pending" chip, which rendered on every
// scenario card in every state because `baseless_repeat_count` is a field the
// backend hardcodes to null — the cross-document pass that would populate it is
// unwired. R2 ruled the chip dies, so the helper that fed it goes with it rather
// than staying as a tested function nothing renders. (That shape — built,
// covered, unreachable — is the `QuestionLine` defect, and it is not something to
// create deliberately.)
//
// The three-way honesty the helper encoded is worth keeping on the record for
// whoever wires the analysis: null "pending", 0 "analysed, nothing found" and
// >0 "repeated N times" are three different claims, and the first must never
// read as the second.

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
    case "draft":
      return { label: "Draft", color: "var(--text-muted)" };
    case "needs_evidence":
      return { label: "Needs evidence", color: "var(--state-warning-strong)" };
    case "ready":
      return { label: "Ready", color: "var(--state-success-strong)" };
  }
}
