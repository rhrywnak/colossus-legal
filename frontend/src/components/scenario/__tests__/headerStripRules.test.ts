// =============================================================================
// headerStripRules.test.ts — which of the strip's controls are live
// =============================================================================
//
// T5.4's first named suite. The rule is one line of code and four years of
// defect history, which is why it is asserted at every status the column
// actually permits rather than at the two everybody thinks of.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { isKnownDirection, showsViewTimeline, stripControls } from "../headerStripRules";

describe("stripControls — the rehearsal gate is GONE (v2.1, ruling R35)", () => {
  // The suite it replaces asserted `rehearsalEnabled` at four statuses, and it
  // was right to: that one branch was the whole reason this module exists. The
  // page it gated is retired, the control is off the strip, and the flag is off
  // the type — so what is worth asserting now is that nothing grew back.
  //
  // Reading the SOURCE rather than calling the function is deliberate: a flag
  // that no longer exists cannot be asserted `toBe(false)` through the type, and
  // `expect((c as any).rehearsalEnabled)` would pass just as happily against a
  // typo. The fence has to be able to fail.
  it("names no rehearsal control, and gates nothing on the status", () => {
    const source = readFileSync(
      join(__dirname, "..", "headerStripRules.ts"),
      "utf8",
    );
    const code = source
      .split("\n")
      .filter((line) => !line.trimStart().startsWith("//") && !line.trimStart().startsWith("*"))
      .join("\n");

    expect(code, "no control may be named `rehearsal…` again").not.toMatch(
      /rehearsalEnabled/,
    );
    expect(
      code,
      'nothing on this strip compares the status — see the module note, and the ' +
        'v2.1 STOP: Practice does not gate on Ready either',
    ).not.toMatch(/status\s*===/);
  });

  it("still refuses to grow a hidden branch: every status yields one answer", () => {
    // The property the deleted suite was really protecting — that a reader can
    // predict this strip from the status alone. It now has exactly one answer,
    // and this asserts that rather than trusting the constant three lines up.
    const answers = ["draft", "ready", "needs_evidence", "archived", ""].map((s) =>
      JSON.stringify(stripControls(s)),
    );
    expect(new Set(answers).size).toBe(1);
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

describe("isKnownDirection — which colour the role chip takes", () => {
  it("recognises the two directions this build names", () => {
    expect(isKnownDirection("offense")).toBe(true);
    expect(isKnownDirection("defense")).toBe(true);
  });

  it("refuses anything else, so an unknown posture goes AMBER not red", () => {
    // "Offensive" on a scenario the database calls something else would be the
    // page inventing a posture. The chip shows the raw token in amber instead.
    expect(isKnownDirection("neutral")).toBe(false);
    expect(isKnownDirection("Offense")).toBe(false); // case matters: it is a column value
    expect(isKnownDirection("")).toBe(false);
  });
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
