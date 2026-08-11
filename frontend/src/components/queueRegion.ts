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

import { candidateState, isProposed } from "./candidateFilters";
import { fillCount } from "../services/evidenceLinks";
import type { ScenarioCard } from "../services/scenarioCards";

/** What the collapsible region shows, as data. */
export type QueueRegionDescriptor = {
  /** Whether the region starts open. Computed, never restored from storage. */
  open: boolean;
  /**
   * The head line's headline, or `null` when the FRAME's own heading speaks.
   *
   * `null` is the ordinary open-queue state since task R4 (P4): the heading is
   * then "Included — 21", composed from the active filter. A string here is
   * either the served proposal heading or a served zero-state sentence.
   */
  summary: string | null;
  /** The labelling-law clause that sits beside it, or `null` — at zero, and on a
   *  queue led by proposals, where the heading already names its own source. */
  scope: string | null;
  /** The chevron's accessible label + tooltip, naming what collapsing costs. */
  chevronLabel: string;
  /**
   * The counting sentence, shown ONLY before anything has measured the pool.
   *
   * ## The pool-wide bar died here (ONE_CARD_GRAMMAR, Piece 1c)
   *
   * It read "23 of 148 ruled" over a bar of 125 nobody owes. Rule-the-promising
   * is the ratified triage model — a curator works the proposals and leaves the
   * rest — so a bar measuring the whole gathered pool was reporting a debt the
   * method says does not exist. Progress now follows the ACTIVE FILTER and lives
   * under the filter chips, beside the list it describes.
   *
   * What survives is the one state the chips cannot show: nothing has been
   * measured yet, which is different from a pool of zero.
   */
  countingNotice: string | null;
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
  /** The heading when a completed scan is proposing candidates. Carries
   *  `{count}` and `{when}` (2026-08-08). */
  proposedHeading: string;
};

/**
 * What the projection is putting in front of the human, or `null`.
 *
 * `when` arrives already formatted, in the reader's locale — the same division of
 * labour the scan-history delete confirmation uses for its `{run}`: the server
 * owns the sentence, the browser owns the date format.
 */
export type QueueProposals = { count: number; when: string };

/** What the queue has measured, or `null` when it has not measured anything. */
export type QueueProgress = { ruled: number; total: number };

/**
 * How many cards the latest completed scan is proposing.
 *
 * `null` in, `0` out: an unread pool proposes nothing, and a heading claiming
 * otherwise before the read lands is the same false-before-measured defect
 * `QueueProgress | null` exists to prevent.
 *
 * ## Why the browser counts this and the payload also carries it
 *
 * They are the same number counted two ways over the same served payload, and
 * that is deliberate rather than duplicated: `proposal_source.proposed_count` is
 * what the SCAN CARD reports (it is a fact about the run), while this counts the
 * cards actually in hand for the queue's own heading. If the two ever disagree,
 * something dropped a card between the wire and the list — which is exactly the
 * class of defect §9 exists to make visible.
 */
export function proposedCount(cards: ScenarioCard[] | null): number {
  return cards === null ? 0 : cards.filter(isProposed).length;
}

/**
 * Whether ANY card in the pool carries a scan's score.
 *
 * The test behind the "from all scans" clause (task 2.15, piece 3b), and it
 * SURVIVES the projection deliberately. The clause describes a queue led by the
 * POOL — the state a scanned-and-fully-ruled scenario is in, where the rows do
 * have scan parentage but nothing is being proposed. Measured 2026-08-07: without
 * this condition a freshly created scenario led with "148 · from all scans" over a
 * pool no scan had ever touched, and that defect must not come back through the
 * door this task opened.
 *
 * `null` in, `false` out: an unread pool has nothing scored in it.
 */
export function anyScanScored(cards: ScenarioCard[] | null): boolean {
  return cards !== null && cards.some((c) => c.confidence.band !== "unscored");
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
  proposals: QueueProposals | null = null,
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
      countingNotice: wording?.counting ?? null,
      keyboardActive: false,
    };
  }

  // Clamp rather than trust: a reload that returns a shorter pool can briefly put
  // `ruled` above `total`, and a 140%-full progress bar is a rendering fault the
  // human would read as a data fault.
  const safeTotal = Math.max(0, progress.total);
  const safeRuled = Math.min(Math.max(0, progress.ruled), safeTotal);
  const unruled = safeTotal - safeRuled;

  // The queue LEADS with proposals when there are any (2026-08-08): they are what
  // the human came to rule, and before this they arrived buried in a pool of 148
  // with nothing marking them. The heading names the scan, because "30 awaiting
  // ruling" and "30 the Aug 7 scan put in front of you" are different claims and
  // only the second is true of a projection.
  const proposedHeading =
    proposals && proposals.count > 0 && wording
      ? fillCount(wording.proposedHeading, proposals.count).replace(
          "{when}",
          proposals.when,
        )
      : null;

  return {
    open: unruled > 0,
    // `null` means "the FRAME's own heading speaks here" — the active filter and
    // its count, composed by `ScanSection` from what the queue reports.
    //
    // ## What died on this line (task R4, P4)
    //
    // The old heading interpolated the unruled count into a hardcoded English
    // sentence — in a codebase whose standing law is that every visible word is
    // a stored row. It named the whole unruled pool while the
    // list underneath showed one filter's slice, so the heading and the rows
    // disagreed by construction — 145 in the heading over 21 rows on screen.
    //
    // The zero-state sentences below stay: they are served rows, and they say
    // something the filter heading cannot ("nothing here" and "the work is
    // done" are facts about the pool, not about the view).
    summary:
      proposedHeading ??
      (unruled > 0
        ? null
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
          : (wording?.allRuled ?? "")),
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
    // TWO conditions now, and both are load-bearing. It is absent when the
    // proposal heading is showing — that sentence already names its source, and a
    // second clause underneath would be the page saying "from all scans" beside
    // "proposed by the Aug 7 scan": two answers to one question. And it is still
    // EARNED by the data (task 2.15, piece 3b), because a pool nothing has scanned
    // must not claim a scan parentage.
    scope: proposedHeading === null && unruled > 0 && scanScored ? "from all scans" : null,
    chevronLabel:
      unruled > 0
        ? "Collapse the queue — only this arrow collapses it; keys pause while collapsed"
        : "Expand the queue",
    // Measured: the chips below carry the counts, and the filter's own progress
    // line carries how much of it is done.
    countingNotice: null,
    keyboardActive: unruled > 0,
  };
}

// REMOVED (task R2, Roman's cleanup ruling): `nextUpHint`.
//
// It rendered "Next up: C-14" beside a list whose next row WAS C-14, one line
// below. A hint that names what the reader is already looking at is a line they
// learn to skip, and this frame had five of them.

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
