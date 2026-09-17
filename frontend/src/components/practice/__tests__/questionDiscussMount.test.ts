// =============================================================================
// questionDiscussMount.test.ts — opening "Discuss with AI" never disturbs the page
// =============================================================================
//
// CC_GO_QUESTION_CHAT_v1, STOP 1 ruling: a source-structure guard in the
// `dockPopoutCallSite.test.ts` style. There is no RTL/jsdom here (rule 30), so
// this cannot mount the page and type into it. It proves the STRUCTURE that makes
// the guarantee hold:
//
//   1. the drawer is a conditional SIBLING of the page content — rendered after
//      the page's card section closes, inside the page root — never a wrapper
//      around it and never a replacement for it;
//   2. the answer box's state (`draft`) is owned by the PAGE component, which the
//      drawer never remounts; the dock receives it as a read-only prop and cannot
//      set it.
//
// What this CANNOT prove: that React keeps the textarea's DOM node on open. That
// follows from (1) by React's reconciliation rules, and Roman opening the dock on
// DEV with a half-typed answer is what witnesses it.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

/** Source with comments removed, so prose about the rule cannot satisfy it. */
function code(source: string): string {
  return source
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^\s*\/\/.*$/gm, "");
}

const PAGE = code(readFileSync(join(__dirname, "..", "..", "..", "pages", "PracticeQuestionPage.tsx"), "utf8"));
const DOCK = code(readFileSync(join(__dirname, "..", "QuestionDiscussDock.tsx"), "utf8"));

describe("the Discuss dock's mount", () => {
  it("renders the drawer as a conditional sibling AFTER the page's card closes", () => {
    const pageReturn = PAGE.lastIndexOf("return (");
    const sectionClose = PAGE.lastIndexOf("</section>");
    const conditional = PAGE.indexOf("{discussOpen && (");
    const dock = PAGE.indexOf("<QuestionDiscussDock", conditional);
    expect(pageReturn).toBeGreaterThan(-1);
    expect(sectionClose).toBeGreaterThan(pageReturn);
    expect(conditional, "the drawer must be conditional on discussOpen").toBeGreaterThan(sectionClose);
    expect(dock).toBeGreaterThan(conditional);
    // ...and still inside the page root: the root div closes after it.
    expect(PAGE.lastIndexOf("</div>")).toBeGreaterThan(dock);
  });

  it("keeps the answer textarea OUTSIDE the conditional — it is never remounted by it", () => {
    const textarea = PAGE.indexOf("<textarea");
    const conditional = PAGE.indexOf("{discussOpen && (");
    expect(textarea).toBeGreaterThan(-1);
    expect(textarea).toBeLessThan(conditional);
  });

  it("owns the draft in the page; the dock only reads it", () => {
    expect(PAGE).toMatch(/const \[draft, setDraft\] = React\.useState/);
    expect(PAGE).toMatch(/<QuestionDiscussDock[\s\S]*?draft=\{draft\}/);
    expect(DOCK).not.toContain("setDraft");
    expect(DOCK).not.toMatch(/<textarea/);
  });
});
