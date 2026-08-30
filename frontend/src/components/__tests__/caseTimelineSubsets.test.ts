/**
 * Service tests for the timeline subsets client (task 2).
 *
 * ## ⚑ THE HALF THESE EXIST FOR
 *
 * The sibling `caseTimelineWrites.test.ts` says it: the .377 build shipped a
 * client calling an API path the axum router did not serve — well-typed,
 * syntactically perfect, and pointing nowhere. So these pin the METHOD and the
 * PATH of every call against the nine routes `api::timeline_subsets` declares,
 * alongside the behaviours that would otherwise fail in silence: every distinct
 * failure must throw its OWN sentence, the server's own message must reach the
 * person who caused it, and a 200 with the wrong shape must not become a subset
 * with an `undefined` name rendered into the section.
 *
 * `fetch` is stubbed at the global, which is what `authFetch` calls. No DOM.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  createSubset,
  deleteSubset,
  getSubset,
  listSubsets,
  replaceSubsetEvents,
  undeleteSubset,
  updateSubset,
} from "../../services/caseTimelineSubsets";

const ID = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee";

/** A minimal well-formed subset, as a write's response carries it. */
const SUBSET = {
  id: ID,
  name: "The $50,000",
  description: "Emil's own money leaves his account.",
  events: [],
  carried_by: ["S-11"],
  event_count: 0,
  gap_count: 0,
  created_by: "roman",
  created_at: "2026-08-30T12:00:00Z",
  updated_by: "roman",
  updated_at: "2026-08-30T12:00:00Z",
};

type Call = { url: string; init: RequestInit | undefined };
let calls: Call[];

function stubFetch(response: {
  ok: boolean;
  status?: number;
  body?: unknown;
  throws?: Error;
  badJson?: boolean;
}): void {
  vi.stubGlobal(
    "fetch",
    vi.fn((url: string, init?: RequestInit) => {
      calls.push({ url, init });
      if (response.throws) return Promise.reject(response.throws);
      return Promise.resolve({
        ok: response.ok,
        status: response.status ?? (response.ok ? 200 : 500),
        json: () =>
          response.badJson
            ? Promise.reject(new Error("not json"))
            : Promise.resolve(response.body),
      } as unknown as Response);
    }),
  );
}

