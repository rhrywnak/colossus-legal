// =============================================================================
// practiceAnswers.test.ts — the question page's two reads and its one write
// =============================================================================
//
// The service layer IS testable here — `global.fetch` can be replaced — which
// makes it the one boundary in this project where a real behavioural test is
// possible. The components above it cannot be rendered and the handlers below it
// cannot be called; this thin layer is what is left, so it is tested properly.

import { beforeEach, describe, expect, it, vi } from "vitest";

import { fetchQuestionAnswers, openAnswerSession } from "../practiceAnswers";

const ok = (body: unknown) => {
  const mock = vi.fn().mockResolvedValue({
    ok: true,
    status: 200,
    json: async () => body,
  });
  vi.stubGlobal("fetch", mock);
  return mock;
};

const failing = (status: number) => {
  const mock = vi.fn().mockResolvedValue({
    ok: false,
    status,
    text: async () => "",
    json: async () => ({}),
  });
  vi.stubGlobal("fetch", mock);
  return mock;
};

beforeEach(() => {
  vi.unstubAllGlobals();
});

describe("one question's answers", () => {
  it("asks the question's own address", async () => {
    const mock = ok({ current: null, earlier: [] });
    await fetchQuestionAnswers("q-1");

    expect(String(mock.mock.calls[0][0])).toContain("/practice/questions/q-1/answers");
  });

  it("escapes the id rather than pasting it into a path", async () => {
    const mock = ok({ current: null, earlier: [] });
    await fetchQuestionAnswers("id/with/slashes");

    expect(String(mock.mock.calls[0][0])).not.toContain("id/with/slashes");
  });

  it("carries the current answer and the earlier ones apart", async () => {
    ok({
      current: { answer_id: "a2", text: "what stands", answered_on: "Answered on 22 Aug" },
      earlier: [{ answer_id: "a1", text: "what came before", answered_on: "Answered on 19 Aug" }],
    });
    const answers = await fetchQuestionAnswers("q-1");

    expect(answers.current?.text).toBe("what stands");
    expect(answers.earlier).toHaveLength(1);
    expect(answers.earlier[0].text).toBe("what came before");
  });

  it("accepts a null current — she has not answered yet", async () => {
    ok({ current: null, earlier: [] });
    await expect(fetchQuestionAnswers("q-1")).resolves.toEqual({ current: null, earlier: [] });
  });

  it("REFUSES a payload missing `earlier` rather than rendering no history", async () => {
    // An absent `earlier` would render "0 earlier versions" over a history the
    // server never sent — a contract breach dressed as an empty one.
    ok({ current: null });
    await expect(fetchQuestionAnswers("q-1")).rejects.toThrow(/contract mismatch/);
  });

  it("REFUSES a payload missing `current`", async () => {
    // `null` is a legitimate value and `undefined` is not: the first says "she
    // has not answered", the second says the field never arrived.
    ok({ earlier: [] });
    await expect(fetchQuestionAnswers("q-1")).rejects.toThrow(/contract mismatch/);
  });

  it("says which HTTP status refused it", async () => {
    failing(500);
    await expect(fetchQuestionAnswers("q-1")).rejects.toThrow(/500/);
  });
});

describe("the invisible sitting", () => {
  it("POSTs to the scenario's answer-session address", async () => {
    const mock = ok({ session_id: "s-1" });
    await openAnswerSession("awad-v-cfs", "sc-1");

    expect(String(mock.mock.calls[0][0])).toContain("/practice/answer-session");
    expect(mock.mock.calls[0][1]).toMatchObject({ method: "POST" });
  });

  it("hands back the session id", async () => {
    ok({ session_id: "s-1" });
    await expect(openAnswerSession("c", "s")).resolves.toBe("s-1");
  });

  it("REFUSES a response with no session id", async () => {
    // Otherwise the caller posts an answer with `undefined` for a NOT NULL
    // foreign key, and the failure surfaces as a 500 with a constraint name in
    // it rather than as the contract breach it is.
    ok({});
    await expect(openAnswerSession("c", "s")).rejects.toThrow(/contract mismatch/);
  });

  it("says which HTTP status refused it", async () => {
    failing(403);
    await expect(openAnswerSession("c", "s")).rejects.toThrow(/403/);
  });
});
