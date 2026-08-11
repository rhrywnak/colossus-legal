// =============================================================================
// queueRegion.test.ts — the collapsible ruling queue's rules (D10, ruling R7)
// =============================================================================

import { describe, expect, it } from "vitest";

/** The two stored zero-state lines, as the backend serves them. */
const WORDS = {
  emptyPool: "No candidates gathered yet",
  allRuled: "All candidates ruled",
  counting: "Counting candidates…",
  proposedHeading: "Candidates awaiting ruling — {count} proposed by the {when} scan",
};

import {
  anyScanScored,
  keyboardShouldRule,
  proposedCount,
  queueRegion,
  progressFromCards,
} from "../queueRegion";
import { candidateCounts } from "../candidateFilters";
import type { ScenarioCard } from "../../services/scenarioCards";

describe("the default open state", () => {
  it("is EXPANDED while anything is unruled", () => {
    // §2.3: the queue is the work in front of the human, and hiding 145 unruled
    // candidates behind a summary is how a pass never gets started.
    expect(queueRegion({ ruled: 3, total: 148 }).open).toBe(true);
  });

  it("is COLLAPSED once everything is ruled", () => {
    expect(queueRegion({ ruled: 148, total: 148 }).open).toBe(false);
  });

  it("is collapsed for a scenario with no candidates at all", () => {
    // Nothing to rule is nothing to show. Not an error (Standing Rule 1).
    expect(queueRegion({ ruled: 0, total: 0 }).open).toBe(false);
  });
});

