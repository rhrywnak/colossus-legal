/**
 * The practice palette's three halves must agree.
 *
 * `practiceStyles.ts` references `var(--practice-…)`; `styles/tokens.css`
 * defines those variables under `[data-surface="practice"]`; `PracticePage.tsx`
 * puts that attribute on the page. Break any one of the three and the screen
 * still RENDERS — with no colour at all, or with the mockup's colours silently
 * replaced by whatever the cascade offers.
 *
 * That is the failure this file exists for. It is the same shape as the .382
 * route defect and the .377 URL defect: files that must be edited to agree, with
 * nothing in the build able to notice when they stop.
 *
 * Reads source from disk, as `visualLanguage.test.ts` and `routePaths.test.ts`
 * already do.
 */
import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

const SRC = join(__dirname, "..", "..", "..");
const read = (...parts: string[]) => readFileSync(join(SRC, ...parts), "utf8");

const COMPONENTS = [
  "practiceStyles.ts",
  "practiceDeckStyles.ts",
  "practiceFlowStyles.ts",
  "practiceEditorStyles.ts",
  // The print sheets. In the list the day they were written: the palette test
  // is per-FILE, so a style module absent from it is a module where the hex
  // rule silently does not apply — which is exactly what happened here.
  "printStyles.ts",
  // The answers sheet's own block. Added the day it was written: this list is
  // hand-maintained, and a style file missing from it is a file whose colours
  // nothing checks — which is how `printStyles.ts` shipped 15 raw hex values.
  "printAnswerStyles.ts",
  "PracticeStart.tsx",
  "PracticeQuestion.tsx",
  "PracticeReveal.tsx",
  "PracticeSheet.tsx",
  "PracticeDeckList.tsx",
  "PracticeResume.tsx",
  "PracticeTopBar.tsx",
  "PracticePointsTo.tsx",
  "PracticeDeckRow.tsx",
  "PracticeRowEdit.tsx",
  "PracticeAddQuestion.tsx",
];

/** Every file that renders a practice screen and therefore needs the attribute. */
const PAGES = [
  "PracticePage.tsx",
  "PracticeSessionPage.tsx",
  "practiceChrome.tsx",
];

/**
 * Every `var(--practice-…)` and `var(--print-…)` the components reference.
 *
 * Both prefixes, because the print sheets carry a palette of their OWN: a
 * document's colours rather than a screen's, so that what comes out of a printer
 * does not follow a theme change. Matching only `--practice-` would have left
 * every print token unchecked — a `var(--print-typo)` renders as nothing at all,
 * and on paper that is an invisible rule or a colourless key box.
 */
function referencedTokens(): string[] {
  const sources = COMPONENTS.map((f) => read("components", "practice", f)).join("\n");
  return [
    ...new Set([...sources.matchAll(/var\((--(?:practice|print)-[a-z-]+)\)/g)].map((m) => m[1])),
  ];
}

describe("the practice palette", () => {
  it("defines every token the screens reference", () => {
    const tokens = read("styles", "tokens.css");
    const referenced = referencedTokens();

    // Anti-vacuity first: a regex that stopped matching would make the loop
    // below check nothing at all and report success.
    expect(referenced.length).toBeGreaterThanOrEqual(20);

    for (const token of referenced) {
      expect(tokens, `${token} is referenced but tokens.css defines no such variable`).toContain(
        `${token}:`,
      );
    }
  });

  it("scopes the palette so no other screen inherits it", () => {
    // The mockup's colours are Marie's drill's, not the product's. Promoted to
    // `:root` they would restyle screens nobody reviewed — the measured reason
    // ruling R1 scoped the v3 palette the same way.
    const tokens = read("styles", "tokens.css");
    expect(tokens).toContain('[data-surface="practice"]');
  });

  it("puts the scoping attribute on every state the pages can render", () => {
    // Loading, the load failure, the empty deck and the sitting itself are
    // separate returns, now spread over two pages and the frame they share. One
    // of them missing the attribute is one screen rendering with no palette —
    // and it would most likely be the failure screen, which is the one nobody
    // looks at until it matters.
    const sources = PAGES.map((f) => read("pages", f)).join("\n");
    const returns = sources.match(/<div style=\{s\.page\}/g) ?? [];
    const scoped = sources.match(/<div style=\{s\.page\} data-surface="practice"/g) ?? [];
    expect(returns.length).toBeGreaterThanOrEqual(3);
    expect(scoped.length).toBe(returns.length);
  });

  it("declares `font` before `fontSize` wherever a style sets both", () => {
    // The .401 defect, measured: `font` is a SHORTHAND and setting it resets
    // `font-size`. React writes a style object's properties in declaration
    // order, so `{ fontSize: 13, …, font: "inherit" }` renders at the body's
    // 18px — which is why the mockup check found the start card's row controls
    // "larger than drawn". Nothing else in the build can see it: both
    // properties are valid, both are typed, and the screen still renders.
    let checked = 0;
    for (const file of COMPONENTS) {
      const source = read("components", "practice", file);
      for (const block of source.split(/export const /).slice(1)) {
        const body = block.slice(0, block.indexOf("};"));
        const font = body.indexOf('font: "inherit"');
        const size = body.indexOf("fontSize:");
        if (font === -1 || size === -1) continue;
        checked += 1;
        expect(
          font,
          `${file}: ${block.slice(0, block.indexOf(":"))} sets fontSize before the ` +
            `font shorthand, which resets it — the control will render at 18px`,
        ).toBeLessThan(size);
      }
    }
    // ANTI-VACUITY: a split that stopped matching would check nothing and pass.
    // Six style objects set both today (three in the deck styles, three in the
    // flow styles); the floor is deliberately below that so an honest deletion
    // does not fail, and above zero so a broken parse does.
    expect(checked).toBeGreaterThanOrEqual(4);
  });

  it("keeps no hex literal in the components themselves", () => {
    // Rule 2. The palette is one named place; a hex that crept back into a style
    // object is a colour that stops following an edit to tokens.css.
    for (const file of COMPONENTS) {
      const source = read("components", "practice", file);
      expect(source, `${file} carries a hex colour literal`).not.toMatch(/#[0-9a-fA-F]{3,8}\b/);
    }
  });
});
