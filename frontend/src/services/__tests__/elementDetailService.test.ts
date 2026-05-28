/**
 * Service tests for fetchElementDetail and saveElementNotes.
 *
 * Exercises every observable outcome of the two clients:
 *  - fetchElementDetail: valid payload, 404, unparseable body, missing
 *    `allegations` array (shape mismatch).
 *  - saveElementNotes: PATCH success, PATCH non-2xx (404 + generic 500),
 *    and the wire-distinct behavior of null vs "" in the request body
 *    (so the backend's "clear column" vs "write empty string" semantics
 *    survive on the wire — Rule 1, distinguishable observables).
 *
 * Mocks `global.fetch` because `authFetch` ultimately calls it. Pattern
 * matches causesOfAction.test.ts.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  fetchElementDetail,
  saveElementNotes,
  type ElementDetailResponse,
} from "../elementDetailService";

const validResponse: ElementDetailResponse = {
  element_id: "element-1-1",
  element_name: "Existence of fiduciary duty",
  what_plaintiff_must_prove: "That a fiduciary relationship existed.",
  order_in_count: 1,
  count_number: 1,
  count_name: "Breach of fiduciary duty",
  review_notes: null,
  allegations: [],
  allegation_count: 0,
  common_count: 0,
  dedicated_count: 0,
};

describe("fetchElementDetail", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("returns the typed payload when the response is valid", async () => {
    // @ts-ignore — minimal mock of the fetch Response we use
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => validResponse,
    });

    await expect(
      fetchElementDetail("awad_v_catholic_family_service", "element-1-1"),
    ).resolves.toEqual(validResponse);
  });

  it("throws with an Element-not-found hint on 404", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 404,
      json: async () => {
        throw new Error("should not be called on non-OK");
      },
    });

    await expect(
      fetchElementDetail("awad_v_catholic_family_service", "ghost-id"),
    ).rejects.toThrow(/Failed to load Element detail for "ghost-id" \(HTTP 404 — no Element with that id/);
  });

  it("throws with status code on generic non-2xx (e.g. 500) without the 404 hint", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      json: async () => {
        throw new Error("should not be called on non-OK");
      },
    });

    // Distinct observable from the 404 branch: the message must carry the
    // status code and must NOT include the "no Element with that id" hint
    // (which is reserved for the 404 case).
    const call = fetchElementDetail("awad_v_catholic_family_service", "element-1-1");
    await expect(call).rejects.toThrow(/HTTP 500/);
    await expect(call).rejects.toThrow(
      /^(?!.*no Element with that id).*Failed to load Element detail for "element-1-1"/,
    );
  });

  it("throws when the body is not valid JSON", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => {
        throw new Error("Unexpected token < in JSON");
      },
    });

    await expect(
      fetchElementDetail("awad_v_catholic_family_service", "element-1-1"),
    ).rejects.toThrow(/was not valid JSON/);
  });

  it("throws when the allegations array is missing", async () => {
    // @ts-ignore — valid JSON, wrong shape (no allegations array)
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ element_id: "element-1-1" }),
    });

    await expect(
      fetchElementDetail("awad_v_catholic_family_service", "element-1-1"),
    ).rejects.toThrow(/missing the "allegations" array/);
  });

  it("URL-encodes the slug and element id", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => validResponse,
    });
    // @ts-ignore
    global.fetch = fetchMock;

    await fetchElementDetail("awad v cfs", "element/1-1");

    const url = fetchMock.mock.calls[0][0] as string;
    expect(url).toContain("awad%20v%20cfs");
    expect(url).toContain("element%2F1-1");
  });
});

describe("saveElementNotes", () => {
  let fetchMock: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ status: "saved" }),
    });
    // @ts-ignore
    global.fetch = fetchMock;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("PATCHes with method, JSON content type, and the notes body", async () => {
    await saveElementNotes("awad_v_catholic_family_service", "element-1-1", "Hello");

    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toContain("/elements/element-1-1/notes");
    expect(init.method).toBe("PATCH");
    expect((init.headers as Record<string, string>)["Content-Type"]).toBe(
      "application/json",
    );
    expect(init.body).toBe(JSON.stringify({ review_notes: "Hello" }));
  });

  it("sends explicit null to clear notes (distinct from empty string)", async () => {
    await saveElementNotes("awad_v_catholic_family_service", "element-1-1", null);
    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(init.body).toBe(JSON.stringify({ review_notes: null }));
  });

  it("sends an explicit empty string when the user saves empty notes", async () => {
    await saveElementNotes("awad_v_catholic_family_service", "element-1-1", "");
    const [, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(init.body).toBe(JSON.stringify({ review_notes: "" }));
  });

  it("throws on 404 with a row-missing hint", async () => {
    fetchMock.mockResolvedValueOnce({ ok: false, status: 404 });
    await expect(
      saveElementNotes("awad_v_catholic_family_service", "ghost-id", "x"),
    ).rejects.toThrow(/HTTP 404 — Element row not in authored_entities/);
  });

  it("throws on generic non-2xx with the status code", async () => {
    fetchMock.mockResolvedValueOnce({ ok: false, status: 500 });
    await expect(
      saveElementNotes("awad_v_catholic_family_service", "element-1-1", "x"),
    ).rejects.toThrow(/HTTP 500/);
  });
});
