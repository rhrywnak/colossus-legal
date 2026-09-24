// The jobs panel's and the Data tab's reads: the right admin routes, a timeout
// signal on each, and a failure that throws with the server's words.

import { afterEach, describe, expect, it, vi } from "vitest";

import { AiJobRefusal, getAiJobs, getLastRun, saveAiJob, switchBackAiJob } from "../aiJobs";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

const ok = (body: unknown) => {
  const mock = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => body });
  vi.stubGlobal("fetch", mock);
  return mock;
};

describe("the admin jobs reads", () => {
  it("reads the panel from its admin route, with a timeout", async () => {
    const mock = ok({ jobs: [] });
    await expect(getAiJobs()).resolves.toEqual({ jobs: [] });
    expect(String(mock.mock.calls[0][0])).toContain("/api/admin/ai-jobs");
    expect(mock.mock.calls[0][1].signal).toBeDefined();
  });

  it("reads the last run from the pipeline admin route", async () => {
    const mock = ok({ empty: null });
    await expect(getLastRun()).resolves.toEqual({ empty: null });
    expect(String(mock.mock.calls[0][0])).toContain("/api/admin/pipeline/last-run");
  });

  it("throws with the status and the server's words on a failure", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({ ok: false, status: 500, text: async () => "folder unreadable" }),
    );
    await expect(getAiJobs()).rejects.toThrow(/500.*folder unreadable/);
    await expect(getLastRun()).rejects.toThrow(/500/);
  });

  it("throws when the network is down", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("network down")));
    await expect(getAiJobs()).rejects.toThrow("network down");
  });
});

describe("Save and Switch back", () => {
  it("PUTs the job's changes to its own address, with a timeout", async () => {
    const mock = ok({ job: "answer_analysis" });
    await saveAiJob("answer_analysis", [{ key: "practice_read_model", value: "claude-opus-5-5" }]);
    expect(String(mock.mock.calls[0][0])).toContain("/api/admin/ai-jobs/answer_analysis");
    expect(mock.mock.calls[0][1].method).toBe("PUT");
    expect(JSON.parse(mock.mock.calls[0][1].body)).toEqual({
      changes: [{ key: "practice_read_model", value: "claude-opus-5-5" }],
    });
    expect(mock.mock.calls[0][1].signal).toBeDefined();
  });

  it("POSTs Switch back to the job's switch-back address", async () => {
    const mock = ok({ job: "discuss_chat" });
    await switchBackAiJob("discuss_chat");
    expect(String(mock.mock.calls[0][0])).toContain("/api/admin/ai-jobs/discuss_chat/switch-back");
    expect(mock.mock.calls[0][1].method).toBe("POST");
  });

  it("turns a refusal into the server's own sentence", async () => {
    const sentence = "Nothing changed: Answer analysis already uses that.";
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: false,
        status: 409,
        json: async () => ({ error: "conflict", message: sentence }),
      }),
    );
    const err = await saveAiJob("answer_analysis", []).catch((e: unknown) => e);
    expect(err).toBeInstanceOf(AiJobRefusal);
    expect((err as Error).message).toBe(sentence);
  });

  it("keeps a server failure a plain error, not a refusal", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({ ok: false, status: 500, text: async () => "store down" }),
    );
    const err = await switchBackAiJob("discuss_chat").catch((e: unknown) => e);
    expect(err).not.toBeInstanceOf(AiJobRefusal);
    expect((err as Error).message).toMatch(/500.*store down/);
  });
});
