/**
 * Service tests for getCausesOfAction.
 *
 * Exercises the four outcomes of the GET /api/cases/:slug/causes-of-action
 * client: a valid payload, and each of the three throw paths (non-OK status,
 * unparseable body, missing `counts` array, missing `matrix_wording`). Standing
 * Rule 1 (no silent failures): every failure path produces a distinct,
 * observable error. Mocks
 * `global.fetch` because `authFetch` calls it. Pattern matches caseHeader.test.ts.
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  getCausesOfAction,
  type CausesOfActionResponse,
} from "../causesOfAction";
import { MATRIX_WORDING_FIXTURE } from "../../testFixtures/matrixWording";

const validResponse: CausesOfActionResponse = {
  case_slug: "awad_v_catholic_family_service",
  counts: [],
  matrix_wording: MATRIX_WORDING_FIXTURE,
};

describe("getCausesOfAction", () => {
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

    await expect(getCausesOfAction()).resolves.toEqual(validResponse);
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

    await expect(getCausesOfAction("missing_case")).rejects.toThrow(
      /Failed to load causes of action for "missing_case" \(HTTP 404 — case structure not loaded/,
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

    await expect(getCausesOfAction()).rejects.toThrow(/was not valid JSON/);
  });

  it("throws when the counts array is missing", async () => {
    // @ts-ignore — valid JSON, wrong shape (no counts array)
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ case_slug: "x" }),
    });

    await expect(getCausesOfAction()).rejects.toThrow(/missing the "counts" array/);
  });

  /**
   * A payload with no wording block is a REFUSAL, not a page that renders a
   * blank column header.
   *
   * The matrix's words come from the settings store; there is deliberately no
   * fallback vocabulary in the client to draw a heading from, because inventing
   * one would put a sentence on screen that nobody chose (the language law).
   */
  it("throws when the payload carries no matrix wording", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ case_slug: "awad_v_catholic_family_service", counts: [] }),
    });

    await expect(getCausesOfAction()).rejects.toThrow(/matrix_wording/);
  });
});
