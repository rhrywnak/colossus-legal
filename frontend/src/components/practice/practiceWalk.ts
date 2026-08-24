// practiceWalk.ts — which questions a practice walk offers, and where it is.
//
// PURE. Every decision the walk makes is here, and the component below draws
// what it is told — CLAUDE.md rule 30's shape, and the only way this project can
// test a screen at all.
//
// ## ⚑ THE WALK WRITES NOTHING, AND THAT IS A RULE, NOT AN OBSERVATION
//
// No model call. No database write. No session. Nothing is recorded about what
// she practised, how long she took, or whether she looked. It is a witness
// saying her answers out loud to herself with the answer hidden until she asks.
//
// The test for that is NOT "the screen looks unchanged" — it is that NO REQUEST
// WAS MADE, asserted by spying the network layer across the whole loop. "Nothing
// visible happened" and "nothing happened" are different claims, and only the
// second one is the ruling.

import type { PracticeAnswer } from "../../services/practiceAnswers";
import type { PracticeQuestion } from "../../services/practice";

/** Which side the walk is dealing. The stored values, not the human names. */
export type WalkSide = "george" | "chuck";

/** One step of the walk: a question and what she wrote for it. */
export type WalkStep = {
  question: PracticeQuestion;
  answer: PracticeAnswer;
};

/**
 * The questions this walk will offer, in deck order.
 *
 * ## Domain note: ANSWERED questions only
 *
 * Practising an answer she has not written is nothing to practise — there is
 * nothing to reveal and nothing to check herself against. A walk that offered
 * unanswered questions would deal her a blank screen and call it practice.
 *
 * ## Domain note: DECK order, not answered order
 *
 * The order the sitting would deal them at trial, which is the order she needs
 * to get used to. Sorting by when she answered would rehearse her in the order
 * she happened to write, which is an artefact of her afternoon.
 */
export function walkSteps(
  questions: PracticeQuestion[],
  answers: PracticeAnswer[],
  side: WalkSide,
): WalkStep[] {
  const byQuestion = new Map(answers.map((answer) => [answer.question_id, answer]));
  const steps: WalkStep[] = [];
  for (const question of questions) {
    if (question.side !== side) continue;
    const answer = byQuestion.get(question.id);
    // `undefined` is the unanswered case and it is SKIPPED, not rendered empty.
    if (answer === undefined) continue;
    steps.push({ question, answer });
  }
  return steps;
}

/** Where the walk is: asking, revealed, or finished. */
export type WalkState =
  | { kind: "asking"; step: WalkStep; at: number; total: number }
  | { kind: "revealed"; step: WalkStep; at: number; total: number }
  | { kind: "end"; total: number }
  /** The chosen side has nothing answered. Says so; does not show a blank walk. */
  | { kind: "nothing" };

/**
 * What the walk shows at position `at`, with the answer hidden or shown.
 *
 * ## Domain note: SKIPPING IS PRESSING NEXT
 *
 * There is no skip control and no skipped state, because there is nothing to
 * explain: moving on without revealing IS skipping, and a control saying so
 * would be a second name for a button she already has. Nothing records that she
 * did it, because nothing records anything here.
 */
export function walkAt(steps: WalkStep[], at: number, revealed: boolean): WalkState {
  if (steps.length === 0) return { kind: "nothing" };
  if (at >= steps.length) return { kind: "end", total: steps.length };
  const step = steps[at];
  return {
    kind: revealed ? "revealed" : "asking",
    step,
    at,
    total: steps.length,
  };
}
