// =============================================================================
// onePageSurface.test.ts — what the list page carries, and what it must not
// =============================================================================
//
// §10's list, for the list half of L2: a row renders its question text, its date
// when answered and nothing when not, and NO code, number or mark · Delete works
// on an answered question and on an unanswered one · the printed sheet has no
// footer and no codes.
//
// ## Why source scans (the standing limit, restated)
//
// No jsdom, no `@testing-library/*` — CLAUDE.md rule 30, precedent
// `rehearsalPageStructure.test.ts`. These prove a component READS the right
// field and renders no literal in its place. They cannot prove the result is
// legible on screen; Roman's walk is what knows that.
//
// ## Why the ABSENCE assertions are the valuable half
//
// Everything this task did was removal, and a removal has no natural test. A
// row that quietly regained its sequential number would break no build and fail
// nothing — it would simply be wrong again, in the exact way Chuck reported.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

const HERE = join(__dirname, "..");
const PAGES = join(__dirname, "..", "..", "..", "pages");
const read = (dir: string, file: string) => readFileSync(join(dir, file), "utf8");

/** A component's JSX opening tags — where a thing is actually rendered. */
const tagsOf = (source: string): string[] => source.match(/<[a-zA-Z][^>]*>/g) ?? [];

describe("a deck row carries five things and no sixth", () => {
  const row = () => read(HERE, "PracticeDeckRow.tsx");

  it("renders the question text, the receipt and the one status line", () => {
    const source = row();
    expect(source).toContain("question.text");
    expect(source).toContain("question.receipt");
    expect(source).toContain("question.answered_on");
  });

  it("withholds the status line entirely when nobody has answered", () => {
    // NOT an empty string. An empty line under a question reads as a status
    // that failed to load, which is a different fact from "not answered yet".
    expect(row()).toMatch(/question\.answered_on !== null &&/);
  });

  it("renders no question code and no positional number", () => {
    const tags = tagsOf(row()).join("\n");
    expect(tags).not.toMatch(/deck_key/);
    expect(tags).not.toMatch(/\{number\}/);
    // And the prop is gone from the interface, so a number is unreachable
    // rather than merely unrendered.
    const source = row();
    const props = source.slice(source.indexOf("interface Props"), source.indexOf("const PracticeDeckRow"));
    expect(props).toMatch(/question: PracticeQuestion/); // anti-vacuity
    expect(props).not.toMatch(/^\s*number\??:/m);
  });

  it("renders none of the retired marks", () => {
    const source = row();
    for (const gone of ["status_mark", "question.changed", "row_review_link", "badge_draft"]) {
      expect(source, `${gone} is back on the row`).not.toContain(gone);
    }
  });

  it("offers Delete outside edit mode, not inside it", () => {
    // Roman: "do not force users into more steps than is required." Structural,
    // not a comment scan: take the row's control block, cut out everything the
    // `editor.editing` guard owns, and Delete must SURVIVE the cut. If it were
    // nested in that branch it would disappear with it.
    const source = row();
    const controls = source.slice(source.indexOf("<div style={d.rowControls}>"));

    const guardAt = controls.indexOf("{editor.editing && (");
    expect(guardAt, "the editor-only branch must exist, or this proves nothing").toBeGreaterThan(-1);
    const guardEnd = controls.indexOf(")}", guardAt) + 2;
    const outsideTheGuard = controls.slice(0, guardAt) + controls.slice(guardEnd);

    expect(outsideTheGuard).toContain("row_delete_label");
    // Anti-vacuity: the thing that IS edit-only did not survive the same cut.
    expect(outsideTheGuard).not.toContain("editor_edit_label");
  });

  it("has no Hide of its own — Delete is that mechanism", () => {
    expect(row()).not.toContain("editor_hide_label");
    expect(row()).not.toContain("editor_unhide_label");
  });
});

describe("Delete is a hide, and the undo is its only way back", () => {
  it("the page calls hideQuestion in both directions", () => {
    const page = read(PAGES, "PracticePage.tsx");
    expect(page).toContain("hideQuestion(question.id, hidden)");
    expect(page).toMatch(/const remove = .*setHidden\(question, true\)/);
    expect(page).toMatch(/const putBack = .*setHidden\(question, false\)/);
  });

  it("says which question failed, and leaves the row where it is", () => {
    // Standing Rule 1. A row that vanished on a failed write would tell Chuck
    // the deck had changed when it had not.
    const page = read(PAGES, "PracticePage.tsx");
    expect(page).toContain("setDeleteError");
    expect(page).toMatch(/question\.text\.slice/);
  });

  it("the undo line stands where the row stood, and is not a second state", () => {
    const list = read(HERE, "PracticeDeckList.tsx");
    expect(list).toContain("row_deleted_notice");
    expect(list).toContain("row_undo_label");
    // Held in local state only — there is no restore path beyond this one.
    expect(list).toMatch(/useState<PracticeQuestion\[\]>\(\[\]\)/);
  });

  it("there is no confirm dialog on the delete path", () => {
    // The undo REPLACES a dialog. A confirm that crept back would cost a step
    // every time to guard against the rare case.
    const list = read(HERE, "PracticeDeckList.tsx");
    const at = list.indexOf("onDelete(question)");
    expect(at).toBeGreaterThan(-1);
    expect(list.slice(at - 300, at + 300)).not.toContain("window.confirm");
  });
});

describe("the list page after the cuts", () => {
  it("has no fold — the list IS the page now", () => {
    const list = read(HERE, "PracticeDeckList.tsx");
    expect(list).not.toContain("deck_hide_link");
    expect(list).not.toContain("deck_show_link");
  });

  it("explains why a row can carry no date", () => {
    expect(read(HERE, "PracticeDeckList.tsx")).toContain("deck_status_footnote");
  });

  it("carries none of the retired sitting apparatus", () => {
    const start = read(HERE, "PracticeStart.tsx");
    for (const gone of ["who_heading", "how_many_heading", "start_label", "PracticeResume", "PracticeNotes"]) {
      expect(start, `${gone} survived the cuts`).not.toContain(gone);
    }
  });

  it("keeps the warning and the one line about how to testify", () => {
    const start = read(HERE, "PracticeStart.tsx");
    expect(start).toContain('w("intro")');
    expect(start).toContain("AlwaysCard");
  });

  it("puts all three controls in the title row", () => {
    const titleRow = read(HERE, "PracticeTitleRow.tsx");
    expect(titleRow).toContain("print_questions_label");
    expect(titleRow).toContain("print_answers_label");
    expect(titleRow).toContain("editor_switch_label");
  });
});

describe("the printed sheet", () => {
  const sheets = () => read(HERE, "PrintSheets.tsx");

  it("has no footer, so no sheet can end on a page carrying only one", () => {
    // The .405 defect. `break-before: avoid` was measured NOT to prevent it —
    // a forced footer-only page rendered byte-identical with and without the
    // rule — so the footer is deleted instead of asked to behave.
    expect(sheets()).not.toContain("print_footer_template");
    expect(sheets()).not.toContain("print_sheet_number_template");
    expect(read(HERE, "printStyles.ts")).not.toMatch(/breakBefore/);
  });

  it("prints no question code and no draft badge", () => {
    const tags = tagsOf(sheets()).join("\n");
    expect(tags).not.toMatch(/deck_key/);
    expect(sheets()).not.toContain("badge_draft");
  });

  it("the answers sheet draws no ruled lines — Chuck is reading, not marking", () => {
    const answers = read(HERE, "PrintAnswers.tsx");
    expect(answers).not.toContain("<Lines");
    expect(answers).not.toContain("p.line");
  });
});
