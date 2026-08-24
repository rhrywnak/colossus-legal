// printAnswerPlan.ts — what goes on Chuck's answer sheets, and in what order.
//
// PURE, and a module of its own for the reason CLAUDE.md rule 30 gives: this
// project tests pure helpers and services, not components. A decision exported
// from a `.tsx` is a decision that will not be tested.
//
// ## Why it reuses the QUESTIONS plan rather than building its own
//
// §7: "the same sheets in the same order". Chuck reads the two documents side
// by side — he marks a question on one and looks up her answer on the other —
// and two orderings would make that a lookup instead of a glance. Building a
// second plan would also be a second place for the sheet split to be decided,
// and the two would drift the first time either changed.

import type { PracticeQuestion, PracticeWording } from "../../services/practice";
import type { PracticeAnswer } from "../../services/practiceAnswers";
import { wordingOf } from "../../services/practice";
import { planSheets, type PrintPlan } from "./printSheetPlan";

/** One question, with what Marie has said to it — or the fact that she has not. */
export type AnswerRow = {
  question: PracticeQuestion;
  /** Her current answer, or `null` when she has not written one. */
  answer: PracticeAnswer | null;
};

/** The answers document: the questions plan, with an answer beside each row. */
export type AnswerPlan = {
  plan: PrintPlan;
  /** Keyed by question id, so a sheet can look up its own rows. */
  answers: Map<string, PracticeAnswer>;
};

/**
 * Pair the deck's sheets with the answers that belong to them.
 *
 * ## Domain note: a question with no answer STILL PRINTS
 *
 * It carries the stored "not answered yet" line instead. Omitting it would make
 * the answers sheet disagree with the questions sheet about how many questions
 * the deck holds — and Chuck reads them side by side, so a sheet that silently
 * dropped three rows would have him looking for questions that are not missing.
 */
export function planAnswers(
  questions: PracticeQuestion[],
  code: string,
  title: string,
  wording: PracticeWording,
  answers: PracticeAnswer[],
): AnswerPlan {
  return {
    plan: planSheets(questions, code, title, wording),
    answers: new Map(answers.map((a) => [a.question_id, a])),
  };
}

/**
 * What one row prints under its question: her answer, or the stored absence.
 *
 * Returns the SENTENCE either way rather than `null`, because a blank space
 * under a question on paper reads as a printing fault. The absence is stated.
 */
export function answerLine(
  row: AnswerRow,
  wording: PracticeWording,
): { text: string; when: string | null } {
  if (row.answer === null) {
    return { text: wordingOf(wording, "print_answer_missing"), when: null };
  }
  return { text: row.answer.text, when: row.answer.answered_on };
}
