// =============================================================================
// practicePolishSurface.test.ts — the four changes of 2026-09-15, on the page
// =============================================================================
//
// CC_TASK_PRACTICE_POLISH_v1: the side the page opens on, the receipt becoming
// editable, the answer-analysis switch, and the tactic dropdown that replaces a
// number box.
//
// ## Why source scans
//
// No jsdom, no `@testing-library/*` — CLAUDE.md rule 30, precedent
// `onePageSurface.test.ts`. These prove a component READS the right field and
// renders no literal in its place. They cannot prove the result is legible on
// screen; Roman's walk is what knows that.
//
// ## Why the ABSENCE assertions are the valuable half
//
// Three of the four changes REPLACE something: a default, a control, a data
// source. A number box that quietly came back, or a second list of card names
// added "just for the form", would break no build and fail nothing — it would
// simply be wrong again, in the exact way Chuck reported.

import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

const HERE = join(__dirname, "..");
const PAGES = join(__dirname, "..", "..", "..", "pages");
const SRC = join(__dirname, "..", "..", "..");
const read = (dir: string, file: string) => readFileSync(join(dir, file), "utf8");

/**
 * Source with its comments removed — BLOCK comments as well as line ones.
 *
 * ⚑ Required before any scan of this repository's source. This codebase
 * documents its rules next to its rules, so a scanner searching for a forbidden
 * word finds the DOCUMENTATION first — every comment in the files below
 * discusses the very things these assertions forbid.
 *
 * ## Why the block form too, where the older scanners strip only `//`
 *
 * Measured, not assumed: the card-name assertion below failed on its FIRST run
 * against a `{/* … *\/}` comment in `PracticeAddQuestion.tsx` that explains why
 * a person should not have to know card 3 is called "false premise". JSX
 * comments are block comments, and a JSX-heavy file documents itself in them
 * almost exclusively — so a scanner that reads only `//` reads most of a
 * component's prose as if it were code.
 */
const withoutComments = (source: string): string =>
  source
    // Block comments first: one of them may contain a `//`, and stripping lines
    // first would leave the block's opener dangling.
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .split("\n")
    .map((line) => (line.includes("//") ? line.slice(0, line.indexOf("//")) : line))
    .join("\n");

describe("the page opens on Chuck's side", () => {
  const page = () => withoutComments(read(PAGES, "PracticePage.tsx"));

  // Ruled 2026-09-15 from Marie's first week: she practises the direct far more
  // often than the cross, and the page opened on the defense's every time.
  it("initialises the side to chuck", () => {
    expect(page()).toContain('React.useState<"george" | "chuck">("chuck")');
    expect(page()).not.toContain('React.useState<"george" | "chuck">("george")');
  });

  // ⚑ THE MUTATION THIS EXISTS FOR: a second `side` state — one for the bar, one
  // for the tabs — would let the two disagree, and she would practise the side
  // she is not looking at. One state, one default, and the same value handed to
  // both surfaces.
  it("holds exactly one side state and hands it to both surfaces", () => {
    const source = page();
    const states = source.match(/useState<"george" \| "chuck">/g) ?? [];
    expect(states).toHaveLength(1);
    expect(source).toContain("side={side}");
    expect(source).toContain("onSide={setSide}");
  });
});

describe("the answer-analysis switch", () => {
  const bar = () => withoutComments(read(HERE, "PracticeStart.tsx"));

  it("draws the switch on the practice bar, in the store's words", () => {
    const source = bar();
    expect(source).toContain('role="switch"');
    expect(source).toContain('w("answer_analysis_label")');
    expect(source).toContain('w("answer_analysis_on")');
    expect(source).toContain('w("answer_analysis_off")');
  });

  // The state must be REPORTED, not assumed: `aria-checked` is what a screen
  // reader announces, and a switch that never sets it announces itself as off
  // for ever.
  it("reports its state to a screen reader", () => {
    expect(bar()).toContain("aria-checked={analysisOn}");
  });

  // ⚑ The page owns the state because the STORAGE WRITE is the page's to make.
  // A component that wrote storage itself would be a second place the value
  // could be set, and the two would drift the first time either was edited.
  it("is owned by the page, which is what remembers it", () => {
    const page = withoutComments(read(PAGES, "PracticePage.tsx"));
    expect(page).toContain("readAnalysis(analysisStore)");
    expect(page).toContain("writeAnalysis(analysisStore, on)");
    expect(bar()).not.toContain("localStorage");
  });

  // Both surfaces that can submit an answer must ask. The sitting screen is
  // retired from the interface but its route is still served — an answer
  // written there would otherwise be the one address where "off" was not true.
  it("reaches every screen that can submit an answer", () => {
    const question = withoutComments(read(PAGES, "PracticeQuestionPage.tsx"));
    const sitting = withoutComments(read(PAGES, "PracticeSessionPage.tsx"));

    expect(question).toContain("wantRead: analysisOn");
    expect(question).toContain("answerChrome(working ? \"working\" : \"idle\", analysisOn)");
    expect(sitting).toContain("wantRead: readAnalysis(browserStore())");
  });

  // ⚑ MUTATION: `wantRead: true` anywhere → the model is asked whatever the
  // switch says, and the only visible symptom is a bill. This walks every page
  // that submits rather than naming the two above, so a THIRD answer screen
  // cannot arrive with the flag hard-coded.
  it("never hard-codes a request for a read", () => {
    for (const file of readdirSync(PAGES).filter((f) => f.endsWith(".tsx"))) {
      const source = withoutComments(read(PAGES, file));
      if (!source.includes("submitPracticeAnswer(")) continue;
      expect(source, `${file} asks for a read unconditionally`).not.toContain(
        "wantRead: true",
      );
    }
  });
});

