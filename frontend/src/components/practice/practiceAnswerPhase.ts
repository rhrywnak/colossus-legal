// practiceAnswerPhase.ts — what the question page shows while it is working.
//
// PURE, extracted so the working state can be asserted at all. Rendering is not
// testable in this project — no jsdom, no `@testing-library`, vitest in the node
// environment — so a claim that lives only inside a component is a claim nothing
// can check. Roman's defect #1 of 2026-08-20 was precisely a working state that
// did not exist, and "a state not tested is a state that will quietly stop
// existing" is not a risk worth carrying twice.
//
// So the three visible facts about that state are decided HERE, and the page
// spreads them onto its controls.

/** Where the answer path is. */
export type AnswerPhase = "idle" | "working";

/** Everything the screen changes while the read runs. */
export type AnswerChrome = {
  /** `true` → the button is disabled and cannot be pressed again. */
  buttonDisabled: boolean;
  /** The wording KEY the button shows. Relabelled while working. */
  buttonLabelKey: string;
  /** `true` → the answer box is read-only. */
  boxLocked: boolean;
  /** `true` → the critique block is on screen, empty, before anything returns. */
  critiquePresent: boolean;
  /** `true` → Stop waiting is offered. */
  stopOffered: boolean;
};

/**
 * The three claims of the working state, in one place.
 *
 * ## Domain note: `critiquePresent` is the one Roman reported
 *
 * The block is on screen from the moment she presses Answer, EMPTY. Before this
 * the page showed nothing until the read returned, so it looked inert while it
 * worked and she pressed again. A block that appears only on resolution is the
 * defect, not a lesser version of the fix.
 */
export function answerChrome(phase: AnswerPhase): AnswerChrome {
  if (phase === "working") {
    return {
      buttonDisabled: true,
      buttonLabelKey: "read_working_label",
      boxLocked: true,
      critiquePresent: true,
      stopOffered: true,
    };
  }
  return {
    buttonDisabled: false,
    buttonLabelKey: "answer_button",
    boxLocked: false,
    critiquePresent: false,
    stopOffered: false,
  };
}

/** How long before the waiting line changes to say her answer is safe anyway. */
export const LONG_WAIT_MS = 10_000;

/**
 * Which waiting line to show, given how long the read has been running.
 *
 * ## Domain note: the threshold is a boundary, not a suggestion
 *
 * At exactly `LONG_WAIT_MS` the line has changed. A `>` here rather than `>=`
 * would leave the two lines disagreeing about the instant they swap, which no
 * screen would ever show but a test would have to guess at.
 */
export function waitingLineKey(elapsedMs: number): string {
  return elapsedMs >= LONG_WAIT_MS ? "read_still_working" : "read_usually_quick";
}
