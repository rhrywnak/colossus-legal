// =============================================================================
// queueRegion.ts — the ruling queue as a collapsible region (defect D10, §2.3)
// =============================================================================
//
// Roman's grouping ruling 2026-08-03: candidates are scan OUTPUT, so the queue
// lives inside the Scan & candidates section as a collapsible region rather than
// as a standalone list. This module is that region's pure rules; `ScanSection`
// renders them and `CardQueue`'s §7 behaviour is untouched underneath.
//
// ## The labelling law (§2.3), and why the copy has to say it out loud
//
// The queue is EVERY unruled candidate across EVERY scan. Scans only add, rulings
// only drain, and rerunning a scan never removes anything. A human who thinks the
// queue is "the last scan's results" will rerun a scan expecting the pile to
// reset, and it will not — so the summary line says what the pile is.
//
// ## Default state is computed, never remembered
//
// Expanded while anything is unruled, collapsed to a quiet one-liner at zero.
// Deliberately NOT a stored preference: Rule 1's cosmetic-storage carve-out would
// technically permit remembering "collapsed", and Roman declined it (ruling R7) for
// the right reason — a queue that remembers "collapsed" over 145 unruled
// candidates is a silent failure wearing a preference's clothes.

/** What the collapsible region shows, as data. */
export type QueueRegionDescriptor = {
  /** Whether the region starts open. Computed, never restored from storage. */
  open: boolean;
  /** The summary line's headline, e.g. `"Candidates awaiting ruling — 145"`. */
  summary: string;
  /** The labelling-law clause that sits beside it, or `null` at zero. */
  scope: string | null;
  /** `"3 of 148 ruled"`. */
  progressLabel: string;
  /** The progress bar's fill, 0–100, already clamped. */
  progressPercent: number;
  /** `"145 remaining"`, or `null` when nothing remains. */
  remainingLabel: string | null;
  /** Whether the keyboard is live. False while collapsed (ruling R7). */
  keyboardActive: boolean;
};

/**
 * The region's state for a given progress position.
 *
 * @param ruled Cards this human has ruled on (`progress(state).ruled`).
 * @param total Cards in the pool (`progress(state).total`).
 */
export function queueRegion(ruled: number, total: number): QueueRegionDescriptor {
  // Clamp rather than trust: a reload that returns a shorter pool can briefly put
  // `ruled` above `total`, and a 140%-full progress bar is a rendering fault the
  // human would read as a data fault.
  const safeTotal = Math.max(0, total);
  const safeRuled = Math.min(Math.max(0, ruled), safeTotal);
  const unruled = safeTotal - safeRuled;

  return {
    open: unruled > 0,
    summary:
      unruled > 0
        ? `Candidates awaiting ruling — ${unruled}`
        : // At zero the region is a receipt, not a queue. "0 awaiting ruling"
          // technically says the same thing and reads like an empty container;
          // this says the work is done.
          "All candidates ruled",
    scope: unruled > 0 ? "from all scans — scans add, your rulings drain" : null,
    progressLabel: `${safeRuled} of ${safeTotal} ruled`,
    // A pool of zero is 0%, not NaN — dividing by `safeTotal` unguarded is how a
    // brand-new scenario gets a blank bar and a console error.
    progressPercent: safeTotal === 0 ? 0 : Math.round((safeRuled / safeTotal) * 100),
    remainingLabel: unruled > 0 ? `${unruled} remaining` : null,
    keyboardActive: unruled > 0,
  };
}

/**
 * The "Next up" hint, or `null` when there is nothing after the focused card.
 *
 * Takes the next card's CODE rather than the card, because that is all the hint
 * says and passing the whole card would invite this function to start composing
 * more of it.
 *
 * A next card with no ordinal yet yields `null` rather than `"Next up: —"`: the
 * hint exists to tell the human which C-number is coming, and a hint that names
 * nothing is worse than no hint.
 */
export function nextUpHint(nextCode: string | null | undefined): string | null {
  return nextCode ? `Next up: ${nextCode}` : null;
}

/**
 * Whether a keystroke should be allowed to rule.
 *
 * Ruling R7: keys are INERT while the region is collapsed, and the guard lives
 * here (and in `CardQueue`) rather than in the reducer — `cardTriage`'s 31 tests
 * stay byte-identical, and the reducer stays a pure state machine that knows
 * nothing about chrome.
 *
 * Roman's reason for not relying on the zero-unruled coincidence: it is true today
 * only because the region auto-collapses exactly when the pool empties. Anything
 * that later lets a human collapse a non-empty queue by hand would silently start
 * ruling invisible cards, and the bug would be indistinguishable from a stray
 * keypress.
 */
export function keyboardShouldRule(regionOpen: boolean): boolean {
  return regionOpen;
}
