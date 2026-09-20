// =============================================================================
// deckReviewConfirmModel.test.ts — pressing Done reviewing must not mark a deck
// =============================================================================
//
// The mutation proof. Until 2026-09-20 the review bar's button was wired
// straight to `markDeckReviewed`: one click, and the SHARED cursor had moved for
// the whole reviewer bench with nothing asked. There is no undo — the mark is a
// timestamp, and every answer behind it stops waiting for anybody.
//
// ## Why this is a reducer test and not a click test
//
// There is no RTL and no jsdom in this tree (CLAUDE.md rule 30), so a click
// cannot be fired. That constraint produced a better proof than the click would
// have been: the property worth asserting is a NEGATIVE — "nothing except Yes
// writes" — and over a component that is only as good as the list of
// interactions somebody remembered to simulate. Over a closed union of actions
// it is exhaustive, and a control added to the bar later that forgets to go
// through the machine is a control this file's first test will not cover —
// which is why `PracticeReviewBar` routes even the no-op `refresh` through it.

import { describe, expect, it } from "vitest";

import {
  deckReviewConfirmSentence,
  deckReviewConfirmStep,
  idleDeckReviewConfirm,
} from "../deckReviewConfirmModel";
import type {
  DeckReviewConfirmAction,
  DeckReviewConfirmState,
} from "../deckReviewConfirmModel";

const COUNT = 12;

/** Every action a human (or the page's own re-read) can take on this bar. */
const EVERY_ACTION: DeckReviewConfirmAction[] = [
  { type: "open" },
  { type: "cancel" },
  { type: "dismiss" },
  { type: "confirm" },
  { type: "refresh", count: 3 },
];

const confirming: DeckReviewConfirmState = { phase: "confirming", count: COUNT };

const ONE = "Mark the {count} answer in {code} as reviewed?";
const MANY = "Mark all {count} answers in {code} as reviewed?";

describe("no control but Yes moves the mark", () => {
  it("writes nothing from idle, whatever is done", () => {
    // Including `confirm` itself: a Yes that somehow fired with no question open
    // must still not write. The button cannot exist in that state today, and
    // "cannot exist" is the kind of claim that survives until the next refactor
    // moves the button.
    for (const action of EVERY_ACTION) {
      expect(
        deckReviewConfirmStep(idleDeckReviewConfirm, action, COUNT).send,
        `${action.type} moved the mark from idle`,
      ).toBe(false);
    }
  });

  it("writes nothing from an OPEN question except on Yes", () => {
    for (const action of EVERY_ACTION) {
      const { send } = deckReviewConfirmStep(confirming, action, COUNT);
      if (action.type === "confirm") {
        expect(send).toBe(true);
      } else {
        expect(send, `${action.type} moved the mark`).toBe(false);
      }
    }
  });

  it("opens the question when Done reviewing is pressed, and only opens it", () => {
    const step = deckReviewConfirmStep(idleDeckReviewConfirm, { type: "open" }, COUNT);
    expect(step.state).toEqual({ phase: "confirming", count: COUNT });
    expect(step.send).toBe(false);
  });

  it("asks nothing when nothing waits", () => {
    // The bar hides itself at zero, so this arm is a second guard — and the one
    // that would still hold if the hiding rule moved.
    for (const count of [0, -1]) {
      const step = deckReviewConfirmStep(idleDeckReviewConfirm, { type: "open" }, count);
      expect(step.state).toEqual({ phase: "idle" });
      expect(step.send).toBe(false);
    }
  });
});

describe("the retreats", () => {
  it("Cancel and Escape both close it, and both send nothing", () => {
    // Two actions, one behaviour, asserted separately: they are two different
    // things a person DID, and a later change to one must not silently change
    // the other.
    for (const type of ["cancel", "dismiss"] as const) {
      const step = deckReviewConfirmStep(confirming, { type }, COUNT);
      expect(step.state).toEqual({ phase: "idle" });
      expect(step.send, `${type} sent the write`).toBe(false);
    }
  });
});

describe("the question keeps the number it asked about", () => {
  it("survives the page re-reading, holding its original count", () => {
    // A note arrives while Chuck is reading the question. Closing it would
    // discard a decision he is part-way through; silently re-aiming it at the
    // new number would be worse — he would agree to a sentence he never read.
    const step = deckReviewConfirmStep(confirming, { type: "refresh", count: 99 }, 99);
    expect(step.state).toEqual({ phase: "confirming", count: COUNT });
    expect(step.send).toBe(false);
  });

  it("a re-read does not open a question nobody asked for", () => {
    const step = deckReviewConfirmStep(idleDeckReviewConfirm, { type: "refresh", count: 5 }, 5);
    expect(step.state).toEqual({ phase: "idle" });
  });
});

describe("the sentence", () => {
  it("is withheld entirely while idle", () => {
    // Not an empty string: an absent question must render nothing at all, the
    // same rule the bar's oldest-waiting clause follows.
    expect(
      deckReviewConfirmSentence({
        state: idleDeckReviewConfirm,
        one: ONE,
        many: MANY,
        code: "S-13",
      }),
    ).toBeNull();
  });

  it("takes the plural above 1 and fills both slots", () => {
    const sentence = deckReviewConfirmSentence({
      state: { phase: "confirming", count: 12 },
      one: ONE,
      many: MANY,
      code: "S-13",
    });
    expect(sentence).toBe("Mark all 12 answers in S-13 as reviewed?");
    expect(sentence).not.toContain("{");
  });

  it("takes the singular at exactly 1 — '1 answers' does not ship", () => {
    expect(
      deckReviewConfirmSentence({
        state: { phase: "confirming", count: 1 },
        one: ONE,
        many: MANY,
        code: "S-7",
      }),
    ).toBe("Mark the 1 answer in S-7 as reviewed?");
  });

  it("names the count the QUESTION holds, not one passed in later", () => {
    // The anti-race assertion, stated on the sentence as well as the state: what
    // is on screen is what Yes acts on.
    const open = deckReviewConfirmStep(idleDeckReviewConfirm, { type: "open" }, 4).state;
    const after = deckReviewConfirmStep(open, { type: "refresh", count: 40 }, 40).state;
    expect(
      deckReviewConfirmSentence({ state: after, one: ONE, many: MANY, code: "S-1" }),
    ).toContain("4 answers");
  });
});
