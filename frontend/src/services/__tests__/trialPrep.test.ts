/**
 * Service tests for the Trial Prep ("War Room") dashboard client.
 *
 * `getTrialPrepDashboard` exercises the four outcomes of the
 * GET /api/cases/:slug/trial-prep/dashboard client: a valid payload, and each of
 * the three throw paths (non-OK status, unparseable body, missing
 * `scenarios`/`metrics`). Standing Rule 1 (no silent failures): every failure
 * path produces a distinct, observable error. Mocks `global.fetch` because
 * `authFetch` calls it. Pattern mirrors proofMatrix.test.ts.
 */
import { afterEach, describe, expect, it, vi } from "vitest";

import type {
  ScenarioDetail,
  TrialPrepDashboard,
} from "../../pages/trialPrepData";
import { getScenarioDetailLive, getTrialPrepDashboard } from "../trialPrep";

const SLUG = "awad_v_catholic_family_service";

// A minimal-but-valid payload: the two load-bearing fields the client validates
// (`metrics` present, `scenarios` an array) plus the one live card.
const validResponse: TrialPrepDashboard = {
  metrics: {
    scenarios: 5,
    ready: 1,
    drafted_or_review: 3,
    instances: 16,
    baseless_repeat_patterns: 1,
    no_response_yet: 1,
  },
  alerts: [{ message: "an alert" }],
  scenarios: [
    {
      id: "marie-obstructive",
      attack: "Marie is obstructive and uncooperative",
      status: "draft",
      instance_count: 4,
      response_count: 1,
      speakers: ["CFS", "George Phillips"],
      baseless_repeat_count: 3,
    },
  ],
};

describe("getTrialPrepDashboard", () => {
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

    await expect(getTrialPrepDashboard()).resolves.toEqual(validResponse);
  });

  it("throws with the slug and status on a non-OK response", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      json: async () => {
        throw new Error("should not be called on non-OK");
      },
    });

    await expect(getTrialPrepDashboard("missing_case")).rejects.toThrow(
      /Failed to load the Trial Prep dashboard for "missing_case" \(HTTP 500/,
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

    await expect(getTrialPrepDashboard()).rejects.toThrow(/was not valid JSON/);
  });

  it("throws when scenarios/metrics are missing (contract mismatch)", async () => {
    // @ts-ignore — valid JSON, wrong shape (no scenarios array, no metrics)
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ alerts: [] }),
    });

    await expect(getTrialPrepDashboard()).rejects.toThrow(
      /missing "scenarios"\/"metrics"/,
    );
  });
});

// A minimal-but-valid detail payload: the two load-bearing fields the client
// validates (`attack` string, `timeline` array) plus one evidence turn.
const validDetail: ScenarioDetail = {
  id: "00000000-0000-0000-0000-000000000000",
  attack: "Marie is obstructive and uncooperative",
  status: "draft",
  pattern_summary: null,
  timeline: [
    {
      kind: "evidence",
      grounded: true,
      speaker: "George Phillips",
      date: null,
      text: "the quote",
      relationship_type: "rebuts",
      source_document: "doc-x",
      page_number: 54,
      paragraph: "¶54",
      repeated_after_rebuttal: false,
    },
  ],
  responses: [],
  notes: null,
};

describe("getScenarioDetailLive", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("returns the typed detail when the response is valid", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => validDetail,
    });

    await expect(getScenarioDetailLive(SLUG, "abc")).resolves.toEqual(
      validDetail,
    );
  });

  it("returns null on a 404 (deleted/unknown scenario)", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 404,
      json: async () => ({ error: "scenario not found" }),
    });

    await expect(getScenarioDetailLive(SLUG, "abc")).resolves.toBeNull();
  });

  it("throws with the status on a non-404 non-OK response", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      json: async () => {
        throw new Error("should not be called on non-OK");
      },
    });

    await expect(getScenarioDetailLive(SLUG, "abc")).rejects.toThrow(/HTTP 500/);
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

    await expect(getScenarioDetailLive(SLUG, "abc")).rejects.toThrow(
      /was not valid JSON/,
    );
  });

  it("throws on a contract mismatch (missing attack/timeline)", async () => {
    // @ts-ignore — valid JSON, wrong shape
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ id: "x" }),
    });

    await expect(getScenarioDetailLive(SLUG, "abc")).rejects.toThrow(
      /missing "attack"\/"timeline"/,
    );
  });
});
