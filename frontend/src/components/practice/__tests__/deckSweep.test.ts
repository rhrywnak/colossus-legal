// =============================================================================
// deckSweep.test.ts — what "Done reviewing" is allowed to sweep
// =============================================================================
//
// CC_TASK_FOR_YOU_v1 L2. The property worth holding down here is a NEGATIVE:
// nothing the page did not show can reach the press. A negative asserted over a
// component is only as good as the clicks somebody remembered to simulate,
// which is why `shownItems` is a function and this is a test rather than a
// rendering.

import { describe, expect, it } from "vitest";

import { shownItems } from "../deckSweep";
import type { PracticeNote, PracticeQuestion } from "../../../services/practice";

function note(id: string, struck = false): PracticeNote {
  return {
    id,
    question_id: "q-1",
    answer_id: null,
    author: "Chuck",
    text: "look at this one again",
    when: "Tue 22 Sep",
    struck: struck ? "struck Tue 22 Sep" : null,
  } as PracticeNote;
}

function question(over: Partial<PracticeQuestion> = {}): PracticeQuestion {
  return {
    id: "q-1",
    side: "chuck",
    braid: false,
    text: "Whose money was the check?",
    tactic: null,
    tactic_card: null,
    receipt: null,
    braid_rows: null,
    watch_for: null,
    pair_said: null,
    pair_admitted: null,
    stronger: null,
    stronger_lean: null,
    kind: "direct",
    deck_key: null,
    follows_key: null,
    answered_on: null,
    hidden: false,
    draft_by: null,
    notes: [],
    ...over,
  } as PracticeQuestion;
}

describe("what a deck page showed", () => {
  it("names each row's CURRENT answer and every note standing on it", () => {
    const items = shownItems([
      question({ id: "q-1", answer_id: "a-1", notes: [note("n-1")] }),
      question({ id: "q-2", answer_id: "a-2", notes: [] }),
    ]);
    expect(items).toEqual([
      { kind: "answer", id: "a-1" },
      { kind: "note", id: "n-1" },
      { kind: "answer", id: "a-2" },
    ]);
  });

  it("names a question's notes even when nobody has answered it", () => {
    // A note can stand on a question with no answer — it is on screen, so it
    // is swept. There is simply no answer id to name.
    const items = shownItems([question({ answer_id: undefined, notes: [note("n-9")] })]);
    expect(items).toEqual([{ kind: "note", id: "n-9" }]);
  });

  it("names nothing at all for an unanswered question with no notes", () => {
    expect(shownItems([question()])).toEqual([]);
    expect(shownItems([])).toEqual([]);
  });

  it("includes a STRUCK note, because the page rendered it", () => {
    // The rule is what the page SHOWED, not what happened to be waiting. A
    // struck note is rendered (struck through, with its date) and waits for
    // nobody, so marking it read changes no count and keeps the rule simple.
    const items = shownItems([question({ answer_id: "a-1", notes: [note("n-2", true)] })]);
    expect(items).toContainEqual({ kind: "note", id: "n-2" });
  });

  it("NEVER names a deck change", () => {
    // ⚑ The negative that matters. A reworded question waits for the WITNESS,
    // and Done reviewing is the reviewers' act. A sweep that included changes
    // would let a reviewer clear work he was never shown — off her list, out of
    // her count, with nothing having told her.
    const items = shownItems([question({ answer_id: "a-1", notes: [note("n-1")] })]);
    expect(items.every((item) => item.kind !== "change")).toBe(true);
  });

  it("names only what it was given — a second page's rows cannot leak in", () => {
    // The composition is a pure function of the payload the page rendered
    // from, so "what the page showed" and "what the press sends" are the same
    // list by construction rather than by anybody remembering to keep them so.
    const first = shownItems([question({ id: "q-1", answer_id: "a-1" })]);
    expect(first).toEqual([{ kind: "answer", id: "a-1" }]);
  });
});
