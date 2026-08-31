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
  ["components/scenario", "ScenarioHeaderStrip.tsx"],
  ["components", "TrialPrepViews.tsx"],
  ["pages", "ScenarioDetailPage.tsx"],
  // The sixth, added with the surface it serves (task R1 Piece 1d). Every row of
  // the rehearsal picker composes an address, which makes it a route-composing
  // file and therefore a file this guard has to cover — a new screen is exactly
  // where a hand-spelled route creeps back in.
  ["components", "RehearsalPicker.tsx"],
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
    expect(CONVERTED_FAMILY).toHaveLength(6);
    for (const [dir, file] of CONVERTED_FAMILY) {
      expect(read(dir, file).length).toBeGreaterThan(0);
    }
  });
});

/**
 * The guard that the .382 pair could not provide (task R1 Piece 1e, audit defect 6).
 *
 * ## What the existing two halves prove, and what they cannot
 *
 * `routePaths.test.ts` proves what each builder EMITS matches what `App.tsx`
 * DECLARES. The `CONVERTED_FAMILY` block above proves no screen in this family
 * spells a route by hand. Both passed, green, on beta.389 — while the scenario
 * page's rehearsal control composed `rehearsalPath(slug)`: a perfectly spelled,
 * correctly declared, correctly escaped address to the WRONG SUBJECT. It carried
 * no scenario at all, and the rehearsal page then rendered whichever scenario sat
 * at index 0.
 *
 * A URL guard that checks spelling cannot catch an argument. So this block checks
 * the argument: the header must compose the address that NAMES a scenario, and
 * must not compose the case-level one.
 */
describe("the rehearsal control names the scenario it is on", () => {
  const header = read("components", "scenario", "ScenarioHeaderStrip.tsx");

  it("composes the per-scenario address with this scenario's own code", () => {
    // `rehearsalScenarioPath` was routed, tested and documented for two releases
    // with NO producer anywhere in the app. This is its first caller.
    // T5: the strip self-fetches, so the code comes off the identity it read
    // rather than off a prop. The ARGUMENT is what this guard is about and it is
    // unchanged — the address still names this scenario.
    expect(header).toContain("rehearsalScenarioPath(slug, identity.code)");
  });

  it("cannot reach the case-level rehearsal address at all", () => {
    // The defect itself. `rehearsalPath(slug)` opens rehearsal with nothing
    // named, which is how a Draft scenario's control delivered a different
    // scenario's rehearsal — twice in one round trip, silently.
    //
    // Asserted on the IMPORT rather than on the body, for the reason the
    // `CONVERTED_FAMILY` block states one screen up: this file explains the
    // defect it fixes, and a test that flags prose is a test people learn to
    // work around. A builder that is not imported cannot be called.
    const imports = header.slice(0, header.indexOf("const eyebrowStyle"));
    expect(imports).toContain("rehearsalScenarioPath");
    expect(imports).not.toMatch(/\brehearsalPath\b/);
  });

  it("gates the live link on the scenario actually being ready", () => {
    // T5: the RULE moved to `headerStripRules.ts`, where a unit test can reach
    // it — the component now asks `controls.rehearsalEnabled`. Both halves are
    // asserted, so neither can drift: the rule says "ready", the strip obeys it.
    const rules = read("components", "scenario", "headerStripRules.ts");
    expect(rules).toContain('status === "ready"');
    expect(header).toContain("controls.rehearsalEnabled");
    // Rehearsal mode serves READY scenarios through a human gate (v2 §5/§10).
    // Before .390 the control rendered identically at every status and the only
    // statement of the rule was a hover tooltip. The literal now lives in the
    // rules module asserted two lines up — asserting it HERE as well would pin
    // the rule to the component it was just extracted out of.
  });

  it("says why it is inert in a STORED sentence, never a literal", () => {
    // The reason is a settings row filled with the status on screen. A literal
    // here would be the one word on this control the configuration law cannot
    // reach — and it would have to guess "Draft", which is wrong for any other
    // non-Ready value the column still permits.
    // T5: the stored sentence is now the SHORT tooltip row, and it reaches the
    // strip on the payload the strip already reads rather than as a prop. Still
    // a stored row, never a literal, which is what this test is for.
    expect(header).toContain("wording.rehearsal_disabled_tooltip");
    // T5: the tooltip row has NO placeholder to fill. The long
    // `{status}`-bearing sentence stayed in the store (the rehearsal gate still
    // speaks it) but left this control: a tooltip on a button disabled for
    // being Draft need not say "Draft" when the segmented control beside it
    // does. So this asserts the absence of a hand-written reason instead.
    expect(header).not.toContain("Not in rehearsal");
    expect(header).not.toContain("Switch it to Ready");
  });
});