describe("the summary line", () => {
  it("YIELDS the open-queue heading to the frame (task R4, P4)", () => {
    // WAS: "counts what is LEFT, not what has been done" — the summary read
    // `Candidates awaiting ruling — 145`.
    //
    // Two things were wrong with it and P4 killed both. It was a hardcoded
    // English sentence on a surface whose standing law is that every visible
    // word is a stored row; and 145 counted the whole unruled pool while the
    // list beneath it showed one filter's 21, so the heading and its own rows
    // disagreed by construction.
    //
    // `null` is this module saying "the frame's heading speaks here" —
    // `ScanSection` composes the active filter's stored name and count. What is
    // pinned is that no sentence is invented HERE.
    expect(queueRegion({ ruled: 3, total: 148 }).summary).toBeNull();
  });

  it("says the pile spans EVERY scan, not just the last one", () => {
    // §2.3's labelling law: scans only ADD, rulings only DRAIN, rerunning never
    // removes. A human who believes the queue is "the last scan's results" will
    // rerun a scan expecting a reset, and will be wrong.
    //
    // Task 1.7D shortened this clause to the mockup's "from all scans" and moved
    // the add/drain sentence up to the section subtitle, where it has room to be a
    // sentence. The head line keeps the load-bearing half — that the count spans
    // every scan — and the structural test below pins the other half's new home so
    // the law cannot go missing between the two.
    // Task 2.15 (piece 3b) made the clause EARNED rather than always printed, and
    // 2026-08-08 changed what earns it: the clause belongs to a queue led by the
    // POOL. When the projection is leading instead, the heading names its own scan
    // and this clause would be a second, wronger answer to the same question.
    expect(queueRegion({ ruled: 3, total: 148 }, null, null, true).scope).toContain("all scans");
  });

  it("still refuses the clause on a pool no scan has scored", () => {
    // The 2026-08-07 defect, guarded through the change: the clause is EARNED by
    // the data, and a projection-led heading does not change that for the pool-led
    // case underneath it.
    const scored = { confidence: { band: "high" } } as unknown as ScenarioCard;
    const unscored = { confidence: { band: "unscored" } } as unknown as ScenarioCard;
    expect(anyScanScored(null)).toBe(false);
    expect(anyScanScored([unscored])).toBe(false);
    expect(anyScanScored([unscored, scored])).toBe(true);
  });

  it("leads with the PROPOSALS when a scan is proposing anything", () => {
    // The queue's whole reason for existing on a scanned scenario. The heading is
    // the stored template, filled with the live count and the run's date — and it
    // names the scan, because "30 awaiting ruling" and "30 the Aug 7 scan put in
    // front of you" are different claims and only the second is true here.
    const led = queueRegion({ ruled: 0, total: 148 }, WORDS, { count: 30, when: "Aug 7" });
    expect(led.summary).toBe("Candidates awaiting ruling — 30 proposed by the Aug 7 scan");
    // And the pool's own labelling clause stands down: one answer, not two.
    expect(led.scope).toBeNull();
  });

  it("falls back to the pool heading when the projection proposes nothing", () => {
    // A completed run whose every pick has been ruled is a FINISHED queue, not a
    // proposing one — the payload withholds the source entirely in that case, and
    // a count of zero must never render "0 proposed by the …".
    const none = queueRegion({ ruled: 3, total: 148 }, WORDS, { count: 0, when: "Aug 7" });
    // Falls through to the frame's own heading (task R4, P4) rather than to the
    // retired literal. The property under test is unchanged: a count of zero
    // must never render "0 proposed by the …".
    expect(none.summary).toBeNull();
  });

  it("counts the proposals off the CARDS, and says none before the pool is read", () => {
    // What feeds the heading. Before the page's own read lands there is nothing
    // measured at all, and an unread pool must not claim a scan proposed anything
    // (the same third-state rule `QueueProgress | null` exists for).
    const plain = {} as unknown as ScenarioCard;
    const proposed = {
      proposed: { role_label: null, reason: null, duplicate_count: 1, duplicate_label: null },
    } as unknown as ScenarioCard;

    expect(proposedCount(null)).toBe(0);
    expect(proposedCount([])).toBe(0);
    expect(proposedCount([plain, plain])).toBe(0);
    expect(proposedCount([plain, proposed])).toBe(1);
  });

  it("never claims a scan parentage the pool does not have (task 2.15)", () => {
    // THE measured defect of 2026-08-07: a freshly created scenario led with
    // "Candidates awaiting ruling — 148 · from all scans" while the same screen
    // reported all 148 never scanned. The count was right; the clause was a claim
    // about where the rows came from, and no scan had ever looked at them.
    const unscanned = queueRegion({ ruled: 0, total: 148 }, null, null, false);
    expect(unscanned.scope).toBeNull();
    // The clause is what this test is about, and it is still withdrawn. The
    // count moved to the frame's heading in task R4 (P4), so there is no longer
    // a number on this line to assert — `scope` is the whole claim now.
    expect(unscanned.summary).toBeNull();
  });

  it("tells an empty pool apart from a finished one (task 2.13)", () => {
    // THE beta.376 defect, as a test. Before the queue reports its counts both
    // are zero, and the summary read "All candidates ruled" — on a scenario with
    // 92 candidates still unruled. Arithmetically true, substantively false: the
    // header announced the work was done before it had looked.
    expect(queueRegion({ ruled: 0, total: 0 }, WORDS).summary).toBe("No candidates gathered yet");
    // A REAL pool with nothing outstanding still says the work is done.
    expect(queueRegion({ ruled: 10, total: 10 }, WORDS).summary).toBe("All candidates ruled");
    // And the two are never the same string, which is the whole point.
    expect(queueRegion({ ruled: 0, total: 0 }, WORDS).summary).not.toBe(
      queueRegion({ ruled: 10, total: 10 }, WORDS).summary,
    );
  });

  it("says nothing rather than inventing words before the wording loads", () => {
    // R4: there is no literal to fall back to. An empty summary is visibly
    // incomplete for the instant it lasts; a compiled-in default would be a
    // string the Settings page cannot edit and nobody knows is there.
    expect(queueRegion({ ruled: 0, total: 0 }).summary).toBe("");
    expect(queueRegion({ ruled: 10, total: 10 }).summary).toBe("");
    // And the open queue now invents nothing at all rather than falling back to
    // a compiled-in sentence (task R4, P4) — which is the same rule, applied to
    // the one line that used to be exempt from it.
    expect(queueRegion({ ruled: 3, total: 148 }).summary).toBeNull();
  });

  it("hands the chevron a label naming what collapsing costs", () => {
    // Item 4: collapse is chevron-only now, and collapsing PAUSES the keys. A
    // human who does not know that reads a dead keyboard as a broken one.
    const label = queueRegion({ ruled: 3, total: 148 }).chevronLabel;
    expect(label).toContain("only this arrow");
    expect(label).toContain("keys pause");
    // At zero there is nothing to warn about — the queue is a receipt.
    expect(queueRegion({ ruled: 10, total: 10 }).chevronLabel).toBe("Expand the queue");
  });

  it("becomes a receipt at zero rather than an empty container", () => {
    const region = queueRegion({ ruled: 10, total: 10 }, WORDS);
    expect(region.summary).toBe("All candidates ruled");
    // No scope clause and no counting notice: there is nothing left to qualify,
    // and the pool has plainly been measured.
    expect(region.scope).toBeNull();
    expect(region.countingNotice).toBeNull();
  });
});

