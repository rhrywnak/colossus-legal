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
 *
 * ## ⚑ WITH THE ANALYSIS OFF, THE WORKING STATE SAYS LESS — because less is true
 *
 * `analysis` is the practice bar's switch. When it is off nothing is being read:
 * the round trip is a SAVE, and three of the five claims above would be false.
 *
 *   · the label stays the idle one. `read_working_label` is "Reading your
 *     answer", and a button that says so while no model has been asked is the
 *     screen lying about the one thing this switch exists to control. Ruled
 *     2026-09-15: keep the idle label rather than seed a sixth wording row.
 *   · no critique block. There is nothing coming, and an empty bordered box
 *     says "something should be here" — it would invite her to wait for a read
 *     she switched off.
 *   · no *Stop waiting*. It abandons a read in flight; with no read in flight it
 *     is a control that does nothing to a thing that is not happening.
 *
 * The other two hold either way: the answer is still being written, so the
 * button must not be pressable twice and the box must not move under her.
 */
export function answerChrome(phase: AnswerPhase, analysis: boolean): AnswerChrome {
  if (phase === "working") {
    return {
      buttonDisabled: true,
      buttonLabelKey: analysis ? "read_working_label" : "answer_button",
      boxLocked: true,
      critiquePresent: analysis,
      stopOffered: analysis,
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

/**
 * The label over the answer box — or `null`, meaning the plain `answer_label`.
 *
 * ## Domain note: the box is pre-filled, so it must say it is ON FILE
 *
 * The page opens with her current saved answer already in the box. On v2.2.0
 * nothing said so, and after four presses of Answer with analysis off her
 * reading was "nothing happened" (DEV, 2026-09-21). So once an answer exists the
 * label reads `Your answer — saved {when}`, composed by the server.
 *
 * The press's label wins over the loaded one: it is newer, and it is what makes
 * the label move the instant a press returns without a refetch. A press that
 * brought no label (a skip) falls back to what loaded; nothing loaded and
 * nothing pressed is the plain label, unchanged.
 */
export function savedLabelFor(
  current: { saved_label: string } | null,
  pressed: { saved_label: string | null } | null,
): string | null {
  return pressed?.saved_label ?? current?.saved_label ?? null;
}

/**
 * The one confirmation line under the buttons, or `null` for none.
 *
 * Only the server's `saved_line`, which it sends only when the press asked for
 * NO read — with analysis on, the read is the confirmation. `null` before any
 * press, and cleared on the next press (the page resets its result then).
 */
export function savedLineFor(pressed: { saved_line: string | null } | null): string | null {
  return pressed?.saved_line ?? null;
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
