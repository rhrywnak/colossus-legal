// =============================================================================
// deckReviewConfirmModel — pressing Done reviewing must not mark a deck reviewed
// =============================================================================
//
// Until 2026-09-20 the review bar's `Done reviewing` button was wired straight
// to `markDeckReviewed`: one click and the SHARED cursor had moved for the whole
// reviewer bench, with nothing asked and nothing to undo. There is no undo — the
// mark is a timestamp and the answers it swept behind it are simply no longer
// waiting for anybody.
//
// That is the same hazard class as the bare `Scan again` link before
// 2026-09-11, and this is deliberately the same remedy: a PURE state machine
// beside the component rather than a `useState` inside it.
//
// ## Rust Learning: this is the reducer shape, in TypeScript
//
// Roman will recognise it from `PromoteOutcome` and the pipeline steps: a
// function from (state, action) to a new state plus a DESCRIBED effect, where
// the effect is returned as data and PERFORMED by the caller. The component does
// the writing; this decides whether writing is allowed, and a test can enumerate
// every action without a browser, a DOM or a click.
//
// That matters because the property worth asserting is a NEGATIVE — "no other
// control moves the mark" — and a negative over a component is only as good as
// the list of interactions somebody remembered to simulate. Over a closed union
// of actions it is exhaustive by construction.
//
// There is no RTL and no jsdom in this tree (CLAUDE.md rule 30), so a fired
// click is not available to assert this. This shape is what makes the assertion
// possible anyway, and it is stronger than the click would have been.

import { pickByCount } from "../../utils/countWording";

/** Where the confirmation is. */
export type DeckReviewConfirmState =
  | { phase: "idle" }
  /**
   * The question is on screen, holding the COUNT it was asked about.
   *
   * The count is carried rather than re-read at Yes for the reason
   * `scanConfirmModel` carries its model id: the question on screen said
   * "Mark all 12 answers…", and a re-read between the question and the answer
   * would let the human agree to one number and the application act on another.
   * A deck is live — Marie can be answering while Chuck reads — so this is not
   * a theoretical race.
   */
  | { phase: "confirming"; count: number };

/**
 * Everything a human can do to the review bar that the confirmation cares about.
 *
 * `dismiss` is Escape and `cancel` is the Cancel button. They are two actions
 * with one behaviour on purpose: they are two different things a person DID, and
 * collapsing them here would mean a later change to one silently changed the
 * other. `refresh` is in the union for the reason `scanConfirmModel`'s two
 * no-ops are — a control added to this bar later is a control somebody must
 * decide about, and an action that fell outside the machine would be an action
 * the mutation proof never covered.
 */
export type DeckReviewConfirmAction =
  | { type: "open" }
  | { type: "cancel" }
  | { type: "dismiss" }
  | { type: "confirm" }
  | { type: "refresh"; count: number };

/** The new state, and whether to send the write. */
export interface DeckReviewConfirmStep {
  state: DeckReviewConfirmState;
  /**
   * `true` on exactly one arm, and the caller's ONLY authority to write.
   *
   * A boolean is enough here where `scanConfirmModel` needed a model id: there
   * is one deck and the route is addressed by its scenario, so there is no
   * second parameter the confirmation could get wrong. What it still buys is
   * that the component cannot write without having been through this function.
   */
  send: boolean;
}

/** The starting state: nothing asked, nothing written. */
export const idleDeckReviewConfirm: DeckReviewConfirmState = { phase: "idle" };

/**
 * Advance the confirmation.
 *
 * @param state the current phase
 * @param action what the human just did
 * @param count  the bar's CURRENT count, used only when the question OPENS —
 *               from that moment the machine holds its own copy
 */
export function deckReviewConfirmStep(
  state: DeckReviewConfirmState,
  action: DeckReviewConfirmAction,
  count: number,
): DeckReviewConfirmStep {
  switch (action.type) {
    case "open":
      // Nothing waiting, nothing to ask. The bar hides itself at zero
      // (`review.awaiting === 0` returns null), so the button cannot be pressed
      // in that state today — and "cannot be" is the kind of claim that survives
      // exactly until the next refactor moves the guard.
      return count <= 0
        ? { state: { phase: "idle" }, send: false }
        : { state: { phase: "confirming", count }, send: false };

    case "cancel":
    case "dismiss":
      return { state: { phase: "idle" }, send: false };

    case "confirm":
      // The ONE arm that writes. Guarded on `confirming` rather than trusting
      // the caller: a Yes button can only exist while the question is open, and
      // that is a claim about today's JSX, not an invariant of the machine.
      return state.phase === "confirming"
        ? { state: { phase: "idle" }, send: true }
        : { state, send: false };

    case "refresh":
      // The page re-read while the question was open — a note arrived, or Marie
      // answered another one. The question SURVIVES, and it keeps the number it
      // asked about, because that is the number the human is part-way through
      // agreeing to. Closing the question here would discard an answer they had
      // already decided; silently re-aiming it at the new count would be worse.
      return { state, send: false };
  }
}

/**
 * The question to put on screen, or `null` when there is nothing to ask.
 *
 * ## Why the fill happens here and not in the bar
 *
 * Two stored sentences (singular and plural) and two placeholders is enough
 * arithmetic to be worth testing without rendering anything. Returning `null`
 * from `idle` rather than an empty string is the same rule the bar's
 * oldest-waiting clause follows: an absent fact is withheld, never emptied.
 */
export function deckReviewConfirmSentence(input: {
  state: DeckReviewConfirmState;
  one: string;
  many: string;
  code: string;
}): string | null {
  const { state, one, many, code } = input;
  if (state.phase !== "confirming") return null;
  // `pickByCount` rather than a local `=== 1`: countWording.ts is declared as
  // THE one place that decides between a stored singular and plural, and a
  // second copy of that decision is how the two drift the day the rule changes.
  // The count is the one the QUESTION holds, never a fresh read — see the
  // state's note above.
  const template = pickByCount(state.count, one, many);
  return template.replace("{count}", String(state.count)).replace("{code}", code);
}
