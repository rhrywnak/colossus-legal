// =============================================================================
// askModels.test.ts — the chat catalogue no longer invents a model
// =============================================================================
//
// CC_TASK_CHAT_DEFAULT_MODEL_v1, ruling 4b.
//
// Until 2026-09-19 `fetchChatModels` caught its own failure and returned one
// hand-written entry, `claude-sonnet-4-6`, flagged `is_default: true`. That was
// the same defect as the backend constant this task removed — and worse in one
// respect: a page that could not reach its server looked exactly like a
// deployment that offered one model. When that id was deactivated in the Admin
// list, the fabricated entry went on naming it and every ask made through it
// answered 400.
//
// These tests pin the absence. The first two are the behaviour; the third is
// the one that would catch the fallback being typed back in.

import { beforeEach, describe, expect, it, vi } from "vitest";

import { fetchChatModels } from "../ask";

const BODY = {
  models: [
    { model_id: "claude-opus-5", display_name: "Claude Opus 5", is_default: true },
    { model_id: "claude-sonnet-5", display_name: "Claude Sonnet 5", is_default: false },
  ],
  default_model: "claude-opus-5",
};

const respond = (ok: boolean, status: number, body: unknown) => {
  const mock = vi.fn().mockResolvedValue({ ok, status, json: async () => body, text: async () => "" });
  vi.stubGlobal("fetch", mock);
  return mock;
};

beforeEach(() => {
  vi.unstubAllGlobals();
});

describe("fetchChatModels", () => {
  it("returns the catalogue the server served", async () => {
    const mock = respond(true, 200, BODY);
    await expect(fetchChatModels()).resolves.toEqual(BODY);
    expect(String(mock.mock.calls[0][0])).toContain("/api/chat/models");
  });

  it("THROWS on a failed request rather than inventing a model", async () => {
    respond(false, 503, {});
    // The status is in the sentence: the person who can fix a 503 needs to know
    // it was one, and the picker shows the sentence on hover.
    await expect(fetchChatModels()).rejects.toThrow("503");
  });

  it("throws on a transport failure too", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("network down")));
    await expect(fetchChatModels()).rejects.toThrow();
  });

  it("refuses a body missing the fields the picker renders", async () => {
    // `undefined` here would put `undefined` into the select's value and send
    // it as the chosen model on the next ask.
    const { models: _dropped, ...noModels } = BODY;
    respond(true, 200, noModels);
    await expect(fetchChatModels()).rejects.toThrow("contract mismatch");

    respond(true, 200, { models: [], default_model: 7 });
    await expect(fetchChatModels()).rejects.toThrow("contract mismatch");
  });

  it("accepts an EMPTY catalogue — that is a real deployment state", async () => {
    // A store with no active Anthropic row. Distinct from a failure, and it must
    // not be turned into one: Chat still answers on the server's stored default.
    const empty = { models: [], default_model: "claude-opus-5" };
    respond(true, 200, empty);
    await expect(fetchChatModels()).resolves.toEqual(empty);
  });

  it("names no model of its own, anywhere in the module", async () => {
    // ⚑ The regression guard. Every test above passes if somebody re-adds a
    // `catch` that returns a fabricated entry for a DIFFERENT id — this one
    // does not. Read off disk because that is where the literal would be.
    const { readFileSync } = await import("node:fs");
    const { join } = await import("node:path");
    // ⚑ BLOCK comments too, not just `//`. This file's own history is written
    // in a `/** … */` above `fetchChatModels` and it names the deleted id —
    // stripping one comment style and not the other reported the explanation
    // as the defect. The repo's Rust scanners strip `//` only; TypeScript doc
    // comments make that insufficient here.
    const source = readFileSync(join(__dirname, "..", "ask.ts"), "utf8")
      .replace(/\/\*[\s\S]*?\*\//g, "")
      .split("\n")
      .map((line) => {
        const at = line.indexOf("//");
        return at === -1 ? line : line.slice(0, at);
      })
      .join("\n");
    expect(source).not.toMatch(/claude-[a-z0-9-]+/);
    expect(source).not.toContain("is_default: true");
  });
});