describe("the tactic dropdown", () => {
  const editForm = () => withoutComments(read(HERE, "PracticeRowEdit.tsx"));
  const addForm = () => withoutComments(read(HERE, "PracticeAddQuestion.tsx"));

  // Both forms, because "in BOTH" was the ruling: one form left on a number box
  // would be the same defect, reachable from a different button.
  it("offers the cards as options in both forms", () => {
    for (const source of [editForm(), addForm()]) {
      expect(source).toContain("tacticCards.map");
      expect(source).toContain("value={String(card.card)}");
      expect(source).toContain("{card.name}");
      expect(source).toContain('w("editor_tactic_none")');
    }
  });

  // ⚑ THE NUMBER BOX IS GONE. A `<select>` added beside a surviving `<input>`
  // would pass every assertion above and still let Chuck type `8`.
  it("leaves no number box behind in either form", () => {
    for (const source of [editForm(), addForm()]) {
      // The label and the `aria-label` bracket the control, so the slice between
      // the first and last mention of the tactic's key IS the control.
      const tacticBlock = source.slice(
        source.indexOf('w("editor_field_tactic")'),
        source.lastIndexOf('w("editor_field_tactic")'),
      );
      // ANTI-VACUITY: a slice that came back empty — the key mentioned once,
      // the control renamed — would satisfy every `not.toContain` below for
      // ever. Asserting what the block IS is what makes the absences mean
      // something.
      expect(tacticBlock).toContain("<select");
      expect(tacticBlock).not.toContain("<input");
    }
  });

  // ⚑ The defect this change fixes, pinned so it cannot return: the edit form
  // seeded its box from `question.tactic`, which is the RESOLVED NAME — and on
  // a braid row, the name plus a suffix. Any edit of it was a 400.
  it("selects by the card NUMBER, never by the resolved name", () => {
    const source = editForm();
    expect(source).toContain("question.tactic_card");
    expect(source).not.toContain("React.useState(question.tactic ?? \"\")");
  });

  // ⚑ ONE AUTHORITY. The seven names live in a settings row the server resolves
  // pills against, and they reach the browser on the payload. A list here would
  // be correct until somebody renamed a card, and then the tag and the form
  // would disagree while both rendered.
  it("holds no list of card names anywhere in the frontend", () => {
    const named: string[] = [];
    const walk = (dir: string) => {
      for (const entry of readdirSync(dir, { withFileTypes: true })) {
        const path = join(dir, entry.name);
        if (entry.isDirectory()) {
          walk(path);
          continue;
        }
        if (!entry.name.endsWith(".ts") && !entry.name.endsWith(".tsx")) continue;
        if (entry.name.endsWith(".test.ts")) continue;
        const source = withoutComments(readFileSync(path, "utf8"));
        // The two card names this case's deck actually uses. Either of them as a
        // literal in the bundle is a second vocabulary.
        if (source.includes('"false premise"') || source.includes('"half-truth"')) {
          named.push(path);
        }
      }
    };
    walk(SRC);
    expect(named).toEqual([]);
  });
});

describe("the receipt is editable", () => {
  const editForm = () => withoutComments(read(HERE, "PracticeRowEdit.tsx"));

  it("renders a receipt field in the same stack as the others", () => {
    const source = editForm();
    expect(source).toContain('w("editor_field_receipt")');
    expect(source).toContain("value={receipt}");
  });

  // ⚑ The field must be in the SAVE list, not merely on screen. A box that
  // takes typing and is never compared or sent is the most convincing possible
  // way to lose an edit: it looks exactly like a box that worked.
  it("sends the receipt as its own change, and clears it with a blank", () => {
    const source = editForm();
    expect(source).toContain('["receipt", receipt, question.receipt ?? ""]');
    // The shared loop is what turns a blank into `null`; this pins that the
    // receipt goes through it rather than around it.
    expect(source).toContain('editor.edit(question.id, field, next.trim() === "" ? null : next.trim())');
  });

  // The row still PRINTS the receipt — the edit form is a second surface for the
  // same field, not a replacement for the line under the question.
  it("leaves the row's own Built-from line alone", () => {
    expect(withoutComments(read(HERE, "PracticeDeckRow.tsx"))).toContain("question.receipt");
  });
});
