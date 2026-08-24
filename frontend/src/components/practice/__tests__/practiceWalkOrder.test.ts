// =============================================================================
// practiceWalkOrder.test.ts — what the walk actually serves (measured)
// =============================================================================
//
// ## Measured against live DEV on 2026-08-23, not reasoned about
//
// The task treated "Marie cannot practice" as a defect report about the walk
// until measurement said otherwise. The real `walkSteps` was run against S-5 as
// DEV holds it, and it was CORRECT on both sides: chosen side only, deck order,
// answered questions only. What made practice feel broken was upstream — the
// list was interleaved, so nine of Chuck's ten questions had never been answered
// at all, and a walk over answered questions had one step to serve.
//
// These tests pin the sequences that measurement produced, so the walk cannot
// drift away from them unnoticed.

import { describe, expect, it } from "vitest";

import { walkSteps } from "../practiceWalk";
import { sideSections } from "../../../pages/practiceQueue";
import type { PracticeAnswer } from "../../../services/practiceAnswers";
import type { PracticeQuestion } from "../../../services/practice";

const q = (
  deck_key: string,
  side: "george" | "chuck",
  kind: string,
): PracticeQuestion =>
  ({ id: deck_key, deck_key, side, kind, follows_key: null, text: deck_key, hidden: false }) as
    unknown as PracticeQuestion;

/** S-5 in `sort_order`, as DEV holds it. The authored order is not key order. */
const S5: PracticeQuestion[] = [
  q("g3", "george", "cross"),
  q("g4", "george", "cross"),
  q("g2", "george", "cross"),
  q("g1", "george", "cross"),
  q("g5", "george", "cross"),
  q("c1", "chuck", "direct"),
  q("c2", "chuck", "direct"),
  q("c3", "chuck", "direct"),
  q("c4", "chuck", "direct"),
  q("c5", "chuck", "direct"),
];

const answer = (question_id: string): PracticeAnswer =>
  ({ question_id, text: `the answer to ${question_id}`, answered_on: "22 Aug" }) as
    unknown as PracticeAnswer;

/** The four answers S-5 actually carried when this was measured. */
const ANSWERED = [answer("g3"), answer("g4"), answer("g1"), answer("c1")];

const served = (side: "george" | "chuck", answers: PracticeAnswer[]) =>
  walkSteps(S5, answers, side).map((step) => step.question.deck_key);

describe("the practice walk", () => {
  it("serves the defense's answered questions in deck order — the measured sequence", () => {
    // g3, g4, g1 — NOT g1, g3, g4. The deck's authored order, with g2 and g5
    // absent because nobody has answered them.
    expect(served("george", ANSWERED)).toEqual(["g3", "g4", "g1"]);
  });

  it("serves Chuck's one answered question and no more", () => {
    expect(served("chuck", ANSWERED)).toEqual(["c1"]);
  });

  it("serves nothing from the side that was not chosen", () => {
    expect(served("george", ANSWERED)).not.toContain("c1");
    expect(served("chuck", ANSWERED)).not.toContain("g3");
  });

  it("skips an unanswered question rather than serving it empty", () => {
    // The whole side is answered here, so the sequence is the full deck order —
    // which is also what proves the three absences above are about the ANSWERS
    // and not about some filter quietly dropping rows.
    const all = S5.map((x) => answer(x.id));
    expect(served("george", all)).toEqual(["g3", "g4", "g2", "g1", "g5"]);
    expect(served("chuck", all)).toEqual(["c1", "c2", "c3", "c4", "c5"]);
  });

  it("deals exactly what the list shows for that side", () => {
    // ⚑ The two read DIFFERENT COLUMNS: the list picks the defense's side by
    // `kind === "cross"` and the walk picks it by `side === "george"`. Measured
    // 2026-08-23 across every deck on DEV, the two agree exactly — 16 george are
    // all cross, 30 chuck are all direct or redirect. This pins that agreement,
    // so the day a question is authored where they diverge, somebody is told
    // rather than Marie being practised on a question she was never shown.
    const all = S5.map((x) => answer(x.id));
    for (const side of ["george", "chuck"] as const) {
      const listed = sideSections(S5, side).flatMap((part) =>
        part.questions.map((x) => x.deck_key),
      );
      expect(served(side, all)).toEqual(listed);
    }
  });
});
