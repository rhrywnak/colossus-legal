// =============================================================================
// questionDiscussMount.test.ts — opening the discussion never disturbs the page
// =============================================================================
//
// First written for the old dock (CC_GO_QUESTION_CHAT_v1, STOP 1 ruling);
// retargeted by CC_TASK_CHAT_ENGINE_v1 at the panel that replaced it, with the
// SAME guarantee. There is no RTL/jsdom here (rule 30), so this cannot mount the
// page and type into it. It proves the STRUCTURE that makes the guarantee hold:
//
//   1. the page is ALWAYS rendered inside the same shell and the same left pane,
//      whatever the panel's state — so opening, expanding or shutting the panel
//      changes a style and a sibling, never the tree above the answer box;
//   2. the panel is a conditional SIBLING of that left pane, after it — never a
//      wrapper around the question and never a replacement for it;
//   3. the answer box's state (`draft`) is owned by the PAGE, and the panel is
//      not given it at all.
//
// What this CANNOT prove: that React keeps the textarea's DOM node on open. That
// follows from (1) by React's reconciliation rules; Roman opening the panel on
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
const PANEL = code(readFileSync(join(__dirname, "..", "chat", "DiscussPanel.tsx"), "utf8"));

describe("the discussion panel's mount", () => {
  it("always renders the page inside the same shell and left pane", () => {
    const ret = PAGE.lastIndexOf("return (");
    const shell = PAGE.indexOf("<div ref={panes.containerRef}", ret);
    const page = PAGE.indexOf("<div style={s.page}", shell);
    expect(shell, "the shell is the root of the rendered page").toBeGreaterThan(ret);
    expect(page).toBeGreaterThan(shell);
    // No conditional sits between the shell and the page: the tree above the
    // answer box is the same in every panel state.
    const between = PAGE.slice(shell, page);
    expect(between).not.toMatch(/&&|\?\s*</);
  });

  it("renders the panel as a conditional sibling AFTER the question closes", () => {
    const sectionClose = PAGE.lastIndexOf("</section>");
    const conditional = PAGE.indexOf('{discussMode !== "closed" && (');
    const panel = PAGE.indexOf("<DiscussPanel", conditional);
    expect(conditional, "the panel must be conditional on the mode").toBeGreaterThan(sectionClose);
    expect(panel).toBeGreaterThan(conditional);
    expect(PAGE.lastIndexOf("</div>")).toBeGreaterThan(panel);
  });

  it("keeps the answer textarea OUTSIDE the conditional, and never hands the panel the draft", () => {
    const textarea = PAGE.indexOf("<textarea");
    expect(textarea).toBeGreaterThan(-1);
    expect(textarea).toBeLessThan(PAGE.indexOf('{discussMode !== "closed" && ('));
    expect(PAGE).toMatch(/const \[draft, setDraft\] = React\.useState/);
    expect(PAGE).not.toMatch(/<DiscussPanel[^>]*draft=/);
    expect(PANEL).not.toContain("setDraft");
    expect(PANEL).not.toMatch(/<textarea/);
  });
});
