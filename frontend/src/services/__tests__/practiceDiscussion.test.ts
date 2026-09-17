// =============================================================================
// practiceDiscussion.test.ts — the dock's two calls
// =============================================================================

import { beforeEach, describe, expect, it, vi } from "vitest";

import { DISCUSS_SEND_TIMEOUT_MS, DiscussionError, fetchDiscussion, sendDiscussion } from "../practiceDiscussion";

const BODY = {
  question_id: "q-1",
  scenario_code: "S-5",
  position: 4,
  turns: [],
  models: [{ model_id: "claude-opus-5", display_name: "Claude Opus 5", billing_class: "billed" }],
  default_model: "claude-opus-5",
  max_turns: 40,
  model_turns: 0,
};

const respond = (ok: boolean, status: number, body: unknown) => {
  const mock = vi.fn().mockResolvedValue({ ok, status, json: async () => body, text: async () => "" });
  vi.stubGlobal("fetch", mock);
  return mock;
};

beforeEach(() => {
  vi.unstubAllGlobals();
});

describe("fetchDiscussion", () => {
  it("GETs the question's discussion address, escaped", async () => {
    const mock = respond(true, 200, BODY);
    await expect(fetchDiscussion("q/1")).resolves.toEqual(BODY);
    expect(String(mock.mock.calls[0][0])).toContain("/api/practice/questions/q%2F1/discussion");
    expect(mock.mock.calls[0][1].signal).toBeDefined();
  });

  it("refuses a body missing a field the dock renders", async () => {
    const { models: _dropped, ...noModels } = BODY;
    respond(true, 200, noModels);
    await expect(fetchDiscussion("q-1")).rejects.toThrow(/contract mismatch/);
  });
});

describe("sendDiscussion", () => {
  it("POSTs the text, the model and — only when given — the draft, with the long timeout", async () => {
    const mock = respond(true, 200, BODY);
    await sendDiscussion("q-1", { text: "How?", model: "claude-opus-5", draft_answer: "my draft" });
    const [, init] = mock.mock.calls[0];
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body)).toEqual({ text: "How?", model: "claude-opus-5", draft_answer: "my draft" });
    expect(DISCUSS_SEND_TIMEOUT_MS).toBeGreaterThan(30000);
  });

  it("carries the status of a refusal, so the dock can tell the cap (409) from a failure", async () => {
    respond(false, 409, { message: "cap" });
    const error = await sendDiscussion("q-1", { text: "x", model: "m" }).catch((e: unknown) => e);
    expect(error).toBeInstanceOf(DiscussionError);
    expect((error as DiscussionError).status).toBe(409);
    respond(false, 503, {});
    await expect(sendDiscussion("q-1", { text: "x", model: "m" })).rejects.toThrow(/not answered.*503/);
  });
});
