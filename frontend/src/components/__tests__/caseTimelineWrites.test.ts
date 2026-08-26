/**
 * Service tests for the chronology's write client (Phase C).
 *
 * ## ⚑ THE HALF THESE EXIST FOR
 *
 * The .377 build shipped a client calling an API path the axum router did not
 * serve — well-typed, syntactically perfect, and pointing nowhere. So these pin
 * the METHOD and the PATH of every write against the routes `api::timeline`
 * declares, alongside the two behaviours that would otherwise fail in silence:
 * a failed write must throw a sentence carrying the server's own message, and a
 * 200 with the wrong shape must not become an event with an `undefined` title
 * rendered into the list.
 *
 * `fetch` is stubbed at the global, which is what `authFetch` calls. No DOM.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  addTimelineNote,
  createTimelineEvent,
  deleteTimelineEvent,
  deleteTimelineNote,
  linkTimelineDocument,
  searchTimelineDocuments,
  undeleteTimelineEvent,
  unlinkTimelineDocument,
  updateTimelineEvent,
} from "../../services/caseTimelineWrites";

/** A minimal well-formed event, as a write's response carries it. */
const EVENT = {
  id: "11111111-2222-3333-4444-555555555555",
  event_date: "2012-04-12",
  date_precision: "day",
  approximate: false,
  phase: "appeals",
  title: "Judge Tighe Issues Post-Appeal Order",
  attributes: {},
  tags: [],
  links: [],
  note_count: 0,
  created_at: "2026-08-26T12:00:00Z",
  updated_at: "2026-08-26T12:00:00Z",
};

/** The last call `fetch` was made with. */
type Call = { url: string; init: RequestInit | undefined };

let calls: Call[];

