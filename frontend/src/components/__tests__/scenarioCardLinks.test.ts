/**
 * The two controls on a Trial Prep card, and the one that left (2026-09-07).
 *
 * Source-level fences (CLAUDE.md Rule 30 — no RTL/jsdom tier here). What they
 * hold is the structural rule this card has been getting wrong in one direction
 * or another since the kebab: a control that navigates somewhere the card does
 * not must be a SIBLING of the card's link, never a child of it.
 *
 * ⚑ Written because nothing fenced any of this. "Open scenario →" sat inside the
 * card's own `<Link>` pointing at the page that link already opens, for two
 * releases, with no test able to notice.
 */
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

const CARD = join(__dirname, "..", "ScenarioCard.tsx");
const source = () => readFileSync(CARD, "utf8");

/**
 * The file with every comment removed — line, block AND JSX.
 *
 * The module's prose deliberately QUOTES the retired label to explain why it
 * went, so a fence reading raw source would find "Open scenario →" and fail
 * against its own documentation.
 *
 * ⚑ Stripping `//` lines alone is NOT enough here and the first draft of this
 * suite proved it: a `{ /* … *\/ }` JSX comment's continuation lines start with
 * ordinary prose, so a line-prefix filter keeps every one of them. The block
 * span has to go first, which also covers the JSX form — `{/* … *\/}` is a block
 * comment in braces. Same limitation the wording reach-scanner has.
 */
function code(): string {
  return source()
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .split("\n")
    .filter((line) => !line.trimStart().startsWith("//"))
    .join("\n");
}

describe('the retired "Open scenario →" affordance', () => {
  it("is gone from the card", () => {
    // It pointed at the page the whole card already navigates to — an arrow
    // announcing the click you were already making, on twelve cards at once.
    expect(code()).not.toContain("Open scenario");
  });

  it("did not simply become a second link to the same page", () => {
    // The regression that would look like a fix: replacing the span with a
    // `<Link to={scenarioPagePath(...)}>` in the same corner. The card IS that
    // link; a second one is the same redundancy wearing an anchor.
    //
    // Counted at the CALL SITE (`to={scenarioPagePath(`) rather than by name —
    // the import is an occurrence too, which is what made the first draft of
    // this assertion expect the wrong number.
    const uses = code().split("to={scenarioPagePath(").length - 1;
    expect(uses, "the scenario page is reached by the CARD, once").toBe(1);
  });
});

describe('"Go to Practice →", the control that replaced it', () => {
  it("goes to Practice through the guarded builder, never a hand-composed path", () => {
    // Law 8: this file composes no path of its own, so the route-link guard in
    // `routePaths.test.ts` covers this link the same way it covers the card's.
    const c = code();
    expect(c).toContain("practicePath(slug, scenario.id)");
    expect(c, "no hand-composed trial-prep path").not.toMatch(/`\/cases\/\$\{/);
  });

  it("carries the label Roman asked for", () => {
    expect(code()).toContain("Go to Practice →");
  });

  it("is a SIBLING of the card's link, not a child of it", () => {
    // The rule the component's own header states: an `<a>` inside an `<a>` is
    // invalid markup, and every click on the inner one would also navigate to
    // the outer one's target. Delete has obeyed this since 2026-08-07; this
    // asserts the new link does too, by position — it must appear after the
    // card link's `</Link>` closes.
    const c = code();
    const cardLinkCloses = c.indexOf("</Link>");
    const practiceAt = c.indexOf("practicePath(slug, scenario.id)");
    expect(cardLinkCloses, "the card's own link closes").toBeGreaterThan(-1);
    expect(practiceAt, "the practice link exists").toBeGreaterThan(-1);
    expect(
      practiceAt,
      "the practice link must be OUTSIDE the card's anchor",
    ).toBeGreaterThan(cardLinkCloses);
  });

  it("sits bottom-LEFT, opposite Delete, in the corner the span held", () => {
    // Two controls on one row that sit at different heights read as a mistake,
    // and the destructive one must stay as far from the other as the card can
    // put it.
    const c = code();
    expect(c).toContain('bottom: "8px", left: "10px"');
    expect(c).toContain('bottom: "8px", right: "10px"');
  });

  it("keeps room reserved for both controls in the card's flow", () => {
    // The removed span held that space as a real line (`marginTop: "auto"`).
    // Both controls are positioned against the card's edge and cannot reflow, so
    // without a reservation a two-line title on a short card runs under them.
    expect(code()).toContain("paddingBottom");
  });
});
