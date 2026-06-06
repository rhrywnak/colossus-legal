/**
 * Service tests for the proof-review client.
 *
 * `getProofReview` exercises the four outcomes of the
 * GET /api/cases/:slug/proof-review client: a valid payload and each throw path
 * (non-OK status, unparseable body, missing section). Standing Rule 1: every
 * failure produces a distinct, observable error. Also pins the URL construction
 * for the optional `?document_id=` filter (an absent filter must NOT append the
 * query param — it maps to the backend's "all documents" branch). Mocks
 * `global.fetch` because `authFetch` calls it. Pattern matches proofMatrix.test.ts.
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  getProofReview,
  type ProofReviewResponse,
} from "../proofReview";

const validResponse: ProofReviewResponse = {
  case_slug: "awad_v_catholic_family_service",
  document_id: null,
  summary: {
    corroborating: {
      total: 1,
      by_statement_type: [{ statement_type: "admission", count: 1 }],
      by_category: [
        { statement_type: "admission", evidence_strength: "sworn_party_admission", count: 1 },
      ],
    },
    excluded: { total: 0, by_statement_type: [] },
  },
  proof_edges: [],
  excluded: [],
  borderline: [],
};

describe("getProofReview", () => {
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

    await expect(getProofReview()).resolves.toEqual(validResponse);
  });

  it("omits the query string when no document filter is given", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => validResponse,
    });
    // @ts-ignore
    global.fetch = fetchMock;

    await getProofReview("case_x");
    const calledUrl = fetchMock.mock.calls[0][0] as string;
    expect(calledUrl).toContain("/api/cases/case_x/proof-review");
    expect(calledUrl).not.toContain("document_id");
  });

  it("appends ?document_id= when a document filter is given", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => validResponse,
    });
    // @ts-ignore
    global.fetch = fetchMock;

    await getProofReview("case_x", "doc-george");
    const calledUrl = fetchMock.mock.calls[0][0] as string;
    expect(calledUrl).toContain("document_id=doc-george");
  });

  it("throws with a 'no proof-review data' hint on 404", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 404,
      json: async () => {
        throw new Error("should not be called on non-OK");
      },
    });

    await expect(getProofReview("missing_case")).rejects.toThrow(
      /Failed to load proof review for "missing_case" \(HTTP 404 — no proof-review data/,
    );
  });

  it("throws a bare HTTP error (no 404 hint) on a non-404 status like 500", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      json: async () => {
        throw new Error("should not be called on non-OK");
      },
    });

    // The generic branch (backend crash / gateway error) must be distinct from
    // the 404 "no data" branch — same status surfaced, but no 404 suffix.
    let message = "";
    try {
      await getProofReview("case_x");
    } catch (e) {
      message = (e as Error).message;
    }
    expect(message).toMatch(/HTTP 500/);
    expect(message).not.toContain("no proof-review data");
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

    await expect(getProofReview()).rejects.toThrow(/was not valid JSON/);
  });

  it("throws when a required section is missing", async () => {
    // @ts-ignore — valid JSON, wrong shape (summary present, proof_edges absent)
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ case_slug: "x", summary: validResponse.summary }),
    });

    await expect(getProofReview()).rejects.toThrow(/missing a required section/);
  });
});
