// questionChat.test.ts — the chat service's parsers: what it accepts, what it refuses.

import { describe, expect, it } from "vitest";

import { asThread, asThreads, ChatCallError, SseParser } from "../questionChat";

const threads = {
  question_id: "q",
  viewer: "docmarie",
  model_short_name: "Opus 5",
  grounded: true,
  others: ["Chuck", "Roman"],
  threads: [],
  earlier: null,
  client_idle_timeout_secs: 150,
  max_turns: 200,
};

describe("asThreads", () => {
  it("accepts a well-formed payload", () => {
    expect(asThreads(threads).viewer).toBe("docmarie");
  });

  it.each(["viewer", "model_short_name", "grounded", "others", "threads", "earlier", "client_idle_timeout_secs"])(
    "refuses a payload missing %s, by name",
    (field) => {
      const broken: Record<string, unknown> = { ...threads };
      delete broken[field];
      expect(() => asThreads(broken)).toThrow(ChatCallError);
      expect(() => asThreads(broken)).toThrow(`"${field}"`);
    },
  );

  it("refuses a non-object", () => {
    expect(() => asThreads(null)).toThrow("the server sent no object");
  });
});

describe("asThread", () => {
  const thread = { username: "docmarie", display_name: "Marie", read_only: false, messages: [] };

  it("accepts a well-formed payload", () => {
    expect(asThread(thread).display_name).toBe("Marie");
  });

  it.each(["display_name", "read_only", "messages"])("refuses a payload missing %s", (field) => {
    const broken: Record<string, unknown> = { ...thread };
    delete broken[field];
    expect(() => asThread(broken)).toThrow(`"${field}"`);
  });
});

describe("SseParser", () => {
  it("turns a malformed event into a named failure, never a silent drop", () => {
    const events = new SseParser().push("event: delta\ndata: {not json\n\n");
    expect(events).toHaveLength(1);
    expect(events[0].event).toBe("failed");
  });

  it("ignores a block with no event name (a keep-alive comment)", () => {
    expect(new SseParser().push(": keep-alive\n\n")).toEqual([]);
  });
});
