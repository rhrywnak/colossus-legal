// =============================================================================
// practiceReviewSurface.test.ts — the Review answers page renders what it must
// =============================================================================
//
// CC_TASK_REVIEW_PAGE_v1. A SOURCE SCAN, in the shape `onePageSurface.test.ts`
// and `practicePolishSurface.test.ts` already use on this surface — there is no
// component-testing infrastructure in this project (CLAUDE.md rule 30), and the
// things asserted here are structural claims about the page rather than
// behaviour a pure helper could carry.
//
// ## What this file is for, and what it is not
//
// It cannot prove the page LOOKS right; the REPRODUCED/DEVIATED table in the
// report does that, measured in a browser. It proves the claims that would fail
// silently: that both Done reviewing controls are the ONE existing component
// rather than a second copy, that the page never compares a username itself,
// that no user-facing string is written in the source, and that the write path
// goes through the existing note routes rather than a new one.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

const PAGES = join(__dirname, "..");
const PRACTICE = join(__dirname, "..", "..", "components", "practice");

const read = (dir: string, name: string) => readFileSync(join(dir, name), "utf8");

/** Source with its `//` comments removed — this repo documents beside its code. */
function withoutComments(source: string): string {
  return source
    .split("\n")
    .map((line) => {
      const at = line.indexOf("//");
      return at === -1 ? line : line.slice(0, at);
    })
    .join("\n");
}

describe("the Review answers page", () => {
  const page = () => withoutComments(read(PAGES, "PracticeReviewPage.tsx"));

  it("reuses the ONE review bar for both Done reviewing controls", () => {
    // Two copies would drift, and the bottom one is the one Chuck actually
    // presses — he has just finished reading forty-two rows.
    const source = page();
    expect(source).toContain("PracticeReviewBar");
    expect(source.match(/<PracticeReviewBar/g) ?? []).toHaveLength(1);
    // Rendered twice from one element: once above the rows, once in the foot.
    expect(source.match(/\{done\}/g) ?? []).toHaveLength(2);
  });

  it("never decides for itself who may press Done reviewing", () => {
    // Rule 12: all action availability lives in the backend. The page passes
    // the server's `review` block through and compares no username.
    const source = page();
    expect(source).toContain("review={deck.review}");
    expect(source).not.toContain("can_mark_reviewed ===");
    expect(source).not.toContain("username");
  });

  it("makes exactly two reads and re-reads BOTH after a write", () => {
    const source = page();
    expect(source).toContain("fetchPracticeDeck");
    expect(source).toContain("fetchPracticeAnswers");
    // One `load` used by the mount AND by every write — not two paths that
    // could fall out of step, and never a local patch of a fetched array.
    expect(source).toContain(".then(load)");
    expect(source).not.toContain("setLoaded((");
  });

  it("writes through the review loop's existing routes", () => {
    // §1: no new business logic server-side. These three are the routes the
    // review loop already shipped.
    const source = page();
    for (const call of ["addAnswerNote", "addQuestionNote", "strikeNote"]) {
      expect(source, `${call} is not wired`).toContain(call);
    }
  });

  it("speaks only stored wording", () => {
    // Law 10. Every user-facing string on this page is a settings row; the
    // page holds no sentence of its own. The one literal it may carry is the
    // technical cause of a failed load, which is not a stored sentence and
    // cannot be — `wording` arrives inside the payload that failed to arrive.
    const source = page();
    for (const key of [
      "review_title",
      "review_load_failed",
      "review_empty_deck",
      "print_answers_label",
    ]) {
      expect(source, `${key} is not read`).toContain(`w("${key}")`);
    }
  });

  it("keeps the printed sheet one click away", () => {
    // Ruling STOP-G: the print page is not deleted, it moves here as a ghost
    // link. A page that dropped it would strand the paper Chuck still uses.
    const source = page();
    expect(source).toContain("practiceAnswersPath");
    expect(source).toContain('target="_blank"');
  });
});

describe("one card on the Review answers page", () => {
  const card = () => withoutComments(read(PRACTICE, "PracticeReviewCard.tsx"));

  it("says which thing a note will land on, from the store", () => {
    // The two boxes write to two different rows and look identical. The
    // placeholder is the only warning, and it is a stored row either way.
    const source = card();
    expect(source).toContain("review_note_placeholder_question");
    expect(source).toContain("review_note_placeholder_answer");
  });

  it("renders the unanswered block from the store, not from silence", () => {
    // An empty space where an answer would be reads as a row that failed to
    // load. The stored sentence says which state this is.
    expect(card()).toContain('w("review_unanswered")');
  });

  it("prints the answered line the SERVER composed", () => {
    // The browser holds no date format and no template: it renders the line.
    const source = card();
    expect(source).toContain("answered_meta");
    expect(source).not.toContain("toLocaleDateString");
  });

  it("keeps struck notes visible, by reusing the one note list", () => {
    // Roman's ruling of 2026-09-19, and the standing law of this table. A card
    // that filtered `struck === null` would make the review page disagree with
    // Marie's deck row about what has been said on a question.
    const source = card();
    expect(source).toContain("PracticeNoteList");
    expect(source).not.toContain("struck === null");
    expect(source).not.toContain("struck !== null");
  });

  it("reuses the deck row's side pills", () => {
    // A question that is green on one page and a different green on the next
    // reads as a different question.
    const source = card();
    for (const pill of ["pillGeorge", "pillChuck", "pillBraid"]) {
      expect(source, `${pill} is not used`).toContain(pill);
    }
  });
});
