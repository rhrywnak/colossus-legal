// =============================================================================
// headerStripRules.test.ts — which of the strip's controls are live
// =============================================================================
//
// T5.4's first named suite. The rule is one line of code and four years of
// defect history, which is why it is asserted at every status the column
// actually permits rather than at the two everybody thinks of.

import { describe, expect, it } from "vitest";

import { showsViewTimeline, stripControls } from "../headerStripRules";

describe("stripControls — the rehearsal gate", () => {
  it("lets a READY scenario into rehearsal", () => {
    expect(stripControls("ready").rehearsalEnabled).toBe(true);
  });

  it("refuses a DRAFT one — the .389 defect, by name", () => {
    // The control used to render identically at every status. Clicked on a Draft
    // scenario it landed on the rehearsal page with no code in the URL, which
    // left that page's index clamped at 0, which rendered a DIFFERENT scenario
    // under its own title with no notice of any kind.
    expect(stripControls("draft").rehearsalEnabled).toBe(false);
  });

  it("refuses NEEDS_EVIDENCE, which is why the test is `=== ready`", () => {
    // ⚑ The reason the predicate is not `!== "draft"`. The status column permits
    // a third value (ruling 6), and a scenario that needs evidence is exactly the
    // kind nobody should be taken into a rehearsal on. A negated test would have
    // let it through.
    expect(stripControls("needs_evidence").rehearsalEnabled).toBe(false);
  });

  it("refuses a status nobody has invented yet", () => {
    // The same property from the other side: an unrecognised status fails
    // CLOSED. A new value added to the column tomorrow does not silently open
    // the gate before anyone decides it should.
    expect(stripControls("archived").rehearsalEnabled).toBe(false);
    expect(stripControls("").rehearsalEnabled).toBe(false);
  });
});

describe("stripControls — the three that are never gated", () => {
  // Practice is the ASYMMETRY worth pinning: it is the surface that reports "this
  // scenario has no deck yet", so gating it would hide the only screen able to
  // say so. Edit and Delete are ungated because a half-authored scenario is the
  // normal case and the confirm dialog is Delete's guard (Roman, 2026-08-07).
  for (const status of ["draft", "ready", "needs_evidence", "anything"]) {
    it(`leaves practice, edit and delete alive at "${status}"`, () => {
      const c = stripControls(status);
      expect(c.practiceEnabled).toBe(true);
      expect(c.editEnabled).toBe(true);
      expect(c.deleteEnabled).toBe(true);
    });
  }
});

describe("showsViewTimeline", () => {
  it("is ABSENT at zero, not disabled — Screen 1's own words", () => {
    // "when no subset is attached the button is simply absent — nothing else
    // shifts". A disabled button would offer something that does not exist:
    // there is no story to view until somebody attaches one, and the place to do
    // that is the Timeline subsets section, not this strip.
    expect(showsViewTimeline(0)).toBe(false);
  });

  it("appears as soon as the scenario carries one", () => {
    expect(showsViewTimeline(1)).toBe(true);
    expect(showsViewTimeline(7)).toBe(true);
  });
});
