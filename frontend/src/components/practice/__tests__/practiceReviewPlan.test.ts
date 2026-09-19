// =============================================================================
// practiceReviewPlan.test.ts — what the Review answers page renders, proved
// =============================================================================
//
// CC_TASK_REVIEW_PAGE_v1. Two things a wrong page would get wrong SILENTLY, and
// this file is the only thing that would notice either: the ORDER Chuck reads
// in, and WHICH ROW a note attaches to.
//
// There is no component-testing infrastructure in this project (CLAUDE.md rule
// 30), which is why the page's decisions live in a pure module and are asserted
// here rather than through a render.

import { describe, expect, it } from "vitest";

import { noteTarget, planReview } from "../practiceReviewPlan";
import type { PracticeQuestion, PracticeNote } from "../../../services/practice";
import type { PracticeAnswer } from "../../../services/practiceAnswers";

const q = (
  id: string,
  extra: Partial<PracticeQuestion> = {},
): PracticeQuestion =>
  ({
    id,
    side: "george",
    text: `question ${id}`,
    tactic: null,
    braid: false,
    kind: "cross",
    hidden: false,
    notes: [] as PracticeNote[],
    ...extra,
  }) as unknown as PracticeQuestion;

const a = (questionId: string): PracticeAnswer => ({
  question_id: questionId,
  answer_id: `answer-${questionId}`,
  text: `answer to ${questionId}`,
  answered_on: "Answered on 14 Sep",
  answered_meta: "Answered 14 Sep · Marie",
});

describe("the review page's row plan", () => {
  it("keeps DECK ORDER, not answered-first", () => {
    // The order is the order she will be asked in. Sorting answered rows to the
    // top would be convenient and would stop the page being a reading of the
    // cross-examination as it will happen.
    const rows = planReview([q("1"), q("2"), q("3")], [a("3")]);
    expect(rows.map((row) => row.question.id)).toEqual(["1", "2", "3"]);
  });

  it("pairs each question with ITS OWN answer, not a neighbour's", () => {
    const rows = planReview([q("1"), q("2")], [a("2"), a("1")]);
    expect(rows[0].answer?.answer_id).toBe("answer-1");
    expect(rows[1].answer?.answer_id).toBe("answer-2");
  });

  it("gives an unanswered question a NULL answer, not an absent one", () => {
    // `null` and not `undefined`: "not answered yet" is a state the page
    // RENDERS — the muted block — so the row must carry it as a value.
    const rows = planReview([q("1")], []);
    expect(rows).toHaveLength(1);
    expect(rows[0].answer).toBeNull();
  });

  it("drops hidden questions", () => {
    // A hidden question is deleted as far as every list is concerned; the
    // mechanism is a hide only so her old answers keep pointing at something.
    const rows = planReview([q("1"), q("2", { hidden: true }), q("3")], []);
    expect(rows.map((row) => row.question.id)).toEqual(["1", "3"]);
  });

  it("drops a hidden question even when it HAS an answer", () => {
    // The anti-vacuity half of the test above: an implementation that filtered
    // on "has no answer" rather than on `hidden` would pass that one.
    const rows = planReview([q("1", { hidden: true })], [a("1")]);
    expect(rows).toEqual([]);
  });

  it("carries an answer for a question that is not in the deck nowhere", () => {
    // An answer whose question has been hidden or removed must not invent a
    // row: the page shows the DECK, and a question nobody will be asked is not
    // part of it.
    const rows = planReview([q("1")], [a("1"), a("99")]);
    expect(rows).toHaveLength(1);
    expect(rows[0].question.id).toBe("1");
  });

  it("plans an empty deck as no rows, not as a failure", () => {
    expect(planReview([], [])).toEqual([]);
  });
});

describe("where a note written on a row lands", () => {
  it("attaches to the ANSWER when one stands", () => {
    // What Chuck is reading — and a note there counts toward Marie's pill
    // until she answers again.
    const [row] = planReview([q("1")], [a("1")]);
    expect(noteTarget(row)).toEqual({ kind: "answer", answerId: "answer-1" });
  });

  it("attaches to the QUESTION when nothing has been answered", () => {
    // She sees it before she writes. A different row, a different reader, and
    // nothing on screen distinguishes the two boxes but their wording.
    const [row] = planReview([q("1")], []);
    expect(noteTarget(row)).toEqual({ kind: "question", questionId: "1" });
  });

  it("never returns an answer id for a row with no answer", () => {
    // The tagged shape is what makes this impossible to get wrong at the call
    // site; this asserts the shape itself, so a future `{answerId, questionId}`
    // pair carrying both cannot slip in unnoticed.
    const [row] = planReview([q("1")], []);
    expect(noteTarget(row)).not.toHaveProperty("answerId");
  });
});
