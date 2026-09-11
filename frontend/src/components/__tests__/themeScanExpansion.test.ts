/**
 * Whether the scan card's body is showing (fix/running-view-visible).
 *
 * The rule that failed on DEV. S-13 had a scan going — 40 of 273 judged — and
 * four earlier FAILED runs; the failed runs gave the card a summary to fold to,
 * so it folded, so the running view did not render, while the header's own
 * tooltip said "A scan is running — its progress is below." Nothing was below it,
 * and in that layout there is no chevron to open it with.
 *
 * The first two tests are that scenario, exactly. The rest keep the ordinary
 * behaviour from being lost while fixing it.
 */
import { describe, expect, it } from "vitest";

import { isExpanded } from "../themeScanExpansion";

/** The ordinary case: nobody has clicked, no header lent, nothing running. */
const base = {
  expandOverride: null,
  collapsedSummary: null,
  running: false,
  headerLent: false,
};

describe("a running scan is always visible", () => {
  /**
   * DEV S-13, reproduced as data.
   *
   * A run in flight AND a settled run to describe. Before the fix this returned
   * false and the progress the header promised was nowhere on the page.
   */
  it("shows the body when a scan is running and an earlier run has settled", () => {
    expect(
      isExpanded({
        ...base,
        running: true,
        collapsedSummary: "Sep 10 · Qwen3.8 27B · 4 failed",
        headerLent: true,
      }),
    ).toBe(true);
  });

  /**
   * And it beats a human's own fold.
   *
   * Someone folds the card, then starts a scan — or folds it and comes back ten
   * minutes later. Honouring the click would hide the thing they are now waiting
   * on, and they would have to remember they had folded it to understand why the
   * page looks empty.
   */
  it("shows the body even when the human folded it by hand", () => {
    expect(
      isExpanded({
        ...base,
        running: true,
        expandOverride: false,
        collapsedSummary: "Sep 10 · Qwen3.8 27B · 31 waiting",
      }),
    ).toBe(true);
  });
});

describe("a lent header has no collapse control, so it never folds", () => {
  /**
   * The second half of the DEV defect, and the reason it was unrecoverable.
   *
   * The chevron only renders when `header` is undefined. On the scenario page the
   * caller draws the header, so a folded card there cannot be unfolded by any
   * means — `false` is not a preference, it is a dead end.
   */
  it("shows the body when the caller drew the header, even with a summary", () => {
    expect(
      isExpanded({ ...base, headerLent: true, collapsedSummary: "Sep 10 · Opus · 31 waiting" }),
    ).toBe(true);
  });

  it("shows the body when the caller drew the header and the human folded it", () => {
    expect(
      isExpanded({
        ...base,
        headerLent: true,
        expandOverride: false,
        collapsedSummary: "Sep 10 · Opus · 31 waiting",
      }),
    ).toBe(true);
  });
});

describe("the card's own layout keeps the behaviour it had", () => {
  /**
   * Ruling 4a, unchanged: once a run has settled, the work is in the queue below
   * and the card folds to one line. This is the case the whole collapse exists
   * for and the fix must not take it away.
   */
  it("folds once there is a settled run to describe", () => {
    expect(isExpanded({ ...base, collapsedSummary: "Sep 10 · Opus · 31 waiting" })).toBe(false);
  });

  /**
   * A never-scanned scenario opens expanded, because running the first scan IS
   * the work there.
   */
  it("opens when there is nothing to fold to", () => {
    expect(isExpanded(base)).toBe(true);
  });

  it("obeys a human who opened it", () => {
    expect(
      isExpanded({ ...base, expandOverride: true, collapsedSummary: "Sep 10 · Opus · 31 waiting" }),
    ).toBe(true);
  });

  it("obeys a human who closed it", () => {
    expect(
      isExpanded({ ...base, expandOverride: false, collapsedSummary: "Sep 10 · Opus · 31 waiting" }),
    ).toBe(false);
  });
});

describe("the panel actually uses this rule", () => {
  /**
   * Every assertion above is satisfiable by a helper nothing calls. RTL/jsdom are
   * not set up here (CLAUDE.md Rule 30), so this source scan is the available
   * fence — and it is the one that matters: the bug was a one-line expression
   * inline in the component, and re-inlining it would pass every test above.
   */
  it("calls isExpanded rather than re-deciding inline", async () => {
    const { readFileSync } = await import("node:fs");
    const { join } = await import("node:path");
    const panel = readFileSync(
      join(__dirname, "..", "ThemeScanPanel.tsx"),
      "utf8",
    );
    expect(panel).toContain('from "./themeScanExpansion"');
    expect(panel).toContain("isExpanded({");
    expect(panel).toContain("headerLent: header !== undefined");
    // The exact expression that shipped the bug.
    expect(panel).not.toContain("expandOverride ?? collapsedSummary === null");
  });
});
