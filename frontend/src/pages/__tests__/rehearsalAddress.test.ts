// =============================================================================
// rehearsalAddress.test.ts — one mover, one address, and controls that look inert
// =============================================================================
//
// The two rehearsal-page rules the .382 fix put in place, both of which decay
// silently: they produce no error, no warning, and no visibly broken screen —
// only a page that behaves one way for the keyboard and another for the mouse,
// or a dead button that looks alive.
//
// These read the SOURCE and assert what the files DECLARE. Component testing
// (RTL/jsdom) is not set up in this repo (CLAUDE.md Rule 30), and the precedent
// for this shape is `scenarioPageStructure.test.ts` in this same directory,
// which has already caught two regressions.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { navButtonDisabledStyle, navButtonStyle } from "../../components/rehearsalStyles";

const SRC = join(__dirname, "..", "..");
const read = (...parts: string[]) => readFileSync(join(SRC, ...parts), "utf8");

describe("moving between scenarios takes the address along", () => {
  const page = read("pages", "RehearsalPage.tsx");

  it("routes the buttons through the same mover as the keys", () => {
    // The defect this pins: the keyboard updated the URL on every step and the
    // buttons did not, so the same position was linkable or not depending on how
    // the reader got there. Both call `move` now.
    expect(page).toContain('onPrevious={() => move("previous")}');
    expect(page).toContain('onNext={() => move("next")}');
  });

  it("has exactly one place that navigates between scenarios", () => {
    // Two navigate() calls would mean the two paths had drifted apart again —
    // which is precisely how they drifted the first time.
    const calls = page.match(/navigate\(/g) ?? [];
    expect(calls).toHaveLength(1);
  });

  it("composes that address through the guarded builder", () => {
    expect(page).toContain("rehearsalScenarioPath(slug, moved.code)");
  });

  it("replaces rather than pushes, so Back still means out of here", () => {
    expect(page).toContain("{ replace: true }");
  });
});

/**
 * The converted family: every file re-pointed through `utils/routePaths`.
 *
 * `RehearsalPageHeader` is first because it is where BOTH .382 dead links lived
 * — the breadcrumb and the "Scenario page ↗" control, the same wrong string
 * twice. Pinning only the page and not the header would have left the actual
 * scene of the defect unguarded, which is the gap the test-auditor caught.
 */
const CONVERTED_FAMILY = [
  ["components", "RehearsalPageHeader.tsx"],
  ["pages", "RehearsalPage.tsx"],
  ["components", "ScenarioHeaderTiers.tsx"],
  ["components", "TrialPrepViews.tsx"],
  ["pages", "ScenarioDetailPage.tsx"],
];

describe("no screen in the converted family spells a route by hand", () => {
  it.each(CONVERTED_FAMILY)("%s/%s composes through the builder", (dir, file) => {
    // An INTERPOLATED route literal — a backticked `/cases/…${…}` — is a route
    // spelled by hand, and the route-side URL guard in
    // `utils/__tests__/routePaths.test.ts` CANNOT SEE those: it proves what the
    // builders emit, not which call sites bothered to use them. This test is the
    // other half. Both .382 dead links took exactly this shape, as do all 23
    // hand-composed sites the survey found.
    //
    // Scoped to interpolation deliberately: a plain `/cases/…` inside a doc
    // comment is prose (RehearsalPage.tsx line 36 is exactly that), and a test
    // that flagged prose is a test people learn to work around.
    expect(read(dir, file)).not.toMatch(/`\/cases\/[^`]*\$\{/);
  });

  it("names files that exist, so a rename cannot empty this list quietly", () => {
    // Without this, deleting or renaming a file would make its guard vanish
    // rather than fail — the whole list could rot to nothing and stay green.
    expect(CONVERTED_FAMILY).toHaveLength(5);
    for (const [dir, file] of CONVERTED_FAMILY) {
      expect(read(dir, file).length).toBeGreaterThan(0);
    }
  });
});

describe("a bounded nav control looks as inert as it is", () => {
  it("differs from the live control on colour AND cursor", () => {
    // The .382 observable: `disabled` was set and honoured, but the base style
    // survived, so the button kept a pointer cursor and live text. Inert and
    // alive-looking is worse than either, because it reads as broken.
    expect(navButtonDisabledStyle.color).not.toBe(navButtonStyle.color);
    expect(navButtonDisabledStyle.cursor).not.toBe(navButtonStyle.cursor);
    expect(navButtonDisabledStyle.cursor).toBe("not-allowed");
  });

  it("dims with the app's existing token rather than a new hex", () => {
    // Standing Rule 2: a hex literal in a component is the one thing that cannot
    // follow a theme change.
    expect(navButtonDisabledStyle.color).toBe("var(--text-disabled)");
  });

  it("keeps the box identical, so only the aliveness changes", () => {
    // A disabled control that also moved or resized would read as a different
    // control appearing, not as this one being unavailable.
    expect(navButtonDisabledStyle.border).toBe(navButtonStyle.border);
    expect(navButtonDisabledStyle.padding).toBe(navButtonStyle.padding);
    expect(navButtonDisabledStyle.fontSize).toBe(navButtonStyle.fontSize);
  });

  it("is applied at both bounds in the header", () => {
    const header = read("components", "RehearsalPageHeader.tsx");
    expect(header).toContain("atFirst ? navButtonDisabledStyle : navButtonStyle");
    expect(header).toContain("atLast ? navButtonDisabledStyle : navButtonStyle");
  });
});