/**
 * The rehearsal page shows a refusal INSTEAD OF a scenario, never beside one.
 *
 * Roman's ruling of 2026-08-10, and the second half of the S-5 → S-2 fix. The
 * three modes are mutually exclusive by construction; before .390 the not-ready
 * notice and `scenarios[0]`'s full blocks rendered together, which is a refusal
 * that nonetheless produces content under another scenario's title.
 */
describe("the rehearsal page is in exactly one mode at a time", () => {
  const page = read("pages", "RehearsalPage.tsx");

  it("derives one mode rather than testing conditions in three places", () => {
    expect(page).toContain('const mode = notReady ? "refusing" : picked ? "rehearsing" : "picking"');
  });

  it("renders the blocks ONLY while rehearsing", () => {
    expect(page).toContain('{mode === "rehearsing" &&');
  });

  it("renders the refusal ONLY while refusing", () => {
    expect(page).toContain('{mode === "refusing" && (');
  });

  it("offers the picker when nobody has picked", () => {
    // The front door. A bare `/rehearsal` used to open on the first ready
    // scenario and say nothing about having chosen for her.
    expect(page).toContain('{mode === "picking" && (');
    expect(page).toContain("<RehearsalPicker");
  });

  it("withholds the scenario's name from the header outside rehearsing mode", () => {
    // The breadcrumb and "Scenario page ↗" compose their address from the
    // scenario on screen. Left populated during a refusal, they would point at a
    // scenario the reader never asked for — which is exactly how the .389 round
    // trip ended at S-2 twice.
    expect(page).toContain('scenario={mode === "rehearsing" ? scenario : undefined}');
  });
});

/**
 * The prep page is READ-ONLY, and the phase filter offers only what it can show.
 *
 * Both rules decay silently. An edit control creeping back onto this surface
 * looks like a feature; a phase chip that filters to nothing looks like a broken
 * control. Neither produces an error, and this repo has no component-testing
 * infrastructure (rule 30), so the pins read the source — the same shape as the
 * `CONVERTED_FAMILY` block above.
 */
describe("the prep page stays read-only", () => {
  /**
   * The IMPORTS of each prep component, not the whole file.
   *
   * These files explain what they removed and name the editors they no longer
   * use, so a whole-file scan would match its own prose — the trap the
   * `CONVERTED_FAMILY` block one screen up documents. A component that is not
   * imported cannot be rendered, and a field that is not read cannot be shown.
   */
  const importsOf = (file: string) => {
    const source = read("components", file);
    const firstStyle = source.search(/^const \w+(Style|Row)/m);
    return firstStyle > 0 ? source.slice(0, firstStyle) : source;
  };
  const heads = [
    importsOf("RehearsalScenarioBlocks.tsx"),
    importsOf("PrepInstanceCard.tsx"),
    importsOf("PrepTopBlock.tsx"),
  ];

  it("imports no editor onto any block", () => {
    for (const head of heads) {
      expect(head).not.toMatch(/^import .*SentenceEditor/m);
    }
  });

  it("carries no write path of its own", () => {
    // Every edit routes to the working page through the header's one link.
    // `rehearsalEdits.ts` was deleted with this build; a fetch reappearing here
    // would be a second write path onto a surface that must not have one.
    for (const head of heads) {
      // Importing TYPES from the service is fine and expected; importing its
      // fetchers is not.
      expect(head).not.toContain("authFetch");
      expect(head).not.toContain("fetchRehearsal");
    }
    for (const file of ["RehearsalScenarioBlocks.tsx", "PrepInstanceCard.tsx", "PrepTopBlock.tsx"]) {
      expect(read("components", file)).not.toContain("fetch(");
    }
  });

  it("shows no authorship line", () => {
    // "Written in plain words by roman · Aug 7" is provenance for the person who
    // wrote it, on the page where they wrote it — not for a witness rehearsing.
    for (const file of ["RehearsalScenarioBlocks.tsx", "PrepInstanceCard.tsx", "PrepTopBlock.tsx"]) {
      expect(read("components", file)).not.toContain("what_this_is_attribution");
    }
  });
});

describe("the phase filter offers only the phases it can show", () => {
  const blocks = read("components", "RehearsalScenarioBlocks.tsx");

  it("derives its chips from the instances present, not from the four phases", () => {
    // A chip for a phase with no cards would filter to nothing, which reads as a
    // broken control rather than an empty phase.
    expect(blocks).toContain("instances.reduce<string[]>");
    expect(blocks).toContain("if (!seen.includes(i.phase)) seen.push(i.phase)");
  });

  it("hides the filter row entirely when every card shares one phase", () => {
    // One chip that cannot narrow anything is a control with no purpose.
    expect(blocks).toContain("phases.length > 1 &&");
  });

  it("clears the filter by tapping the active chip", () => {
    expect(blocks).toContain("setPhase(phase === name ? null : name)");
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
