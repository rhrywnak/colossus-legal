// =============================================================================
// factsHeaderLine.test.ts — the PROD S-13 defect, pinned
// =============================================================================
//
// On 2026-09-11 the Scenario facts header read:
//
//     Scenario facts
//     No scan has run yet. Run a theme scan above…
//
// on a scenario whose history table, three lines below, listed a cancelled run
// of 313 candidates. Neither half was wrong on its own — the notice was served
// because no run had COMPLETED, and the table was showing what the database
// held. The screen was the lie.
//
// These tests are over the function that now makes both answers, so the pair is
// not a state the page can reach.

import { describe, expect, it } from "vitest";

import { factsHeaderLine } from "../factsHeaderLine";
import type { ScanRunState } from "../../services/themeScan";
import { NEVER_SCANNED, formatWhen, run, wording } from "./scanHeaderFixtures";

const base = { wording: wording(), neverScannedNotice: NEVER_SCANNED, formatWhen };

describe("the never-scanned notice", () => {
  it("is reachable ONLY on an empty history", () => {
    expect(factsHeaderLine({ ...base, runs: [] })).toBe(NEVER_SCANNED);
  });

  it("is unreachable on a history with a row in it, whatever that row is", () => {
    // The exhaustive form of the defect. Every state a run can be in, including
    // the three that are not `completed` — the cancelled case is the measured
    // one, and the other two would have failed the same way.
    const states: ScanRunState[] = ["running", "completed", "cancelled", "failed"];
    for (const status of states) {
      const line = factsHeaderLine({ ...base, runs: [run({ status })] });
      expect(line, `a ${status} run must not be described as never scanned`).not.toBe(
        NEVER_SCANNED,
      );
      expect(line).toContain("Last scan");
    }
  });

  it("is still SERVED, not composed — an absent notice yields no sentence", () => {
    // The browser does not own this wording. When the payload carries no notice
    // the header says nothing, rather than inventing the sentence it expected.
    expect(factsHeaderLine({ ...base, runs: [], neverScannedNotice: null })).toBeNull();
  });
});

describe("what the header says before it knows anything", () => {
  it("says nothing while the history has not been read", () => {
    // `null` is not `[]`. Collapsing the two would flash the never-scanned
    // notice onto every scenario for as long as its history took to load —
    // the same false sentence, arriving as a race instead of as a rule.
    expect(factsHeaderLine({ ...base, runs: null })).toBeNull();
  });

  it("says nothing while the words have not loaded", () => {
    // The absent-not-fake law: a header with no stored sentence renders no
    // sentence, rather than falling back to a literal compiled in here.
    expect(factsHeaderLine({ ...base, runs: [run()], wording: null })).toBeNull();
  });
});

describe("the last-scan line", () => {
  it("describes the NEWEST run, which the server has already ordered first", () => {
    const line = factsHeaderLine({
      ...base,
      runs: [
        run({ status: "cancelled", candidates_read: 313 }),
        run({ status: "completed", candidates_read: 148 }),
      ],
    });
    expect(line).toBe("Last scan Sep 11, 12:06 AM · cancelled · 313 candidates");
  });

  it("names the status in the lowercase register, not the history pill's", () => {
    // "Last scan Sep 11 · Complete · 313 candidates" reads as a proper noun
    // dropped into a sentence. The two vocabularies are separate rows for this.
    const line = factsHeaderLine({ ...base, runs: [run({ status: "completed" })] });
    expect(line).toContain("· completed ·");
    expect(line).not.toContain("Complete");
  });

  it("omits the count when the run never reached the pool read", () => {
    // `candidates_read` is NOT NULL, so a run that died before the read stores
    // 0. "0 candidates" claims it looked and found nothing; it did not look.
    const line = factsHeaderLine({
      ...base,
      runs: [run({ status: "failed", candidates_read: 0 })],
    });
    expect(line).toBe("Last scan Sep 11, 12:06 AM · failed");
    expect(line).not.toContain("0 candidates");
  });

  it("leaves no placeholder unfilled", () => {
    // A wrong or renamed slot ships the raw token to the screen, and the sentence
    // still renders — which is exactly why this is asserted rather than eyeballed.
    for (const runs of [[run()], [run({ candidates_read: 0 })]]) {
      expect(factsHeaderLine({ ...base, runs })).not.toMatch(/\{[a-z]+\}/);
    }
  });
});