function stubFetch(response: {
  ok: boolean;
  status?: number;
  body?: unknown;
  throws?: Error;
}): void {
  vi.stubGlobal(
    "fetch",
    vi.fn((url: string, init?: RequestInit) => {
      calls.push({ url, init });
      if (response.throws) return Promise.reject(response.throws);
      return Promise.resolve({
        ok: response.ok,
        status: response.status ?? (response.ok ? 200 : 500),
        json: () => Promise.resolve(response.body),
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

describe("every write reaches the route the backend declares", () => {
  beforeEach(() => stubFetch({ ok: true, body: EVENT }));

  it("creates with POST /api/timeline/events", async () => {
    await createTimelineEvent({ event_date: "2012-04-12", title: "x", phase: "appeals" });
    expect(calls[0].url).toContain("/api/timeline/events");
    expect(calls[0].init?.method).toBe("POST");
    expect(JSON.parse(String(calls[0].init?.body))).toMatchObject({ phase: "appeals" });
  });

  it("edits with PUT /api/timeline/events/:id", async () => {
    await updateTimelineEvent(EVENT.id, {
      event_date: "2012-04-12",
      title: "x",
      phase: "appeals",
    });
    expect(calls[0].url).toContain(`/api/timeline/events/${EVENT.id}`);
    expect(calls[0].init?.method).toBe("PUT");
  });

  it("strips links from an edit, because the endpoint refuses the field", async () => {
    // ⚑ The server's request struct is `deny_unknown_fields`, so sending a
    // create's payload to the edit endpoint would be a 400 naming a field the
    // author never saw. The form is one component for both paths, so this is
    // the seam where the difference is enforced.
    await updateTimelineEvent(EVENT.id, {
      event_date: "2012-04-12",
      title: "x",
      phase: "appeals",
      links: [{ target_type: "document", target_id: "doc-a" }],
    });
    expect(JSON.parse(String(calls[0].init?.body))).not.toHaveProperty("links");
  });

  it("deletes with DELETE /api/timeline/events/:id and no body", async () => {
    await deleteTimelineEvent(EVENT.id);
    expect(calls[0].init?.method).toBe("DELETE");
    expect(calls[0].init?.body).toBeUndefined();
  });

  it("undoes with POST /api/timeline/events/:id/undelete", async () => {
    await undeleteTimelineEvent(EVENT.id);
    expect(calls[0].url).toContain(`/api/timeline/events/${EVENT.id}/undelete`);
    expect(calls[0].init?.method).toBe("POST");
  });

  it("links with POST /api/timeline/events/:id/links", async () => {
    await linkTimelineDocument(EVENT.id, { target_type: "document", target_id: "doc-a" });
    expect(calls[0].url).toContain(`/api/timeline/events/${EVENT.id}/links`);
    expect(calls[0].init?.method).toBe("POST");
  });

  it("unlinks by the natural key, in the query string", async () => {
    // The key is three values a human picked off a screen, so it travels in the
    // address of the thing being removed rather than in a DELETE body that
    // proxies drop.
    await unlinkTimelineDocument(EVENT.id, "document", "doc-a");
    expect(calls[0].url).toContain("target_type=document");
    expect(calls[0].url).toContain("target_id=doc-a");
    expect(calls[0].init?.method).toBe("DELETE");
  });

  it("escapes a target id that would otherwise become a query of its own", async () => {
    await unlinkTimelineDocument(EVENT.id, "document", "doc a&b=c");
    expect(calls[0].url).toContain("target_id=doc+a%26b%3Dc");
  });

  it("adds a note with POST /api/timeline/events/:id/notes", async () => {
    await addTimelineNote(EVENT.id, "Need the certified copy.");
    expect(calls[0].url).toContain(`/api/timeline/events/${EVENT.id}/notes`);
    expect(JSON.parse(String(calls[0].init?.body))).toEqual({
      note: "Need the certified copy.",
    });
  });

  it("deletes a note with DELETE /api/timeline/events/:id/notes/:note_id", async () => {
    await deleteTimelineNote(EVENT.id, "99999999-0000-0000-0000-000000000000");
    expect(calls[0].url).toContain(
      `/api/timeline/events/${EVENT.id}/notes/99999999-0000-0000-0000-000000000000`,
    );
    expect(calls[0].init?.method).toBe("DELETE");
  });

  it("escapes an id in the path rather than letting it add a segment", async () => {
    await deleteTimelineEvent("a/b");
    expect(calls[0].url).toContain("/api/timeline/events/a%2Fb");
  });
});

describe("every write carries the standing timeout", () => {
  it("arms an abort signal rather than hanging forever", async () => {
    stubFetch({ ok: true, body: EVENT });
    await deleteTimelineEvent(EVENT.id);
    // `authFetch` creates the controller when the caller supplies no signal;
    // this proves the write path does not pass one of its own and defeat it.
    expect(calls[0].init?.signal).toBeDefined();
  });
});

describe("nothing is swallowed", () => {
  it("throws a sentence carrying the server's own message on a 422", async () => {
    // ⚑ This is what puts "no phase named 'apeals'" in front of the person who
    // typed it, instead of a button that quietly does nothing.
    stubFetch({
      ok: false,
      status: 422,
      body: { error: "unprocessable_entity", message: "no phase named 'apeals'" },
    });
    await expect(
      createTimelineEvent({ event_date: "2012-04-12", title: "x", phase: "apeals" }),
    ).rejects.toThrow(/apeals/);
  });

  it("names the status even when the body carries no message", async () => {
    stubFetch({ ok: false, status: 401, body: null });
    await expect(deleteTimelineEvent(EVENT.id)).rejects.toThrow(/401/);
  });

  it("turns a network failure into a sentence naming the action", async () => {
    stubFetch({ ok: true, throws: new Error("The user aborted a request.") });
    await expect(addTimelineNote(EVENT.id, "x")).rejects.toThrow(/note was not saved/);
  });

  it("refuses a 200 whose body is not an event", async () => {
    // A well-typed nothing. Without this the page would render a card with an
    // `undefined` title and look like a data problem rather than a contract one.
    stubFetch({ ok: true, body: { ok: true } });
    await expect(undeleteTimelineEvent(EVENT.id)).rejects.toThrow(/payload shape/);
  });

  it("fills in the four list fields an older payload might omit", async () => {
    stubFetch({ ok: true, body: { id: EVENT.id, title: "x" } });
    const written = await deleteTimelineEvent(EVENT.id);
    expect(written.links).toEqual([]);
    expect(written.tags).toEqual([]);
    expect(written.notes).toEqual([]);
    expect(written.history).toEqual([]);
  });
});

describe("the document picker's search", () => {
  it("asks the timeline's own endpoint, not the graph's document list", async () => {
    // ⚑ It must read the SAME table the link resolver reads, or an author could
    // pick a document the very next render marks "⚠ no document yet".
    stubFetch({ ok: true, body: { matches: [], total: 0, shown_limit: 20 } });
    await searchTimelineDocuments("tighe");
    expect(calls[0].url).toContain("/api/timeline/documents?q=tighe");
  });

  it("carries the total so a capped list can say so", async () => {
    const page = await stubbedSearch({
      matches: [{ id: "doc-a", title: "A" }],
      total: 40,
      shown_limit: 20,
    });
    expect(page.total).toBe(40);
    expect(page.matches).toHaveLength(1);
  });

  it("never claims a cap it was not told about", async () => {
    // An absent `total` falls back to what was shown, so the surface says
    // nothing rather than inventing a number of hidden matches.
    const page = await stubbedSearch({ matches: [{ id: "doc-a", title: "A" }] });
    expect(page.total).toBe(1);
  });

  it("throws rather than returning nothing when the search fails", async () => {
    // A picker that quietly returned nothing tells an author a document does
    // not exist when the truth is that nobody asked.
    stubFetch({ ok: false, status: 500, body: null });
    await expect(searchTimelineDocuments("tighe")).rejects.toThrow(/document search failed/i);
  });

  it("refuses a body with no matches array", async () => {
    stubFetch({ ok: true, body: { total: 3 } });
    await expect(searchTimelineDocuments("tighe")).rejects.toThrow(/payload shape/);
  });
});

/** Run one search against a stubbed body. */
async function stubbedSearch(body: unknown) {
  stubFetch({ ok: true, body });
  return searchTimelineDocuments("tighe");
}
