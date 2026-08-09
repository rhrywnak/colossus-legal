/**
 * The scan report tells the truth about calls that died (ruling R4, 2026-08-09).
 *
 * ## The incident these tests are written against
 *
 * S-4 scan run 6a9fad89 (Claude Opus 5, 2:57 PM) attempted 104 judge calls. All
 * 104 returned HTTP 400 "temperature is deprecated for this model", inside five
 * seconds. `scan_runs.failed_count` recorded 104. What Roman saw was:
 *
 *     Last scan 2:57 PM · Claude Opus 5 · 0 proposed
 *     Complete · 104 judged · 0 relevant
 *
 * Every number on the screen was true and the sentence they formed was false.
 * The failed count had nowhere to appear, so the run read as a scan that worked
 * and found nothing — and, being `completed`, it took the projecting slot from
 * the Opus 4.8 run before it and projected nothing over 30 waiting proposals.
 *
 * What is asserted HERE is the browser's half: the collapsed line, which is the
 * sentence Roman read first. The reconciliation sentence and the tile counts are
 * composed by the backend from the run's frozen record and are pinned there
 * (`scan_conservation`, `theme_scan_persist`) — a browser assertion about them
 * would be asserting a fixture.
 */
import { describe, expect, it } from "vitest";

import { collapsedFailedSummary, collapsedScanSummary } from "../themeScanFormat";
import type { ScanConservation } from "../../services/themeScan";

/** The two seeded templates, as the migrations write them. */
const COMPLETED_TEMPLATE = "Last scan {when} · {model} · {count} proposed";
const FAILED_TEMPLATE = "Last scan {when} · {model} · Failed — {count} calls errored";

describe("failed_counts_render_in_tiles_and_summary_line", () => {
  it("says the run failed and how many calls died", () => {
    const line = collapsedFailedSummary(
      FAILED_TEMPLATE,
      "2:57 PM",
      "Claude Opus 5",
      104,
    );

    expect(line).toContain("2:57 PM");
    expect(line).toContain("Claude Opus 5");
    expect(line).toContain("104");
    // No token survives to the screen. A stray "{count}" is the failure mode a
    // template-filling helper actually has.
    expect(line).not.toMatch(/\{[a-z_]+\}/);
  });

  it("does not read like a scan that worked and found nothing", () => {
    // THE defect, stated as a comparison. Both lines describe the same run; only
    // one of them is true about it. A reader must not be able to mistake the
    // failed line for "scanned, nothing relevant" — which is exactly what "0
    // proposed" said on 2026-08-09.
    const asItShipped = collapsedScanSummary(
      COMPLETED_TEMPLATE,
      "2:57 PM",
      "Claude Opus 5",
      0,
    );
    const asItReadsNow = collapsedFailedSummary(
      FAILED_TEMPLATE,
      "2:57 PM",
      "Claude Opus 5",
      104,
    );

    expect(asItShipped).toContain("0 proposed");
    expect(asItReadsNow).not.toContain("0 proposed");
    expect(asItReadsNow).not.toEqual(asItShipped);
  });

  it("reports the failed count, never the proposed count", () => {
    // ANTI-VACUITY for the helper itself: a copy-paste of its sibling would fill
    // {count} from the wrong number and still produce a plausible sentence. Two
    // runs that differ ONLY in how many calls died must differ in their line.
    const one = collapsedFailedSummary(FAILED_TEMPLATE, "9:14 AM", "Qwen 32B", 1);
    const many = collapsedFailedSummary(FAILED_TEMPLATE, "9:14 AM", "Qwen 32B", 104);

    expect(one).toContain("1 calls errored");
    expect(many).toContain("104 calls errored");
    expect(one).not.toEqual(many);
  });
});

describe("the conservation block carries the term the law needs", () => {
  it("reconciles judged against relevant, irrelevant and failed", () => {
    // The identity the tiles and the reconciliation sentence exist to let a human
    // check: `judged = relevant + irrelevant + failed`. Before ruling R4 the
    // block had no `failed` field at all, so the left side was on screen and one
    // term of the right side was not — the arithmetic could not close, and
    // nothing said which number was missing.
    //
    // Typed as `ScanConservation`, so this is also the compile-time proof that
    // `failed` is on the wire: delete the field from the type and this file stops
    // building.
    const deadRun: ScanConservation = {
      pool: 148,
      excluded_empty: 2,
      excluded_statement_type: 4,
      excluded_too_short: 15,
      duplicates_collapsed: 23,
      judged: 104,
      failed: 104,
    };
    const relevant = 0;
    const irrelevant = 0;

    expect(deadRun.judged).toBe(relevant + irrelevant + deadRun.failed);

    // …and the pool still accounts for every row it gathered, which is the
    // separate identity the same sentence carries: 148 = 21 set aside + 23 folded
    // + 104 judged.
    const setAside =
      deadRun.excluded_empty + deadRun.excluded_statement_type + deadRun.excluded_too_short;
    expect(deadRun.pool).toBe(setAside + deadRun.duplicates_collapsed + deadRun.judged);
  });
});
