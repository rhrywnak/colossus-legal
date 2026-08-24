// =============================================================================
// printChrome.test.ts — the application's navigation must not reach Chuck's paper
// =============================================================================
//
// ## Why this file exists
//
// Roman printed page 1 of S-7 from .405 and the product's own navigation bar was
// across the top of it: "Colossus Legal v2.0.0-beta.405 · Home · Trial Prep ▾ ·
// Documents · Chat · Admin ▾ · R Roman". The print view is a ROUTE INSIDE THE APP
// SHELL, so the shell renders around it; `PRINT_CSS` hid the print page's own two
// buttons and nothing hid the application's.
//
// Nothing in the suite noticed. Paper did. The instruction was explicit that the
// fix must not be "a `display: none` a later change can quietly undo" — so this
// pins BOTH HALVES OF THE COUPLING and, more importantly, pins them TO EACH OTHER.
//
// ## Why the coupling is the thing worth testing, not either half
//
// The rule and the attribute live in different files that no import connects:
// `Header.tsx` marks the shell, `printStyles.ts` hides what is marked. Two tests
// that each checked one half would both stay green if someone renamed the
// attribute in one file — which is exactly the silent return the instruction
// named. So the assertions below EXTRACT the selector from the stylesheet and go
// looking for that same string in the header. Rename it in one place and this
// fails; rename it in both and the coupling is intact, which is the correct
// outcome.
//
// ## Why a source scan and not a render
//
// The convention CLAUDE.md rule 30 records: this repo has no jsdom and no
// `@testing-library/*`, and `rehearsalPageStructure.test.ts` is the established
// precedent for structural facts read out of component source. The limit is worth
// stating: this proves the mark and the rule exist and agree. It cannot prove the
// browser honoured them — that is what the printed PDF in the report is for.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

import { PRINT_CSS } from "../printStyles";

const HEADER = join(__dirname, "..", "..", "Header.tsx");
const headerSource = () => readFileSync(HEADER, "utf8");

/** The `@media print` block alone — the rest of the sheet is screen styling. */
function printBlock(): string {
  const at = PRINT_CSS.indexOf("@media print");
  expect(at, "PRINT_CSS must have an @media print block at all").toBeGreaterThan(-1);
  return PRINT_CSS.slice(at);
}

describe("the app shell does not print", () => {
  it("the print rules hide a marked shell, and the shell carries that mark", () => {
    // Read the selector OUT of the stylesheet rather than writing it twice: this
    // is what makes the test see a rename instead of sleeping through one.
    const rule = /\[data-([a-z-]+)\]\s*\{\s*display:\s*none\s*!important/g;
    const hidden = [...printBlock().matchAll(rule)].map((m) => m[1]);

    expect(hidden, "print must hide something, or the app prints").not.toHaveLength(0);

    // One of the hidden marks must be the one the application shell wears. The
    // print page's own chrome (`data-print-chrome`) is the other, and it is not
    // this test's business.
    //
    // Read the marks out of the OPENING TAGS, not out of the file: this very
    // file's comments name the attribute, and the header's do too, so a
    // whole-source `includes` would keep passing over a header that had lost it.
    const tags = headerSource().match(/<[a-zA-Z][^>]*>/g) ?? [];
    const worn = hidden.filter((mark) => tags.some((tag) => tag.includes(`data-${mark}`)));

    expect(
      worn,
      `Header.tsx wears none of the marks print hides (${hidden.join(", ")}) — ` +
        "the navigation bar is back on Chuck's page 1",
    ).not.toHaveLength(0);
  });

  it("the mark is on the header ELEMENT, not merely mentioned in a comment", () => {
    // ANTI-VACUITY. The check above is satisfied by the string appearing anywhere
    // in the file, and this file's own explanatory comments contain it. What has
    // to be true is that the rendered `<header>` tag carries the attribute.
    const opening = headerSource().match(/<header[^>]*>/);

    expect(opening, "Header.tsx must render a <header> element").not.toBeNull();
    expect(
      opening![0],
      "the <header> tag itself must carry data-app-chrome — a comment does not hide anything",
    ).toContain("data-app-chrome");
  });

  it("hides it with !important, because the shell sets its own display", () => {
    // The shell header is a flex row styled inline from `headerStyle`. An inline
    // style outranks a stylesheet rule of any specificity, so without
    // `!important` the rule is present, correct, and completely ineffective —
    // a green test over a printed navigation bar.
    expect(printBlock()).toMatch(
      /\[data-app-chrome\]\s*\{\s*display:\s*none\s*!important\s*;?\s*\}/,
    );
  });
});
