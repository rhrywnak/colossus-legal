/**
 * Service tests for getCaseHeader.
 *
 * Exercises the four outcomes of the GET /api/cases/:slug client: a valid
 * payload, and each of the three throw paths (non-OK status, unparseable body,
 * shape mismatch). Standing Rule 1 (no silent failures): every failure path
 * must produce a distinct, observable error. Mocks `global.fetch` because
 * `authFetch` calls it under the hood. Pattern matches `claims.test.ts`.
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { getCaseHeader, type CaseHeaderResponse } from "../caseHeader";

// A minimal-but-valid payload: the load-bearing fields the validator checks
// (display_title, court, parties) are present and correctly typed.
const validResponse: CaseHeaderResponse = {
  case_id: "case-awad-v-cfs-phillips",
  case_slug: "awad_v_catholic_family_service",
  display_title: "Awad v. Catholic Family Service",
  display_title_full: null,
  court: {
    name: "Bay County Circuit Court",
    jurisdiction: "Michigan",
    case_number: null,
    filed_date: "2013-11-01",
    transferred_from: null,
    transfer_date: null,
  },
  status: "active",
  complaint_document_id: null,
  parties: { plaintiffs: [], active_defendants: [], dropped_defendants: [] },
  counsel: [],
};

describe("getCaseHeader", () => {
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

    await expect(getCaseHeader()).resolves.toEqual(validResponse);
  });

  it("throws with the slug and a 404 hint when the case is not found", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 404,
      json: async () => {
        throw new Error("should not be called on non-OK");
      },
    });

    await expect(getCaseHeader("missing_case")).rejects.toThrow(
      /Failed to load case "missing_case" \(HTTP 404 — case not found\)/,
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

    await expect(getCaseHeader()).rejects.toThrow(/was not valid JSON/);
  });

  it("throws when required fields are missing from the body", async () => {
    // @ts-ignore — valid JSON, wrong shape (no display_title/court/parties)
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ unexpected: "shape" }),
    });

    await expect(getCaseHeader()).rejects.toThrow(/missing required fields/);
  });
});
