// =============================================================================
// forYouTokens.test.ts — every colour the For you page draws actually exists
// =============================================================================
//
// CC_TASK_FOR_YOU_POLISH_v1, phase D. A DISK/CODE CONSISTENCY test (CLAUDE.md
// rule 21): the invariant it guards holds across four files and a stylesheet,
// and review alone has already failed to catch it once.
//
// ## The defect this exists to stop happening twice
//
// The page shipped on 2026-09-22 declaring `data-surface="for-you"` while every
// style it drew read a `var(--practice-…)` token. Those tokens are defined on
// `[data-surface="practice"]`; custom properties INHERIT, and this page is not
// inside that subtree, so all of them resolved to nothing. An undefined custom
// property with no fallback makes the whole declaration invalid at
// computed-value time: `border: 1px solid var(--practice-line)` becomes no
// border at all, `background: var(--practice-paper)` becomes transparent.
//
// Measured in a browser on 2026-09-23, the same `<div>` under the two surfaces:
//
//   data-surface="for-you"   border 0px none rgb(0,0,0)        bg rgba(0,0,0,0)
//   data-surface="practice"  border 1px solid rgb(223,228,234)  bg rgb(255,255,255)
//
// Nothing failed. `tsc` was clean, the tests were green, the page rendered, and
// what it rendered was bare text lines where the ratified board draws cards.
// That is the exact shape of a silent failure, and Standing Rule 1 says it must
// become observable. This is where it becomes observable.
//
// ## Why a source scan and not a rendered assertion
//
// There is no jsdom or component rig in this project (rule 30), and a rendered
// assertion would not have caught it anyway: the page rendered perfectly well.
// The claim is about two FILES agreeing — what the styles ask for, and what the
// stylesheet defines for the surface the page declares — and that is a claim a
// scan can make exactly.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

const SRC = join(__dirname, "..", "..", "..");
const FOR_YOU = join(SRC, "components", "forYou");

const read = (...parts: string[]) => readFileSync(join(...parts), "utf8");

/**
 * Source with its comments removed — this repo documents beside its code, and
 * those comments NAME the tokens they are discussing. Scanning them would make
 * a sentence about the old palette read as a use of it, which is how the first
 * run of this test failed on its own explanatory header.
 */
function withoutComments(source: string): string {
  return source
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .split("\n")
    .map((line) => {
      const at = line.indexOf("//");
      return at === -1 ? line : line.slice(0, at);
    })
    .join("\n");
}

/** The stylesheet, once. */
const tokens = () => read(SRC, "styles", "tokens.css");

/** Every file that draws the page. */
const drawingFiles = (): Array<[string, string]> => [
  ["forYouStyles.ts", withoutComments(read(FOR_YOU, "forYouStyles.ts"))],
  ["ForYouRow.tsx", withoutComments(read(FOR_YOU, "ForYouRow.tsx"))],
  ["ForYouPage.tsx", withoutComments(read(SRC, "pages", "ForYouPage.tsx"))],
];

/** `var(--x)` and `var(--x, fallback)` → `--x`. */
function varsIn(source: string): string[] {
  return [...source.matchAll(/var\(\s*(--[a-z0-9-]+)/gi)].map((m) => m[1]);
}

/**
 * The custom-property names a CSS block DEFINES.
 *
 * `selector` is matched literally and the block is taken to the first closing
 * brace at the start of a line — which is how every block in `tokens.css` is
 * written, and a shape the stylesheet's own tests already rely on.
 */
function definedUnder(css: string, selector: string): string[] {
  const at = css.indexOf(selector);
  if (at === -1) return [];
  const end = css.indexOf("\n}", at);
  const block = css.slice(at, end === -1 ? undefined : end);
  return [...block.matchAll(/^\s*(--[a-z0-9-]+)\s*:/gim)].map((m) => m[1]);
}

describe("the For you page's palette", () => {
  it("declares a surface tokens.css actually defines", () => {
    // The whole defect in one line: the page named a surface, and nothing in
    // the stylesheet answered to that name.
    const page = read(SRC, "pages", "ForYouPage.tsx");
    const declared = page.match(/data-surface="([a-z0-9-]+)"/i);
    expect(declared, "ForYouPage no longer declares a surface at all").not.toBeNull();
    expect(tokens()).toContain(`[data-surface="${declared?.[1] ?? ""}"]`);
  });

  it("asks for no custom property that the page's own surface leaves undefined", () => {
    const css = tokens();
    // Everything the page can legitimately see: its own surface block, plus the
    // global `:root`, which every subtree inherits.
    const available = new Set([
      ...definedUnder(css, '[data-surface="for-you"] {'),
      ...definedUnder(css, ":root {"),
    ]);
    expect(available.size, "neither block was found in tokens.css").toBeGreaterThan(10);

    for (const [name, source] of drawingFiles()) {
      for (const used of varsIn(source)) {
        expect(
          available.has(used),
          `${name} draws with ${used}, which is not defined for the For you ` +
            `surface or at :root — it will resolve to nothing and the ` +
            `declaration using it will be thrown away silently`,
        ).toBe(true);
      }
    }
  });

  it("reads no --practice- token, on any of the three files", () => {
    // The specific wrong palette, named. A style that reaches for it again is
    // reaching into a subtree this page is not inside — even if, one day, some
    // other surface happens to define a token of that name.
    for (const [name, source] of drawingFiles()) {
      expect(
        varsIn(source).filter((v) => v.startsWith("--practice-")),
        `${name} reads a practice token; the For you page is not inside ` +
          `[data-surface="practice"] and it will resolve to nothing`,
      ).toEqual([]);
    }
  });

  it("does not IMPORT the practice palette either", () => {
    // ⚑ The hole the test above cannot see, and the one the page actually fell
    // through: `import { LINE } from "../practice/practiceStyles"` hides the
    // token behind an identifier, so a scan for `var(--practice-…)` finds
    // nothing while every border on the page is still drawn from a palette this
    // surface cannot see. Run against the shipped file, the scan above caught
    // ONE of its seven wrong colours — the only one written out in full.
    //
    // The styles a shared component legitimately needs are copied to this
    // surface's own block in tokens.css, by the mockup's own names, which is
    // the rule the practice palette itself records.
    for (const [name, source] of drawingFiles()) {
      expect(
        source,
        `${name} imports from the practice styles; its tokens are scoped to a ` +
          `surface the For you page is not inside`,
      ).not.toMatch(/from\s+"[^"]*practice\/practiceStyles"/);
    }
  });

  it("keeps the menu badge's colour at :root, where the header can see it", () => {
    // The badge is drawn by `Header.tsx`, which sits OUTSIDE every surface
    // subtree. A colour for it scoped to the For you page would be the same
    // defect again, one component over.
    const css = tokens();
    expect(definedUnder(css, ":root {")).toContain("--for-you-badge-bg");
    expect(withoutComments(read(FOR_YOU, "ForYouBadge.tsx"))).toContain(
      "var(--for-you-badge-bg)",
    );
  });

  it("defines the board-4 pair on the surface the deck page declares", () => {
    // Board 4 is drawn by the PRACTICE surface, so its two values belong in
    // that block — the mirror image of the rule above, and the reason the mark
    // does not simply reuse the For you page's tokens.
    const practice = definedUnder(tokens(), '[data-surface="practice"] {');
    expect(practice).toContain("--practice-waiting-row-bg");
    expect(practice).toContain("--practice-waiting-ink");
  });
});
