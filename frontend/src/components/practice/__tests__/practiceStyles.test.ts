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
  "PracticeStart.tsx",
  "PracticeQuestion.tsx",
  "PracticeReveal.tsx",
  "PracticeSheet.tsx",
];

/** Every `var(--practice-…)` the style module and its components reference. */
function referencedTokens(): string[] {
  const sources = COMPONENTS.map((f) => read("components", "practice", f)).join("\n");
  return [...new Set([...sources.matchAll(/var\((--practice-[a-z-]+)\)/g)].map((m) => m[1]))];
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

  it("puts the scoping attribute on every state the page can render", () => {
    // Loading, the load failure, the empty deck, and the session itself are four
    // separate returns. One of them missing the attribute is one screen rendering
    // with no palette — and it would most likely be the failure screen, which is
    // the one nobody looks at until it matters.
    const page = read("pages", "PracticePage.tsx");
    const returns = page.match(/<div style=\{s\.page\}/g) ?? [];
    const scoped = page.match(/<div style=\{s\.page\} data-surface="practice"/g) ?? [];
    expect(returns.length).toBeGreaterThanOrEqual(4);
    expect(scoped.length).toBe(returns.length);
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
