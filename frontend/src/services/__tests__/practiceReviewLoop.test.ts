// =============================================================================
// practiceReviewLoop.test.ts — the review loop's four writes
// =============================================================================
//
// The service layer is the one boundary here a behavioural test can reach
// (`global.fetch` is replaceable). What these pin: each call's address and verb,
// that ids are escaped, that the body carries only the text, and that every
// refusal and every malformed response THROWS rather than resolving quietly.

import { beforeEach, describe, expect, it, vi } from "vitest";

import type { SweptItem } from "../../components/practice/deckSweep";

import {
  addAnswerNote,
  addQuestionNote,
  markDeckReviewed,
  strikeNote,
} from "../practiceReviewLoop";

const NOTE = {
  id: "n-1",
  question_id: "q-1",
  answer_id: "a-1",
  author: "Chuck",
  text: "hold the number",
  when: "17 Sep",
  struck: null,
};

const ok = (body: unknown) => {
  const mock = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => body });
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

/** What a page showed, in the shape the sweep takes. */
const SHOWN: SweptItem[] = [
  { kind: "answer", id: "a-1" },
  { kind: "note", id: "n-1" },
];
const SERVED_AT = "2026-09-22T21:31:00Z";

describe("Done reviewing", () => {
  it("PUTs the review-cursor address carrying EXACTLY the items the page showed", async () => {
    // The ruling, on the wire: the press sends ids, so an item that arrived
    // after the page loaded cannot be in the body and cannot be swept.
    const mock = ok({ scenario_id: "sc-1", marked: 2 });
    await expect(markDeckReviewed("awad", "sc-1", SHOWN, SERVED_AT)).resolves.toBe(2);
    expect(String(mock.mock.calls[0][0])).toContain("/api/cases/awad/scenarios/sc-1/practice/review-cursor");
    expect(mock.mock.calls[0][1]).toMatchObject({ method: "PUT" });
    expect(JSON.parse(mock.mock.calls[0][1].body)).toEqual({
      items: SHOWN,
      served_at: SERVED_AT,
    });
    // A timeout signal rides every call (authFetch's AbortController).
    expect(mock.mock.calls[0][1].signal).toBeDefined();
  });

  it("resolves on ZERO marked — everything shown had already been read", async () => {
    ok({ scenario_id: "sc-1", marked: 0 });
    await expect(markDeckReviewed("awad", "sc-1", SHOWN, SERVED_AT)).resolves.toBe(0);
  });

  it("throws on a refusal, naming the status — the bar keeps its count", async () => {
    failing(500);
    await expect(markDeckReviewed("awad", "sc-1", SHOWN, SERVED_AT)).rejects.toThrow(
      /not marked reviewed.*500/,
    );
  });

  it("throws on a response with no count", async () => {
    ok({});
    await expect(markDeckReviewed("awad", "sc-1", SHOWN, SERVED_AT)).rejects.toThrow(
      /contract mismatch/,
    );
  });
});

describe("notes", () => {
  it("POSTs a note on an answer, carrying only the text", async () => {
    const mock = ok(NOTE);
    await expect(addAnswerNote("a/1", "hold the number")).resolves.toEqual(NOTE);
    const [url, init] = mock.mock.calls[0];
    expect(String(url)).toContain("/api/practice/answers/a%2F1/notes");
    expect(init).toMatchObject({ method: "POST" });
    expect(JSON.parse(init.body)).toEqual({ text: "hold the number" });
  });

  it("POSTs a note on a question to the question's address", async () => {
    const mock = ok(NOTE);
    await addQuestionNote("q-1", "why this one?");
    expect(String(mock.mock.calls[0][0])).toContain("/api/practice/questions/q-1/notes");
  });

  it("PUTs a strike and returns the struck note", async () => {
    const struck = { ...NOTE, struck: "struck 18 Sep" };
    const mock = ok(struck);
    await expect(strikeNote("n-1")).resolves.toEqual(struck);
    expect(String(mock.mock.calls[0][0])).toContain("/api/practice/notes/n-1/strike");
    expect(mock.mock.calls[0][1]).toMatchObject({ method: "PUT" });
  });

  it("throws on a refused note write", async () => {
    failing(400);
    await expect(addAnswerNote("a-1", " ")).rejects.toThrow(/not saved.*400/);
  });

  it("throws on a refused question note write", async () => {
    failing(400);
    await expect(addQuestionNote("q-1", " ")).rejects.toThrow(/not saved.*400/);
  });

  it("throws on a refused strike, naming the status", async () => {
    failing(409);
    await expect(strikeNote("n-1")).rejects.toThrow(/not struck.*409/);
  });

  it("throws on a response that is not a note — no `struck` key is not 'standing'", async () => {
    const { struck: _dropped, ...noStruck } = NOTE;
    ok(noStruck);
    await expect(strikeNote("n-1")).rejects.toThrow(/contract mismatch/);
  });
});
