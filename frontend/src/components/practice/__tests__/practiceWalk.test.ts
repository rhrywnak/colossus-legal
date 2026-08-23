// =============================================================================
// practiceWalk.test.ts — what the walk offers, and where it is
// =============================================================================
//
// The network half of the ruling — that the walk WRITES NOTHING — is asserted in
// `practiceWalkPage.test.ts` by spying the fetch layer across the whole loop.
// This file is the other half: which questions it offers at all.

import { describe, expect, it } from "vitest";

import { walkAt, walkSteps } from "../practiceWalk";
import type { PracticeAnswer } from "../../../services/practiceAnswers";
import type { PracticeQuestion } from "../../../services/practice";

const q = (id: string, side: string): PracticeQuestion =>
  ({ id, side, text: `question ${id}`, kind: "cross", hidden: false }) as unknown as PracticeQuestion;

const a = (questionId: string): PracticeAnswer => ({
  question_id: questionId,
  text: `answer to ${questionId}`,
  answered_on: "Answered on 22 Aug",
});

describe("which questions a walk offers", () => {
  it("offers only questions she has ANSWERED", () => {
    // Practising an answer she has not written is nothing to practise: there is
    // nothing to reveal and nothing to check herself against.
    const steps = walkSteps([q("g1", "george"), q("g2", "george")], [a("g2")], "george");

    expect(steps).toHaveLength(1);
    expect(steps[0].question.id).toBe("g2");
  });

  it("offers only the chosen side", () => {
    const steps = walkSteps(
      [q("g1", "george"), q("c1", "chuck")],
      [a("g1"), a("c1")],
      "chuck",
    );

    expect(steps.map((s) => s.question.id)).toEqual(["c1"]);
  });

  it("keeps DECK order, not the order she answered them", () => {
    // The order the defense would ask them at trial is the order she needs to
    // get used to. Sorting by when she answered rehearses an artefact of her
    // afternoon.
    const steps = walkSteps(
      [q("g1", "george"), q("g2", "george"), q("g3", "george")],
      [a("g3"), a("g1"), a("g2")],
      "george",
    );

    expect(steps.map((s) => s.question.id)).toEqual(["g1", "g2", "g3"]);
  });

  it("pairs each question with ITS OWN answer", () => {
    // A positional pairing would survive every test above and put g1's answer
    // under g2 the moment one question was unanswered.
    const steps = walkSteps([q("g1", "george"), q("g2", "george")], [a("g2"), a("g1")], "george");

    expect(steps[0].answer.text).toBe("answer to g1");
    expect(steps[1].answer.text).toBe("answer to g2");
  });
});

describe("where the walk is", () => {
  const steps = walkSteps([q("g1", "george"), q("g2", "george")], [a("g1"), a("g2")], "george");

  it("asks before it reveals", () => {
    expect(walkAt(steps, 0, false)).toMatchObject({ kind: "asking", at: 0, total: 2 });
  });

  it("reveals the same question, not the next one", () => {
    const state = walkAt(steps, 0, true);
    expect(state).toMatchObject({ kind: "revealed", at: 0 });
    expect(state.kind === "revealed" && state.step.question.id).toBe("g1");
  });

  it("ends after the last one", () => {
    expect(walkAt(steps, 2, false)).toEqual({ kind: "end", total: 2 });
  });

  it("says so when the side has nothing answered, rather than showing a blank walk", () => {
    expect(walkAt([], 0, false)).toEqual({ kind: "nothing" });
  });

  it("skipping is pressing Next — an unrevealed step advances like any other", () => {
    // There is no skip control and no skipped state. Moving on without
    // revealing IS skipping, and nothing anywhere records that she did.
    expect(walkAt(steps, 1, false)).toMatchObject({ kind: "asking", at: 1 });
  });
});
