// =============================================================================
// practiceStartCard.test.ts — the start card warns instead of encouraging
// =============================================================================
//
// ## The defect this file stands over
//
// The line under each scenario title invited a witness to rehearse — "Answer out
// loud first, then type it in a sentence or two" — on a deck of questions no
// attorney has read. It is now a warning that says so, and the warning is
// PERMANENT: it does not vanish once a deck is reviewed, because "reviewed" is
// not a state this system tracks, and inventing one in order to hide a warning
// is the wrong trade.
//
// ## Why source scans (the standing limit, restated)
//
// No jsdom, no `@testing-library/*` — CLAUDE.md rule 30, and the precedent is
// `rehearsalPageStructure.test.ts`. A source scan proves the component reads the
// right field and renders no literal in its place. It cannot prove the result is
// legible on screen; Roman's walk is what knows that.
//
// The VALUE of the warning is not pinned here — it is pinned in
// `backend/src/domain/wording_practice_tests.rs`, against the migration that
// seeds it, which is the only place that can see both.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

const HERE = join(__dirname, "..");
const read = (file: string) => readFileSync(join(HERE, file), "utf8");

describe("the start card warns instead of encouraging", () => {
  it("renders the stored intro row and no literal in its place", () => {
    const start = read("PracticeStart.tsx");

    expect(start).toContain('w("intro")');
    // The words that were there. If any of them is ever inlined, the row stops
    // being the thing on screen and Settings stops being able to change it.
    expect(start).not.toMatch(/Twenty minutes|out loud|nobody watching/);
  });

  it("does not condition the warning on anything", () => {
    // Roman's ruling: PERMANENT. A warning wrapped in `{reviewed && ...}` or a
    // ternary is a warning with an off switch, and the state that would switch it
    // does not exist in this system. Assert the render site is unguarded.
    const start = read("PracticeStart.tsx");
    const line = start.split("\n").find((l) => l.includes('w("intro")'));

    expect(line, "the intro must be rendered somewhere").toBeDefined();
    expect(line).not.toMatch(/&&|\?/);
  });

  it("stays off the paper", () => {
    // Ruled with it: it does not print. Chuck's sheets are aimed at the very
    // attorney this line asks for; printing it would be the paper asking him to
    // wait for himself. The print view reads its own block, so this asserts the
    // practice-flow key never appears in the print components.
    for (const file of ["PrintSheets.tsx", "printSheetPlan.ts"]) {
      expect(read(file), `${file} must not carry the start card's warning`).not.toContain(
        '"intro"',
      );
    }
  });
});