// The "the progress bar" suite that stood here died with the bar it described
// (ONE_CARD_GRAMMAR, Piece 1c). It pinned "3 of 148 ruled" and a percentage over
// the whole gathered pool — a debt rule-the-promising says nobody owes. Progress
// now follows the ACTIVE FILTER and is tested in `candidateFilters.test.ts`
// under "progress counts follow the active filter", where its denominator lives.
//
// The clamping cases went with it and were not lost: they guarded a PERCENTAGE
// against a reload returning a shorter pool, and there is no percentage now.

// The next-up hint's tests went with the hint (task R2, Roman's cleanup ruling).
// It rendered "Next up: C-14" beside a list whose next row was C-14 — a line
// naming what the reader was already looking at. Nothing replaced it; the row
// below the selected one is the hint.

describe("the collapsed-queue keyboard guard (ruling R7)", () => {
  it("keys rule while the region is open", () => {
    expect(keyboardShouldRule(true)).toBe(true);
  });

  it("keys are INERT while the region is collapsed", () => {
    // A `<details>` body stays in the DOM when closed, so without this the one-key
    // rulings would keep firing on a card nobody can see — and the human would have
    // no way to tell that from a stray keypress.
    //
    // Roman declined relying on the zero-unruled coincidence: it holds today only
    // because the region auto-collapses exactly when the pool empties, and anything
    // that later lets a human collapse a non-empty queue would silently start
    // ruling invisible cards.
    expect(keyboardShouldRule(false)).toBe(false);
  });

  it("reports keyboardActive in step with the open state", () => {
    expect(queueRegion({ ruled: 3, total: 148 }).keyboardActive).toBe(true);
    expect(queueRegion({ ruled: 148, total: 148 }).keyboardActive).toBe(false);
  });
});

// ── The third state: counts not known (the beta.376/.377 defect) ────────────

describe("counts that have not been measured", () => {
  it("says it is counting rather than reporting a result", () => {
    // THE regression guard. Shipped twice on DEV: "All candidates ruled" over 92
    // unruled candidates (.376), then "No candidates gathered yet" over 148
    // gathered ones (.377). Both were confident sentences about a number nothing
    // had read.
    const region = queueRegion(null, WORDS);
    expect(region.summary).toBe("Counting candidates…");
    expect(region.summary).not.toBe(WORDS.emptyPool);
    expect(region.summary).not.toBe(WORDS.allRuled);
  });

  it("claims nothing else about a pool nobody has looked at", () => {
    // A scope clause would be a second claim about the same unmeasured pool.
    const region = queueRegion(null, WORDS);
    expect(region.scope).toBeNull();
    // The counting notice is the ONE thing this state says, and it is the stored
    // sentence — the line that used to hold the only hardcoded copy of it.
    expect(region.countingNotice).toBe("Counting candidates…");
  });

  it("starts open, with the keyboard inert", () => {
    // Open, because a queue that hides itself before it has counted shows the
    // human a page with no work on it. Keys inert, because there is nothing
    // measured to rule on yet.
    const region = queueRegion(null, WORDS);
    expect(region.open).toBe(true);
    expect(region.keyboardActive).toBe(false);
  });

  it("keeps all three states distinct", () => {
    // The property the whole fix rests on: unknown, empty and finished are three
    // different sentences. Any two collapsing is the defect returning.
    const summaries = [
      queueRegion(null, WORDS).summary,
      queueRegion({ ruled: 0, total: 0 }, WORDS).summary,
      queueRegion({ ruled: 10, total: 10 }, WORDS).summary,
    ];
    expect(new Set(summaries).size).toBe(3);
    expect(summaries).toEqual([
      "Counting candidates…",
      "No candidates gathered yet",
      "All candidates ruled",
    ]);
  });

  it("says nothing rather than inventing words before the wording loads", () => {
    // R4 again: no literal to fall back to, on the new state as on the others.
    expect(queueRegion(null).summary).toBe("");
    expect(queueRegion(null).countingNotice).toBeNull();
  });
});

// ── Task 2.13c: counts derived from the page's own pool ─────────────────────

