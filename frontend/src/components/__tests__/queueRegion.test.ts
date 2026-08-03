// =============================================================================
// queueRegion.test.ts — the collapsible ruling queue's rules (D10, ruling R7)
// =============================================================================

import { describe, expect, it } from "vitest";

import { keyboardShouldRule, nextUpHint, queueRegion } from "../queueRegion";

describe("the default open state", () => {
  it("is EXPANDED while anything is unruled", () => {
    // §2.3: the queue is the work in front of the human, and hiding 145 unruled
    // candidates behind a summary is how a pass never gets started.
    expect(queueRegion(3, 148).open).toBe(true);
  });

  it("is COLLAPSED once everything is ruled", () => {
    expect(queueRegion(148, 148).open).toBe(false);
  });

  it("is collapsed for a scenario with no candidates at all", () => {
    // Nothing to rule is nothing to show. Not an error (Standing Rule 1).
    expect(queueRegion(0, 0).open).toBe(false);
  });
});

describe("the summary line", () => {
  it("counts what is LEFT, not what has been done", () => {
    // The human's question at the summary line is "how much is in front of me".
    expect(queueRegion(3, 148).summary).toBe("Candidates awaiting ruling — 145");
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
    expect(queueRegion(3, 148).scope).toContain("all scans");
  });

  it("hands the chevron a label naming what collapsing costs", () => {
    // Item 4: collapse is chevron-only now, and collapsing PAUSES the keys. A
    // human who does not know that reads a dead keyboard as a broken one.
    const label = queueRegion(3, 148).chevronLabel;
    expect(label).toContain("only this arrow");
    expect(label).toContain("keys pause");
    // At zero there is nothing to warn about — the queue is a receipt.
    expect(queueRegion(10, 10).chevronLabel).toBe("Expand the queue");
  });

  it("becomes a receipt at zero rather than an empty container", () => {
    const region = queueRegion(10, 10);
    expect(region.summary).toBe("All candidates ruled");
    // No scope clause and no remaining count: there is nothing left to qualify.
    expect(region.scope).toBeNull();
    expect(region.remainingLabel).toBeNull();
  });
});

describe("the progress bar", () => {
  it("reports M of P and the matching percentage", () => {
    const region = queueRegion(3, 148);
    expect(region.progressLabel).toBe("3 of 148 ruled");
    expect(region.progressPercent).toBe(2);
  });

  it("is 0% rather than NaN for an empty pool", () => {
    // Dividing by a zero total unguarded is how a brand-new scenario gets a blank
    // bar and a console error.
    expect(queueRegion(0, 0).progressPercent).toBe(0);
  });

  it("clamps a count that exceeds its total instead of overfilling", () => {
    // A reload that returns a SHORTER pool can briefly put `ruled` above `total`
    // (the reducer's own "clamps focus when a reload returns a shorter pool" case).
    // A 140%-full bar is a rendering fault the human would read as a data fault.
    const region = queueRegion(20, 10);
    expect(region.progressPercent).toBe(100);
    expect(region.progressLabel).toBe("10 of 10 ruled");
    expect(region.open).toBe(false);
  });

  it("clamps a negative count", () => {
    expect(queueRegion(-5, 10).progressLabel).toBe("0 of 10 ruled");
  });
});

describe("the next-up hint (D10)", () => {
  it("names the card that is coming", () => {
    expect(nextUpHint("C-96")).toBe("Next up: C-96");
  });

  it("is ABSENT rather than empty when the next card has no ordinal yet", () => {
    // A candidate read in the instant it enters the pool has no ordinal. "Next up:
    // —" tells the human nothing and looks like a bug; no hint at all is honest.
    expect(nextUpHint(null)).toBeNull();
    expect(nextUpHint(undefined)).toBeNull();
    expect(nextUpHint("")).toBeNull();
  });
});

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
    expect(queueRegion(3, 148).keyboardActive).toBe(true);
    expect(queueRegion(148, 148).keyboardActive).toBe(false);
  });
});
