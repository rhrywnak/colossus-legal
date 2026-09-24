// The box's two calls: the right URLs and timeouts, and a failure that throws
// with the server's words rather than resolving to something the box would show.

import { afterEach, describe, expect, it, vi } from "vitest";

import { getChatCaseFile, keepLoaded } from "../chatCaseFile";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

const ok = (body: unknown) => {
  const mock = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => body });
  vi.stubGlobal("fetch", mock);
  return mock;
};

describe("the chat case file calls", () => {
  it("reads the activity from the admin route", async () => {
    const mock = ok({ loaded: false });
    await expect(getChatCaseFile()).resolves.toEqual({ loaded: false });
    expect(String(mock.mock.calls[0][0])).toContain("/api/admin/chat-case-file");
  });

  it("POSTs the ping to its own route", async () => {
    const mock = ok({ outcome: "read" });
    await expect(keepLoaded()).resolves.toEqual({ outcome: "read" });
    expect(String(mock.mock.calls[0][0])).toContain("/api/admin/chat-case-file/keep-loaded");
    expect(mock.mock.calls[0][1].method).toBe("POST");
    expect(mock.mock.calls[0][1].signal).toBeDefined();
  });

  it("throws with the status and the server's words on a failure", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({ ok: false, status: 403, text: async () => "admins only" }),
    );
    await expect(keepLoaded()).rejects.toThrow(/403.*admins only/);
    await expect(getChatCaseFile()).rejects.toThrow(/403/);
  });

  it("throws when the network is down", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("network down")));
    await expect(keepLoaded()).rejects.toThrow("network down");
  });
});
