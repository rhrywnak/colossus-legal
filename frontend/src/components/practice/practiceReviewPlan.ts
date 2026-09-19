// =============================================================================
// practiceReviewPlan.ts — what the Review answers page renders, as data
// =============================================================================
//
// CC_TASK_REVIEW_PAGE_v1 §1. Pure functions: deck order in, rows out, and the
// one decision that matters — where a note written on a row will land.
//
// ## Why a separate module and not logic inside the page
//
// The frontend's test pattern is pure-helper tests plus service tests; there is
// no component-testing infrastructure in this project (CLAUDE.md rule 30). So
// anything worth asserting has to be reachable without rendering, and the two
// things worth asserting here are exactly the two a wrong page would get wrong
// silently: the ORDER Chuck reads in, and which row a note attaches to.

import type { PracticeQuestion } from "../../services/practice";
import type { PracticeAnswer } from "../../services/practiceAnswers";

/** One card: a question, and the answer that stands for it (or none). */
export type ReviewRow = {
  question: PracticeQuestion;
  /** `null` when nobody has answered — the muted block, and a question note. */
  answer: PracticeAnswer | null;
};

/**
 * The deck as the review page reads it: deck order, hidden rows dropped, each
 * question paired with its current answer.
 *
 * ## Domain note: DECK ORDER, not answered-first
 *
 * The order is the order the deck is in, which is the order Marie will be asked
 * in. Sorting answered rows to the top would be convenient and wrong: Chuck is
 * reading a cross-examination as it will happen, and a gap where an answer is
 * missing is information about the deck, not clutter to be tidied away.
 *
 * ## Why hidden questions are dropped
 *
 * A hidden question is deleted as far as every list is concerned — the
 * mechanism is a hide so that Marie's old answers keep pointing at something.
 * The deck editor still shows them, greyed; nothing else does, and a review
 * page that did would ask Chuck to read questions nobody will be asked.
 *
 * @param questions the deck payload's questions, in the order it served them
 * @param answers every current answer in the scenario, in any order
 */
export function planReview(
  questions: PracticeQuestion[],
  answers: PracticeAnswer[],
): ReviewRow[] {
  // Indexed once rather than scanned per question: a 42-row deck against 42
  // answers is 1,764 comparisons the naive way, on a page that re-reads after
  // every note. The Map is built once per render of the list.
  const byQuestion = new Map(answers.map((answer) => [answer.question_id, answer]));
  return questions
    .filter((question) => !question.hidden)
    .map((question) => ({
      question,
      // `?? null` and not `?? undefined`: "no answer" is a state this page
      // RENDERS — the muted block — so it must be a value the row carries, not
      // an absent field a reader could mistake for a row that failed to build.
      answer: byQuestion.get(question.id) ?? null,
    }));
}

/**
 * Where a note written on one row lands.
 *
 * With an answer it attaches to THAT ANSWER — what Chuck is reading, and what
 * counts toward Marie's pill until she answers again. Without one it attaches
 * to the QUESTION, and she reads it before she writes.
 *
 * Returned as a tagged value rather than as two nullable ids, so a caller
 * cannot pass both, pass neither, or pick the wrong one of two strings.
 */
export type NoteTarget =
  | { kind: "answer"; answerId: string }
  | { kind: "question"; questionId: string };

/** The target for one row — see [`NoteTarget`]. */
export function noteTarget(row: ReviewRow): NoteTarget {
  return row.answer === null
    ? { kind: "question", questionId: row.question.id }
    : { kind: "answer", answerId: row.answer.answer_id };
}
