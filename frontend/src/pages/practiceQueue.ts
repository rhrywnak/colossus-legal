// =============================================================================
// practiceQueue.ts — the drill's queue, as pure functions
// =============================================================================
//
// The session's only real logic: which questions are dealt, in what order, and
// what "ask me this one again later" does to the rest of the sitting.
//
// ## Why this is a module and not state inside the page
//
// It is the one part of the drill that can be WRONG in a way a screenshot cannot
// show — a re-queued question that never comes back, or one that comes back
// immediately and turns the drill into a loop. Pure functions with a test file
// beside them; the page holds the queue and calls these to change it.
//
// ## Why the queue lives in the browser at all
//
// The server records answers; it does not track a position. The task's re-queue
// is WITHIN one sitting ("it returns as question 6"), and cross-session repeat
// memory is explicitly out of scope for v0. A server-side cursor would be state
// nobody needs and a second thing to keep in step with the screen.

import type { PracticeQuestion } from "../services/practice";

/** How many questions v0 deals. The other two count pills render dimmed. */
export const V0_QUESTION_COUNT = 5;

/**
 * Build the queue for one sitting.
 *
 * ## Domain note: the mixed order is the mockup's, and it is not random
 *
 * The mockup interleaves George · Chuck · George · Chuck · George — the shape of
 * a real day, where a friendly question follows a hostile one and she has to
 * change register between them. Randomising would make two sittings
 * incomparable, which is the opposite of a drill.
 *
 * A side with fewer questions than the target yields a SHORTER queue rather than
 * a padded one: five questions is a target, not a promise, and repeating a
 * question to reach a number would be the system inventing a rep.
 */
export function buildQueue(
  deck: PracticeQuestion[],
  who: "george" | "chuck" | "mixed",
  limit: number = V0_QUESTION_COUNT,
): PracticeQuestion[] {
  return orderedDeck(deck, who).slice(0, limit);
}

/**
 * One side's questions, in the order the queue would deal them, UNCUT.
 *
 * ## Why the start screen's list and the queue share this function
 *
 * Mockup v3 lists the deck on the start card, and the list has to be the same
 * questions in the same order the sitting will deal — otherwise Marie reads one
 * order, skips "row 3", and is asked a different third question. Two functions
 * that "both interleave the same way" is exactly how that drifts. So there is
 * one ordering, here, and `buildQueue` is this plus a `slice`.
 */
export function orderedDeck(
  deck: PracticeQuestion[],
  who: "george" | "chuck" | "mixed",
): PracticeQuestion[] {
  const george = deck.filter((q) => q.side === "george");
  const chuck = deck.filter((q) => q.side === "chuck");

  if (who === "george") return george;
  if (who === "chuck") return chuck;

  // Mixed: alternate, starting with George, and stop when either side runs out.
  // The shape of a real day — a friendly question after a hostile one, so she
  // has to change register between them.
  const mixed: PracticeQuestion[] = [];
  for (let i = 0; ; i += 1) {
    const next = i % 2 === 0 ? george[Math.floor(i / 2)] : chuck[Math.floor(i / 2)];
    if (next === undefined) break;
    mixed.push(next);
  }
  return mixed;
}

/**
 * The questions a sitting could deal right now: this side's, minus the ones she
 * has kept out today.
 *
 * ## Domain note: "skipped today" is not stored on the question
 *
 * It is a fact about THIS sitting — she is not saying the question is wrong
 * (that is what Flag says), only that she does not want it this evening. So it
 * lives in the page's state and in the session's own `skipped_today`, and it is
 * gone tomorrow.
 */
export function availableDeck(
  deck: PracticeQuestion[],
  who: "george" | "chuck" | "mixed",
  skippedToday: ReadonlySet<string>,
): PracticeQuestion[] {
  return orderedDeck(deck, who).filter((q) => !skippedToday.has(q.id));
}

/**
 * Put a question back at the end of THIS sitting's queue.
 *
 * Returns a new array — the page holds the queue in React state, and mutating it
 * in place would leave the screen showing a length React never re-rendered.
 *
 * ## Why the end, and not two questions from now
 *
 * The point of the repeat is that she answers it again having done something
 * else in between. Immediately after would be recall; at the end it is a second
 * attempt. The task names the behaviour precisely — "it returns as question 6".
 */
export function requeue(
  queue: PracticeQuestion[],
  question: PracticeQuestion,
): PracticeQuestion[] {
  return [...queue, question];
}

/**
 * How many questions one side can actually deal.
 *
 * The start screen withdraws Start when this is zero, which is what stops a
 * session opening on a side the deck has nothing for — a sitting that would show
 * Chuck's sheet with no rows and read as a session that lost her work.
 */
export function availableFor(
  deck: PracticeQuestion[],
  who: "george" | "chuck" | "mixed",
): number {
  return buildQueue(deck, who).length;
}
