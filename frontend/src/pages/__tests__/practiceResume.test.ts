// Tests for `resumeAt` — where a reload puts her back.
//
// This is the whole of Section B's fix, expressed as a function: .401 could not
// answer "which question was she on" because the answer lived in component state
// that a reload destroyed. It lives on the server now — the sitting's stored
// queue, and the rows already written — and this is where the two are turned
// back into a position.
//
// Every case below is one way that arithmetic can be wrong in a way no
// screenshot shows: a question dealt twice, a question the deck no longer holds,
// a sitting that is already finished.

import { describe, expect, it } from "vitest";

import type { PracticeQuestion } from "../../services/practice";
import type { Sitting } from "../../services/practiceFlow";
import { resumeAt } from "../PracticeSessionPage";

function question(id: string): PracticeQuestion {
  return {
    id,
    side: "george",
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
    kind: "cross",
    deck_key: id,
    follows_key: null,
    hidden: false,
    answered_on: null,
    draft_by: null,
  };
}

const DECK = ["g1", "g2", "g3"].map(question);

function sitting(queue: string[], answered: string[], hidden: string[] = []): Sitting {
  return {
    session_id: "s1",
    scenario_id: "sc1",
    who: "george",
    queue,
    answered,
    ended: false,
    hidden,
  };
}

describe("resumeAt", () => {
  it("starts at the first question when nothing has been answered", () => {
    const { queue, index } = resumeAt(DECK, sitting(["g1", "g2", "g3"], []));
    expect(queue.map((q) => q.id)).toEqual(["g1", "g2", "g3"]);
    expect(index).toBe(0);
  });

  it("resumes at the next UNDEALT question — the .401 defect, in one line", () => {
    // Roman answered question 1, left, came back, and was shown question 1
    // again. The answer was in the database the whole time.
    const { index } = resumeAt(DECK, sitting(["g1", "g2", "g3"], ["g1"]));
    expect(index).toBe(1);
  });

  it("counts a skipped question as dealt", () => {
    // She was SHOWN it and set it aside. Dealing it again is the one thing the
    // control she pressed asked not to happen — and a skip writes an answer row
    // exactly so this arithmetic can see it.
    const { index } = resumeAt(DECK, sitting(["g1", "g2", "g3"], ["g1", "g2"]));
    expect(index).toBe(2);
  });

  it("advances past BOTH places of a question that was re-queued", () => {
    // "Ask me this one again later" appends the question to the queue, so it
    // appears twice; answering it twice must consume both. Membership testing
    // would leave her stuck on the second copy forever, because the id is
    // always "answered".
    const { queue, index } = resumeAt(DECK, sitting(["g1", "g2", "g1"], ["g1", "g2", "g1"]));
    expect(queue).toHaveLength(3);
    expect(index).toBe(3);
  });

  it("stops on the second copy when only the first has been answered", () => {
    const { index } = resumeAt(DECK, sitting(["g1", "g2", "g1"], ["g1", "g2"]));
    expect(index).toBe(2);
  });

  it("drops a queued id the deck no longer holds rather than rendering a blank", () => {
    // The deck can change between the sitting opening and a reload — that is the
    // whole of the deck editor. A question that is gone cannot be rendered, and
    // a placeholder for it would be the screen inventing one.
    const { queue, index } = resumeAt(DECK, sitting(["g1", "gone", "g2"], ["g1"]));
    expect(queue.map((q) => q.id)).toEqual(["g1", "g2"]);
    expect(index).toBe(1);
  });

  it("reports a finished sitting as past its end", () => {
    // The page sends her back to the start card on this, rather than rendering
    // an empty question screen that reads as a deck that failed to load.
    const { queue, index } = resumeAt(DECK, sitting(["g1", "g2"], ["g1", "g2"]));
    expect(index).toBe(queue.length);
  });

  it("yields an empty queue for a sitting stored before the queue existed", () => {
    // Every session opened before flow v1 carries no queue, and the server sends
    // an empty array rather than guessing at one. Nothing can be resumed, and
    // the page says so by going back to the start card.
    const { queue, index } = resumeAt(DECK, sitting([], []));
    expect(queue).toEqual([]);
    expect(index).toBe(0);
  });

  // ── hotfix §3.6 ───────────────────────────────────────────────────────────
  it("walks past a question hidden while this sitting had it queued", () => {
    // Chuck hid g2 after the sitting was dealt. .402 asked it anyway.
    const { queue, index } = resumeAt(DECK, sitting(["g1", "g2", "g3"], ["g1"], ["g2"]));
    expect(queue.map((q) => q.id)).toEqual(["g1", "g3"]);
    // g1 is answered, so she lands on g3 — g2 is not a stop she pauses at.
    expect(index).toBe(1);
  });

  it("leaves an ALREADY-ANSWERED hidden question's place alone", () => {
    // She answered g1 and g2; g2 was hidden afterwards. Her answers stand, the
    // sheet still prints them, and the queue simply no longer offers it back.
    const { queue, index } = resumeAt(DECK, sitting(["g1", "g2", "g3"], ["g1", "g2"], ["g2"]));
    expect(queue.map((q) => q.id)).toEqual(["g1", "g3"]);
    expect(index).toBe(1);
  });

  it("hides nothing when the list is empty, which is the normal case", () => {
    const { queue } = resumeAt(DECK, sitting(["g1", "g2", "g3"], []));
    expect(queue).toHaveLength(3);
  });
});