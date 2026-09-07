// =============================================================================
// scenarioStatusControl.test.ts — the same-segment no-op rule (task 1.7D)
// =============================================================================
//
// The segmented status control replaced a two-directional button, and with a
// segmented control a human can click the segment they are ALREADY on — something
// the old button made impossible, because it only ever offered the other
// direction.
//
// That new affordance created a new way to be wrong. The ready route writes a row
// to the status-transitions ledger, which is the forensic record of who declared a
// scenario's prep complete and when. A POST fired for a click that changes nothing
// would record a transition that never happened — and a false entry in that ledger
// is worse than a missing feature, because it is evidence about a human's act.
//
// (The ledger's meaning was worded "declared a scenario rehearsable" until v2.1
// retired the rehearsal page. The guard below is untouched; only the word for what
// the declaration means moved, and the tooltip suite at the foot of this file is
// what stops it drifting again.)
//
// So the guard is a correctness rule, not a visual convention, and it is a pure
// function so it can be tested without component-render infrastructure (which this
// repo deliberately does not have — CLAUDE.md Rule 30).

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { shouldApplyStatus } from "../ScenarioStatusControl";

describe("shouldApplyStatus — the same-segment no-op guard", () => {
  it("does nothing when Draft is clicked on a scenario that is already Draft", () => {
    expect(shouldApplyStatus(false, false)).toBe(false);
  });

  it("does nothing when Ready is clicked on a scenario that is already Ready", () => {
    expect(shouldApplyStatus(true, true)).toBe(false);
  });

  it("fires when switching Draft → Ready", () => {
    // The act that declares this scenario's prep complete. It must reach the server.
    expect(shouldApplyStatus(true, false)).toBe(true);
  });

  it("fires when switching Ready → Draft", () => {
    // Demotion must reach the server too — withdrawing the claim that a scenario's
    // prep is complete is as much a recorded human act as making it, and the ledger
    // needs both ends of the round trip or its history has gaps.
    expect(shouldApplyStatus(false, true)).toBe(true);
  });

  it("is symmetric: only the DIFFERENCE matters, not which way", () => {
    // Stated as a property so the rule cannot drift into "only promotion counts",
    // which is the shape a well-meaning optimisation would take.
    for (const [next, current] of [
      [true, false],
      [false, true],
    ] as const) {
      expect(shouldApplyStatus(next, current)).toBe(true);
    }
    for (const [next, current] of [
      [true, true],
      [false, false],
    ] as const) {
      expect(shouldApplyStatus(next, current)).toBe(false);
    }
  });
});

// ── The tooltip says something TRUE (Roman's ruling, 2026-09-07) ─────────────
//
// ⚑ This suite exists because nothing fenced this sentence, and it was false for
// a release. It read "Ready = appears in Marie's rehearsal view" after v2.1
// retired that page, and the replacement first proposed — "appears in Practice"
// — would have been false too, because Practice is deliberately ungated.
//
// Read from SOURCE rather than imported: `TOOLTIP` is component-local, and
// exporting a constant purely so a test can see it widens the component's surface
// for the test's convenience. Reading the file also catches the other way this
// regresses — someone re-inlining the literal at the call site.

describe("the Ready tooltip names what Ready actually does", () => {
  const source = readFileSync(
    join(__dirname, "..", "ScenarioStatusControl.tsx"),
    "utf8",
  );
  /** Non-comment lines only, so the history in the module's own prose is not read
   *  as the live string — that prose deliberately QUOTES both retired sentences. */
  const code = source
    .split("\n")
    .filter((line) => !line.trimStart().startsWith("//"))
    .join("\n");

  it("claims the Trial Prep list, which is what Ready still drives", () => {
    // Verified on this branch: `count_status(cards, ScenarioStatus::Ready)` feeds
    // the served `metric_ready_label`, and `statusMeta("ready")` is the green chip
    // `ScenarioCard` renders. Both are on the Trial Prep list.
    expect(code).toContain("you have declared this scenario's prep complete");
    expect(code).toContain("Shows on the Trial Prep list");
    expect(code).toContain("Human-only switch");
  });

  it("names NEITHER retired surface", () => {
    // The two false sentences, by name. Rehearsal is gone (v2.1, ruling R35);
    // Practice exists but does not gate on Ready, so a tooltip promising it would
    // send a human to a page that behaves identically either way.
    expect(code, "the rehearsal page is retired").not.toMatch(/rehearsal view/i);
    expect(code, "Practice is not gated on Ready").not.toMatch(
      /appears in Practice/i,
    );
  });

  it("is still attached to the control, not merely declared", () => {
    // A tooltip nobody renders is a string, not a tooltip. The whole reason this
    // sentence went stale unnoticed is that nothing connected the two.
    expect(code).toMatch(/title=\{TOOLTIP\}/);
  });
});
