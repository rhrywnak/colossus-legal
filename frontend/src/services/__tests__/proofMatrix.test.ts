/**
 * Service + helper tests for the proof-matrix rollup client.
 *
 * `getProofMatrixRollup` exercises the four outcomes of the
 * GET /api/cases/:slug/proof-matrix/rollup client: a valid payload, and each of
 * the three throw paths (non-OK status, unparseable body, missing `counts`
 * array). Standing Rule 1 (no silent failures): every failure path produces a
 * distinct, observable error. Mocks `global.fetch` because `authFetch` calls it.
 * Pattern matches causesOfAction.test.ts.
 *
 * `indexAllegationTotals` is a pure re-keying helper — tested directly.
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  getProofMatrixRollup,
  indexAllegationTotals,
  type CountRollup,
  type ProofMatrixRollupResponse,
} from "../proofMatrix";

const validResponse: ProofMatrixRollupResponse = {
  case_slug: "awad_v_catholic_family_service",
  counts: [
    { count_number: 1, count_id: "count-1", deduped_allegations: 51 },
    { count_number: 2, count_id: "count-2", deduped_allegations: 41 },
    { count_number: 3, count_id: "count-3", deduped_allegations: 19 },
    { count_number: 4, count_id: "count-4", deduped_allegations: 34 },
  ],
};

describe("getProofMatrixRollup", () => {
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

    await expect(getProofMatrixRollup()).resolves.toEqual(validResponse);
  });

  it("throws with a 'structure not loaded' hint on 404", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 404,
      json: async () => {
        throw new Error("should not be called on non-OK");
      },
    });

    await expect(getProofMatrixRollup("missing_case")).rejects.toThrow(
      /Failed to load proof-matrix rollup for "missing_case" \(HTTP 404 — case structure not loaded/,
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

    await expect(getProofMatrixRollup()).rejects.toThrow(/was not valid JSON/);
  });

  it("throws when the counts array is missing", async () => {
    // @ts-ignore — valid JSON, wrong shape (no counts array)
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ case_slug: "x" }),
    });

    await expect(getProofMatrixRollup()).rejects.toThrow(
      /missing the "counts" array/,
    );
  });
});

describe("indexAllegationTotals", () => {
  it("re-keys rows by count_number to their deduped totals", () => {
    expect(indexAllegationTotals(validResponse.counts)).toEqual({
      1: 51,
      2: 41,
      3: 19,
      4: 34,
    });
  });

  it("returns an empty record for no rows", () => {
    expect(indexAllegationTotals([])).toEqual({});
  });

  it("uses the deduped total verbatim (no summing or transformation)", () => {
    const rows: CountRollup[] = [
      { count_number: 7, count_id: "count-7", deduped_allegations: 0 },
    ];
    // A real 0 is preserved as 0 — distinct from `undefined` (the pending state
    // the card renders as `—`).
    expect(indexAllegationTotals(rows)).toEqual({ 7: 0 });
  });
});
