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
  editorDeck,
  orderedDeck,
  requeue,
  V0_QUESTION_COUNT,
} from "../practiceQueue";

function question(
  id: string,
  side: "george" | "chuck",
  kind: "cross" | "direct" | "redirect" = side === "george" ? "cross" : "direct",
  followsKey: string | null = null,
): PracticeQuestion {
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
    kind,
    // The key IS the id in these fixtures, which keeps the pairing assertions
    // readable: `r1` follows `g1` and the reader can see it.
    deck_key: id,
    follows_key: followsKey,
    hidden: false,
    answered_on: null,
    draft_by: null,
  };
}

/** One redirect, on Chuck's side, answering the George question it names. */
function redirect(id: string, follows: string): PracticeQuestion {
  return question(id, "chuck", "redirect", follows);
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

/** A v1 deck: three traps, each with its redirect, plus two of Chuck's direct. */
const PAIRED: PracticeQuestion[] = [
  question("g1", "george"),
  question("g2", "george"),
  question("g3", "george"),
  question("c1", "chuck"),
  question("c2", "chuck"),
  redirect("r1", "g1"),
  redirect("r2", "g2"),
  redirect("r3", "g3"),
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

  it("deals the mixed queue as PAIRS: a trap, then its redirect", () => {
    // v1's ruling. Not randomised, deliberately — two sittings must be
    // comparable — and not the v0 alternation either: a redirect is the answer
    // to the question just asked, and dealing it three questions later drills
    // something that never happens in a courtroom.
    //
    // 3 = three questions PUT TO HER. PAIRED has exactly three traps, so the
    // three redirects ride along and the two Chuck directs are not reached.
    expect(buildQueue(PAIRED, "mixed", 3).map((q) => q.id)).toEqual([
      "g1",
      "r1",
      "g2",
      "r2",
      "g3",
      "r3",
    ]);
  });

  // ── hotfix §3.5 · what a count means on MIXED ─────────────────────────────

  it("counts the questions asked OF HER, not the redirects that ride along", () => {
    // The defect this replaces: `slice(0, 5)` returned g1 · r1 · g2 · r2 · g3 —
    // a trap left standing, with the redirect that repairs it never dealt. In a
    // drill about recovering from a trap, that is the one place it must not end.
    const five = buildQueue(PAIRED, "mixed", 5).map((q) => q.id);
    expect(five).toEqual(["g1", "r1", "g2", "r2", "g3", "r3", "c1", "c2"]);
    // Five asked of her; the three redirects are Chuck repairing, not asking.
    expect(five.filter((id) => !id.startsWith("r"))).toHaveLength(5);
  });

  it("never ends between a trap and its redirect", () => {
    // Every count from 1 to the whole deck: the last question dealt is never a
    // trap whose redirect exists and was left out. Asserted over the RANGE and
    // not at one number, because the off-by-one this guards is exactly the kind
    // that hides at 4 and shows at 5.
    for (let n = 1; n <= 6; n += 1) {
      const dealt = buildQueue(PAIRED, "mixed", n).map((q) => q.id);
      const last = dealt[dealt.length - 1];
      const orphaned = last.startsWith("g") && !dealt.includes(`r${last.slice(1)}`);
      expect(orphaned, `count ${n} ended on ${last} with its redirect undealt`).toBe(
        false,
      );
    }
  });

  it("fills with Chuck's directs when George's side runs out first", () => {
    // Nine asked of her; PAIRED has only three traps and two directs, so the
    // queue is SHORTER than the count rather than padded. A repeated question
    // would be the system inventing a rep.
    expect(buildQueue(PAIRED, "mixed", 9).map((q) => q.id)).toEqual([
      "g1",
      "r1",
      "g2",
      "r2",
      "g3",
      "r3",
      "c1",
      "c2",
    ]);
  });

  it("deals Chuck's direct questions AFTER every pair", () => {
    expect(orderedDeck(PAIRED, "mixed").map((q) => q.id)).toEqual([
      "g1",
      "r1",
      "g2",
      "r2",
      "g3",
      "r3",
      "c1",
      "c2",
    ]);
  });

  it("deals a redirect exactly once, even though it is on Chuck's side too", () => {
    // The pairs are built from George's list and the tail from Chuck's, and a
    // redirect belongs to both. Without the dedup she would be asked the same
    // redirect twice in one sitting — which reads as a bug in the deck.
    const ids = orderedDeck(PAIRED, "mixed").map((q) => q.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("still deals a redirect whose George question is not in the deck", () => {
    // A `follows` pointing at a key nobody kept. Leaving it out entirely would
    // silently drop a question from the drill; it falls to the tail instead,
    // which is visible.
    const orphaned = [question("g1", "george"), redirect("r9", "g-gone")];
    expect(orderedDeck(orphaned, "mixed").map((q) => q.id)).toEqual(["g1", "r9"]);
  });

  it("keeps the mixed queue alternating when the deck has no redirects at all", () => {
    // Every deck seeded before 2026-08-19. George's five, then Chuck's five —
    // the pairs are empty and the tail is the whole of Chuck's side.
    expect(orderedDeck(DECK, "mixed").map((q) => q.id)).toEqual([
      "g1",
      "g2",
      "g3",
      "g4",
      "g5",
      "c1",
      "c2",
      "c3",
      "c4",
      "c5",
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

  it("puts a redirect on Chuck's side and NOT on George's", () => {
    // A redirect wears Chuck's pill because Chuck asks it. George's filter is
    // every CROSS question, which is what "George's side" has always meant —
    // stated as the kind now because that is the fact it is about.
    expect(orderedDeck(PAIRED, "george").map((q) => q.id)).toEqual(["g1", "g2", "g3"]);
    expect(orderedDeck(PAIRED, "chuck").map((q) => q.id)).toEqual([
      "c1",
      "c2",
      "r1",
      "r2",
      "r3",
    ]);
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

// ── Part B: a hidden question leaves every list Marie sees ───────────────────

/** The same question, hidden by the deck editor. */
function hidden(question: PracticeQuestion): PracticeQuestion {
  return { ...question, hidden: true };
}

describe("hidden questions", () => {
  it("are dropped from the list, the queue and the count", () => {
    // Task B1: a hidden question vanishes from Marie's list and from queues. It
    // is filtered in `orderedDeck` because that is the ONE ordering the whole
    // drill shares — a filter on the queue but not the list is how she reads
    // five questions and is asked four.
    const deck = [question("g1", "george"), hidden(question("g2", "george")), question("g3", "george")];

    expect(orderedDeck(deck, "george").map((q) => q.id)).toEqual(["g1", "g3"]);
    expect(buildQueue(deck, "george").map((q) => q.id)).toEqual(["g1", "g3"]);
    expect(availableFor(deck, "george")).toBe(2);
    expect(availableDeck(deck, "george", new Set()).map((q) => q.id)).toEqual(["g1", "g3"]);
  });

  it("are dropped from the mixed pairs too, on either side of a pair", () => {
    const deck = [
      question("g1", "george"),
      question("g2", "george"),
      redirect("r1", "g1"),
      hidden(redirect("r2", "g2")),
    ];
    expect(orderedDeck(deck, "mixed").map((q) => q.id)).toEqual(["g1", "r1", "g2"]);
  });

  it("a hidden TRAP takes its pair out with it", () => {
    // The redirect survives as a question — it is not hidden — but it has no
    // trap to follow, so it falls to the tail rather than being dropped.
    const deck = [hidden(question("g1", "george")), redirect("r1", "g1")];
    expect(orderedDeck(deck, "mixed").map((q) => q.id)).toEqual(["r1"]);
  });

  it("are still listed for the EDITOR, after the live ones", () => {
    // The one screen that can put a hidden question back must be able to see
    // it. Same order and same side filter, with that one step removed.
    const deck = [question("g1", "george"), hidden(question("g2", "george"))];

    expect(editorDeck(deck, "george").map((q) => q.id)).toEqual(["g1", "g2"]);
    expect(editorDeck(deck, "chuck")).toEqual([]);
  });

  it("puts a hidden REDIRECT on Chuck's editor list, not George's", () => {
    // The editor's side filter has to agree with the live one: a redirect is
    // Chuck's, and a hidden one appearing under George would be un-unhideable
    // from the side it belongs to.
    const deck = [question("g1", "george"), hidden(redirect("r1", "g1"))];

    expect(editorDeck(deck, "george").map((q) => q.id)).toEqual(["g1"]);
    expect(editorDeck(deck, "chuck").map((q) => q.id)).toEqual(["r1"]);
  });
});