describe("progressFromCards", () => {
  /**
   * Only the two fields `candidateState` reads. Cast rather than built in full:
   * a complete `ScenarioCard` is twenty fields of quote, pinpoint and speaker,
   * none of which this counting touches, and spelling them out would bury the
   * one thing each case is about.
   */
  const card = (status: string, deferReason: string | null = null) =>
    ({
      status,
      defer_reason: deferReason,
      // `candidateCounts` also tallies facets this counting does not use:
      // `isScanScored` reads `confidence.band` and `isRulableNow` reads
      // `defer_required`. Both are supplied so the comparison runs against the
      // REAL counting function with every field it touches — omitting
      // `defer_required` left `counts.rulable` quietly wrong (`!undefined` is
      // true), which nothing asserted on today but the next test to reuse this
      // fixture would have inherited.
      defer_required: false,
      confidence: { band: "unscored", label: "unscored" },
    }) as unknown as ScenarioCard;

  it("keeps an unread pool distinguishable from an empty one", () => {
    // THE latch, at its root. `null` means the page has not read yet; `[]` means
    // it read and there is nothing there. Collapsing them is what produced
    // "No candidates gathered yet" over 148 gathered candidates, twice.
    expect(progressFromCards(null)).toBeNull();
    expect(progressFromCards([])).toEqual({ ruled: 0, total: 0 });
  });

  it("counts an included or excluded card as ruled", () => {
    const cards = [
      card("included"),
      card("dropped"),
      card("undecided"),
      card("undecided"),
    ];
    expect(progressFromCards(cards)).toEqual({ ruled: 2, total: 4 });
  });

  it("counts a DEFERRED card as ruled, not as outstanding", () => {
    // The .379 acceptance FAIL. A deferred card is `undecided` carrying a defer
    // reason: a human looked at it and parked it with a stated reason, which is
    // the whole distinction defer exists to preserve. Counting it as remaining
    // told Roman he had ten more candidates to work than he actually did.
    const cards = [card("undecided"), card("undecided", "page missing from scan")];
    expect(progressFromCards(cards)).toEqual({ ruled: 1, total: 2 });
  });

  it("never disagrees with the filter groups drawn from the same pool", () => {
    // THE contradiction, as a test. On DEV the header read "Candidates awaiting
    // ruling — 102" while the status filter immediately below it read "Not ruled
    // (92)" and "Deferred (10)". One screen, one pool, two answers.
    //
    // Both numbers now come from `candidateState`, so this asserts they agree by
    // construction — and it is written against `candidateCounts`, the function
    // the groups actually use, so a change to either side breaks it.
    const pool = [
      ...Array.from({ length: 46 }, () => card("included")),
      ...Array.from({ length: 10 }, () => card("undecided", "waiting on the page")),
      ...Array.from({ length: 92 }, () => card("undecided")),
    ];

    const progress = progressFromCards(pool)!;
    const counts = candidateCounts(pool);

    // THE GUARD. These two are what fail if the predicate drifts: against the old
    // `status !== "undecided"` they read {ruled: 46} and 92, which is the exact
    // "expected 102 to be 92" seen on DEV.
    expect(progress).toEqual({ ruled: 56, total: 148 });
    expect(counts.not_ruled).toBe(92);

    // Meaningful: the four states must still PARTITION the pool. A fifth state
    // left unhandled by `candidateCounts`'s switch is caught here.
    expect(
      counts.not_ruled + counts.deferred + counts.included + counts.excluded,
    ).toBe(counts.all);

    // DOCUMENTATION, not a guard — and labelled so nobody cites it as one. Both
    // sides walk the same array with the same function, so `total - ruled` is
    // the complement of `not_ruled` whatever `candidateState` returns: this
    // assertion is tautological and cannot fail. It is kept because it states
    // the invariant the fix exists to create, in the form a reader will look
    // for. The concrete numbers above are what actually catch a regression.
    expect(progress.total - progress.ruled).toBe(counts.not_ruled);
  });

  it("feeds a summary that is right whether the region is open or closed", () => {
    // The property the whole restructure exists for: these counts do not depend
    // on any component being mounted, so the collapsed header is as well-informed
    // as the expanded one.
    const cards = [
      ...Array.from({ length: 56 }, () => card("included")),
      ...Array.from({ length: 92 }, () => card("undecided")),
    ];
    const progress = progressFromCards(cards);
    expect(progress).toEqual({ ruled: 56, total: 148 });

    const region = queueRegion(progress, WORDS);
    // Task R4 (P4): an open queue yields its heading to the frame. The property
    // this test exists for is untouched and is asserted above — the COUNTS do
    // not depend on any component being mounted, so a collapsed header is as
    // well-informed as an expanded one. What is pinned here is that a real pool
    // with work outstanding is never mistaken for an empty one.
    expect(region.summary).toBeNull();
    expect(region.summary).not.toBe(WORDS.emptyPool);
    expect(region.countingNotice).toBeNull();
  });
});
