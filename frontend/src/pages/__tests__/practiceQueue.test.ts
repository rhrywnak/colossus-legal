// Tests for `practiceQueue`.
//
// The queue is the one part of the drill whose failure a screenshot cannot show:
// a re-queued question that never returns, or an alternation that silently
// becomes five questions from one side.

import { describe, expect, it } from "vitest";

import type { PracticeQuestion } from "../../services/practice";
import {
  availableDeck,
  availableFor,
  buildQueue,
  orderedDeck,
  requeue,
  V0_QUESTION_COUNT,
} from "../practiceQueue";

function question(id: string, side: "george" | "chuck"): PracticeQuestion {
  return {
    id,
    side,
    braid: false,
    text: `question ${id}`,
    tactic: null,
    receipt: null,
    braid_rows: null,
    watch_for: null,
    pair_said: null,
    pair_admitted: null,
    stronger: null,
    stronger_lean: null,
    flag_note: null,
  };
}

/** The S-5 deck's shape: five George questions, then five of Chuck's. */
const DECK: PracticeQuestion[] = [
  question("g1", "george"),
  question("g2", "george"),
  question("g3", "george"),
  question("g4", "george"),
  question("g5", "george"),
  question("c1", "chuck"),
  question("c2", "chuck"),
  question("c3", "chuck"),
  question("c4", "chuck"),
  question("c5", "chuck"),
];

describe("buildQueue", () => {
  it("deals one side in deck order", () => {
    expect(buildQueue(DECK, "george").map((q) => q.id)).toEqual([
      "g1",
      "g2",
      "g3",
      "g4",
      "g5",
    ]);
    expect(buildQueue(DECK, "chuck").map((q) => q.id)).toEqual([
      "c1",
      "c2",
      "c3",
      "c4",
      "c5",
    ]);
  });

  it("alternates the mixed queue starting with George — the shape of a real day", () => {
    // Not randomised, deliberately: two sittings must be comparable, and the
    // point of mixing is that she changes register between a hostile question
    // and a friendly one.
    expect(buildQueue(DECK, "mixed").map((q) => q.id)).toEqual([
      "g1",
      "c1",
      "g2",
      "c2",
      "g3",
    ]);
  });

  it("deals a SHORT queue rather than padding a thin deck", () => {
    // Repeating a question to reach five would be the system inventing a rep.
    const thin = [question("g1", "george"), question("c1", "chuck")];
    expect(buildQueue(thin, "george")).toHaveLength(1);
    expect(buildQueue(thin, "mixed").map((q) => q.id)).toEqual(["g1", "c1"]);
  });

  it("deals nothing from an empty deck, for every side", () => {
    // The S-6 case. An empty queue is what makes the start screen withdraw its
    // Start control instead of opening a session with no questions in it.
    for (const who of ["george", "chuck", "mixed"] as const) {
      expect(buildQueue([], who)).toEqual([]);
      expect(availableFor([], who)).toBe(0);
    }
  });

  it("never deals more than the v0 count", () => {
    expect(V0_QUESTION_COUNT).toBe(5);
    for (const who of ["george", "chuck", "mixed"] as const) {
      expect(buildQueue(DECK, who).length).toBeLessThanOrEqual(V0_QUESTION_COUNT);
    }
  });
});

describe("requeue", () => {
  it("returns the question at the END, so it comes back as question 6", () => {
    // The task's own words. Immediately-after would be recall; at the end it is
    // a second attempt with something in between.
    const queue = buildQueue(DECK, "george");
    const again = requeue(queue, queue[0]);

    expect(again).toHaveLength(6);
    expect(again[5].id).toBe("g1");
    expect(again.slice(0, 5)).toEqual(queue);
  });

  it("does not mutate the queue it was given", () => {
    // React holds this array in state; mutating it in place would leave the
    // progress line showing a length nothing re-rendered.
    const queue = buildQueue(DECK, "george");
    requeue(queue, queue[0]);
    expect(queue).toHaveLength(5);
  });
});

// ── mockup v3: the start screen's list, and today's skips ────────────────────

describe("orderedDeck", () => {
  // The whole reason this function exists: the list Marie reads before Start and
  // the queue she is then dealt must be the same questions in the same order.
  // Two functions that "both interleave the same way" is how she skips row 3 and
  // is asked a different third question.
  it("is the queue's own order, uncut", () => {
    const deck = DECK;
    for (const who of ["george", "chuck", "mixed"] as const) {
      const ordered = orderedDeck(deck, who);
      expect(buildQueue(deck, who, ordered.length)).toEqual(ordered);
      expect(buildQueue(deck, who, 3)).toEqual(ordered.slice(0, 3));
    }
  });

  it("lists a whole side, not just the first five", () => {
    expect(orderedDeck(DECK, "mixed")).toHaveLength(10);
    expect(orderedDeck(DECK, "george")).toHaveLength(5);
  });
});

describe("availableDeck", () => {
  it("drops the questions she kept out today, and keeps the rest in order", () => {
    const deck = DECK;
    const ordered = orderedDeck(deck, "george");
    const skipped = new Set([ordered[2].id]);

    const left = availableDeck(deck, "george", skipped);

    expect(left).toHaveLength(4);
    expect(left.map((q) => q.id)).not.toContain(ordered[2].id);
    // Order is preserved — skipping the third does not reshuffle the rest.
    expect(left.map((q) => q.id)).toEqual(
      ordered.filter((q) => q.id !== ordered[2].id).map((q) => q.id),
    );
  });

  it("returns nothing when every question is kept out — Start has to withdraw", () => {
    const deck = DECK;
    const all = new Set(orderedDeck(deck, "chuck").map((q) => q.id));
    expect(availableDeck(deck, "chuck", all)).toHaveLength(0);
  });

  it("is unaffected by a skip on the side she is not looking at", () => {
    const deck = DECK;
    const chuckIds = new Set(orderedDeck(deck, "chuck").map((q) => q.id));
    expect(availableDeck(deck, "george", chuckIds)).toHaveLength(5);
  });
});
