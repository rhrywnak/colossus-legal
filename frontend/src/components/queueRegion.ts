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

import { candidateState, isScanScored } from "./candidateFilters";
import type { ScenarioCard } from "../services/scenarioCards";

/** What the collapsible region shows, as data. */
export type QueueRegionDescriptor = {
  /** Whether the region starts open. Computed, never restored from storage. */
  open: boolean;
  /** The head line's headline, e.g. `"Candidates awaiting ruling — 145"`. */
  summary: string;
  /** The labelling-law clause that sits beside it, or `null` at zero. */
  scope: string | null;
  /** The chevron's accessible label + tooltip, naming what collapsing costs. */
  chevronLabel: string;
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
/**
 * The two summary lines this region can show at zero outstanding.
 *
 * Stored strings, passed in rather than imported: this module is pure and the
 * words are the settings store's (the configuration law's text half). `null`
 * until the wording loads — see the fallback in `queueRegion`.
 */
export type QueueSummaryWording = {
  /** Shown when there is no pool at all — measured, and empty. */
  emptyPool: string;
  /** Shown when a real pool exists and none of it is outstanding. */
  allRuled: string;
  /** Shown while the counts are NOT KNOWN — nothing has measured the pool yet. */
  counting: string;
};

/** What the queue has measured, or `null` when it has not measured anything. */
export type QueueProgress = { ruled: number; total: number };

/**
 * Whether ANY card in the pool carries a scan's score.
 *
 * The test behind the labelling clause (piece 3b). `null` in, `false` out: an
 * unread pool has nothing scored in it, and a clause claiming otherwise before
 * the read lands is the same false-before-measured defect `QueueProgress | null`
 * exists to prevent.
 */
export function anyScanScored(cards: ScenarioCard[] | null): boolean {
  return cards !== null && cards.some(isScanScored);
}

/**
 * ## Why this takes `QueueProgress | null` and not two numbers
 *
 * It used to take `(ruled, total)`, and every caller passed
 * `progress?.ruled ?? 0`. That made "nobody has counted yet" and "the pool is
 * empty" the SAME argument — `(0, 0)` — so no condition inside could tell them
 * apart, and the summary confidently described a number nothing had read.
 *
 * On DEV that shipped twice: "All candidates ruled" over 92 unruled candidates
 * (beta.376), then "No candidates gathered yet" over 148 gathered ones
 * (beta.377) after task 2.13 split the wrong seam. The bug was never in the
 * branch — it was in a signature that could not represent the third state.
 *
 * `null` now means unknown, and it is unrepresentable as a count. The clamp
 * below still guards the measured case.
 */
export function queueRegion(
  progress: QueueProgress | null,
  wording: QueueSummaryWording | null = null,
  scanScored = false,
): QueueRegionDescriptor {
  // Counts unknown: say so and nothing else. No progress figure, no remaining
  // count, no scope clause — every one of those would be a claim about a pool
  // nothing has looked at (Standing Rule 1). Mirrors the progress label's own
  // long-standing "Counting candidates…" treatment eleven lines up in
  // `ScanSection`, which this now serves from the store as well.
  if (progress === null) {
    return {
      open: true,
      summary: wording?.counting ?? "",
      scope: null,
      chevronLabel: "Expand the queue",
      progressLabel: wording?.counting ?? "",
      progressPercent: 0,
      remainingLabel: null,
      keyboardActive: false,
    };
  }

  // Clamp rather than trust: a reload that returns a shorter pool can briefly put
  // `ruled` above `total`, and a 140%-full progress bar is a rendering fault the
  // human would read as a data fault.
  const safeTotal = Math.max(0, progress.total);
  const safeRuled = Math.min(Math.max(0, progress.ruled), safeTotal);
  const unruled = safeTotal - safeRuled;

  return {
    open: unruled > 0,
    summary:
      unruled > 0
        ? `Candidates awaiting ruling — ${unruled}`
        : // At zero the region is a receipt, not a queue — but WHICH receipt
          // depends on whether there is a pool at all.
          //
          // From the beta.376 click-through (2026-08-05): this line read "All
          // candidates ruled" on a scenario with 92 candidates still unruled. It
          // was never a counting bug. The counts are reported upward by the queue
          // once it has fetched, and until then BOTH are zero — so "nothing is
          // outstanding" was arithmetically true and substantively false. The
          // header announced the work was finished before it had looked.
          //
          // One condition separates the two states: a pool of zero is "there is
          // nothing here (or nothing yet)", a pool with everything ruled is "the
          // work is done". They are different facts and now say different things.
          safeTotal === 0
          ? (wording?.emptyPool ?? "")
          : (wording?.allRuled ?? ""),
    // v3 shortens this to the mockup's "from all scans". The labelling law it
    // carries is unchanged — the queue is EVERY unruled candidate across every
    // scan — and the section subtitle above still spells out that scans only add
    // and rulings only drain.
    //
    // ## The clause is EARNED, not decorative (task 2.15, piece 3b)
    //
    // Measured 2026-08-07: a scenario nothing had ever scanned led with "148 ·
    // from all scans" — a claim of scan parentage over a pool no scan had touched.
    // The clause now appears only when something in the pool actually carries a
    // scan's score, so it describes the rows it sits above rather than the label
    // somebody expected them to have.
    scope: unruled > 0 && scanScored ? "from all scans" : null,
    chevronLabel:
      unruled > 0
        ? "Collapse the queue — only this arrow collapses it; keys pause while collapsed"
        : "Expand the queue",
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

/**
 * The queue's counts, derived from the page's OWN pool (task 2.13c).
 *
 * ## Why `CardQueue` no longer reports them
 *
 * The summary was wrong on every fresh load — "No candidates gathered yet" three
 * lines under "148 candidates gathered" — and it took three attempts to fix
 * because the fault was structural. `CardQueue` reported the counts from an
 * effect on its FIRST render, before its own fetch resolved, so the section
 * received `{0, 0}`; that computed a collapsed region, which unmounted
 * `CardQueue`, so the real counts never arrived. A latch. The page already holds
 * the pool, so the counts come from there and are correct whether the region is
 * open or closed.
 *
 * ## Why "ruled" is `candidateState`, and not a predicate of its own
 *
 * The first version of this asked `status !== "undecided"`, which looks right and
 * is not. A DEFERRED card is `undecided` carrying a defer reason — a human looked
 * at it and parked it with a stated reason, which is the entire distinction defer
 * exists to preserve. Counting it as outstanding put "102 remaining" in the
 * header while the filter groups immediately below it read "Not ruled (92)" and
 * "Deferred (10)": one screen, one pool, two answers.
 *
 * The repair is not a better predicate here — it is having no predicate here at
 * all. `candidateState` is what the filter groups already count by, so deriving
 * from it makes the two numbers the same number by construction rather than by
 * agreement, and a future fifth state cannot make them disagree again.
 *
 * `null` in, `null` out: before the page's own fetch lands there is genuinely
 * nothing measured, and that is the one state that must stay distinguishable
 * from an empty pool.
 */
export function progressFromCards(cards: ScenarioCard[] | null): QueueProgress | null {
  if (cards === null) return null;
  return {
    ruled: cards.filter((card) => candidateState(card) !== "not_ruled").length,
    total: cards.length,
  };
}