beforeEach(() => {
  calls = [];
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("every call reaches the route the backend declares", () => {
  it("lists with GET /api/timeline/subsets", async () => {
    stubFetch({ ok: true, body: [SUBSET] });
    const out = await listSubsets();
    expect(calls[0].url).toContain("/api/timeline/subsets");
    // A read is a GET: authFetch sends no method for one.
    expect(calls[0].init?.method).toBeUndefined();
    expect(out).toHaveLength(1);
    expect(out[0].name).toBe("The $50,000");
  });

  it("reads one with GET /api/timeline/subsets/:id", async () => {
    stubFetch({ ok: true, body: SUBSET });
    const out = await getSubset(ID);
    expect(calls[0].url).toContain(`/api/timeline/subsets/${ID}`);
    expect(out.id).toBe(ID);
  });

  it("creates with POST /api/timeline/subsets, carrying the whole set", async () => {
    stubFetch({ ok: true, body: SUBSET });
    await createSubset("The $50,000", "a story", [
      { event_id: "e1", position: 1 },
      { event_id: "e2", position: 2, note: "Admissions p. 1" },
    ]);
    expect(calls[0].url).toContain("/api/timeline/subsets");
    expect(calls[0].init?.method).toBe("POST");
    const body = JSON.parse(String(calls[0].init?.body));
    expect(body).toMatchObject({ name: "The $50,000", description: "a story" });
    expect(body.events).toHaveLength(2);
    expect(body.events[1]).toEqual({ event_id: "e2", position: 2, note: "Admissions p. 1" });
  });

  it("edits name/description with PUT /api/timeline/subsets/:id and NO events", async () => {
    // ⚑ The events ride their own endpoint. Sending them here would be a second
    // opinion about the ordered set on an endpoint that does not own it.
    stubFetch({ ok: true, body: SUBSET });
    await updateSubset(ID, "New name", "New description");
    expect(calls[0].url).toContain(`/api/timeline/subsets/${ID}`);
    expect(calls[0].init?.method).toBe("PUT");
    const body = JSON.parse(String(calls[0].init?.body));
    expect(body).toEqual({ name: "New name", description: "New description" });
    expect(body).not.toHaveProperty("events");
  });

  it("replaces the set with PUT /api/timeline/subsets/:id/events", async () => {
    stubFetch({ ok: true, body: SUBSET });
    await replaceSubsetEvents(ID, [{ event_id: "e1", position: 1 }]);
    expect(calls[0].url).toContain(`/api/timeline/subsets/${ID}/events`);
    expect(calls[0].init?.method).toBe("PUT");
    expect(JSON.parse(String(calls[0].init?.body)).events).toHaveLength(1);
  });

  it("deletes with DELETE /api/timeline/subsets/:id and no body", async () => {
    stubFetch({ ok: true, body: { ...SUBSET, deleted_at: "2026-08-30T13:00:00Z" } });
    const out = await deleteSubset(ID);
    expect(calls[0].init?.method).toBe("DELETE");
    expect(calls[0].init?.body).toBeUndefined();
    // The DELETE answers with the subset it just deleted, which is what lets the
    // section replace the row with the undo line instead of inferring "gone"
    // from a status code.
    expect(out.deleted_at).toBe("2026-08-30T13:00:00Z");
  });

  it("undoes with POST /api/timeline/subsets/:id/undelete and no body", async () => {
    stubFetch({ ok: true, body: SUBSET });
    await undeleteSubset(ID);
    expect(calls[0].url).toContain(`/api/timeline/subsets/${ID}/undelete`);
    expect(calls[0].init?.method).toBe("POST");
    expect(calls[0].init?.body).toBeUndefined();
  });

  it("encodes the id rather than pasting it into the path", async () => {
    stubFetch({ ok: true, body: SUBSET });
    await getSubset("a/b c");
    expect(calls[0].url).toContain("a%2Fb%20c");
  });
});

describe("nothing is swallowed — every failure is its own sentence", () => {
  it("a network failure names the resource and the cause", async () => {
    stubFetch({ ok: false, throws: new Error("connection reset") });
    await expect(listSubsets()).rejects.toThrow(
      /Failed to load the timeline subsets \(connection reset\)/,
    );
  });

  it("a non-2xx READ carries the status", async () => {
    stubFetch({ ok: false, status: 500, body: {} });
    await expect(getSubset(ID)).rejects.toThrow(/HTTP 500/);
  });

  it("a non-2xx WRITE carries the server's own message, not just the status", async () => {
    // ⚑ This is what puts "a subset with that name is already on this case" in
    // front of the person who typed it. T1 answers 409 with `message`.
    stubFetch({
      ok: false,
      status: 409,
      body: { error: "conflict", message: "a subset with that name is already on this case" },
    });
    await expect(createSubset("dup", "", [])).rejects.toThrow(
      /That subset was not created \(HTTP 409 — a subset with that name is already on this case\)/,
    );
  });

  it("an unparseable body is a DIFFERENT sentence from a bad status", async () => {
    stubFetch({ ok: true, badJson: true });
    await expect(deleteSubset(ID)).rejects.toThrow(
      /That subset was not deleted — the server's answer was not valid JSON/,
    );
  });

  it("a 200 with the wrong SHAPE fails by name, not as an undefined on screen", async () => {
    // Without this the section renders a row whose name is `undefined` — a
    // screen that looks like a data problem and is a contract problem.
    stubFetch({ ok: true, body: { id: ID } });
    await expect(updateSubset(ID, "x", "y")).rejects.toThrow(
      /disagree about the payload shape/,
    );
  });

  it("a list that is not a list fails by name", async () => {
    stubFetch({ ok: true, body: { subsets: [] } });
    await expect(listSubsets()).rejects.toThrow(/came back as something other than a list/);
  });

  it("a subset with no events array still reads, as an empty story", async () => {
    // `events` is absent on the summary shape and present on the detail one;
    // defaulting it keeps a well-formed 200 from failing the shape check.
    stubFetch({ ok: true, body: { id: ID, name: "n" } });
    await expect(getSubset(ID)).resolves.toMatchObject({ id: ID, events: [] });
  });
});

describe("every call is bounded by a timeout", () => {
  it("passes the standing 30s ceiling to authFetch on a read and a write", async () => {
    // Rule 13: a fetch with no AbortController is a page that hangs forever on a
    // dead backend. authFetch arms it from `timeoutMs`.
    stubFetch({ ok: true, body: [SUBSET] });
    await listSubsets();
    expect(calls[0].init?.signal).toBeDefined();

    calls = [];
    stubFetch({ ok: true, body: SUBSET });
    await createSubset("n", "d", []);
    expect(calls[0].init?.signal).toBeDefined();
  });
});
