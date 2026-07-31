/**
 * Service tests for the candidate-workbench client (Phase 1a.6).
 *
 * Covers `gatherCandidates` (GET) and `applyFactAction` (POST) against the right
 * URLs, plus their throw paths. Standing Rule 1 (no silent failures): every
 * non-OK / unparseable / wrong-shape response produces a distinct, observable
 * error. Mocks `global.fetch` because `authFetch` calls it. Pattern mirrors
 * scenarioFacts.test.ts / scenarioCrud.test.ts.
 */
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  applyFactAction,
  gatherCandidates,
  type GatherCandidatesResponse,
} from "../scenarioGather";

const SLUG = "awad_v_catholic_family_service";
const SCENARIO = "11111111-1111-1111-1111-111111111111";
const NODE = "ev-42";

const FACTS_URL = `/api/cases/${SLUG}/scenarios/${SCENARIO}/facts`;

afterEach(() => {
  vi.restoreAllMocks();
});

describe("gatherCandidates", () => {
  const response: GatherCandidatesResponse = {
    pool: [
      {
        content: {
          evidence_id: NODE,
          title: "A statement",
          pattern_tags: [],
          about: [],
        },
        status: "undecided",
        role: null,
        confidence: null,
        ordinal: 1,
        note: null,
      },
    ],
    dropped: [],
  };

  it("GETs the gather URL and returns the typed {pool, dropped} on 200", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => response,
    });
    // @ts-ignore — minimal fetch mock
    global.fetch = fetchMock;

    await expect(gatherCandidates(SLUG, SCENARIO)).resolves.toEqual(response);
    expect(fetchMock.mock.calls[0][0]).toContain(`${FACTS_URL}/gather`);
  });

  it("throws with the scenario id and status on a non-OK response", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({ ok: false, status: 503 });

    await expect(gatherCandidates(SLUG, SCENARIO)).rejects.toThrow(
      new RegExp(`scenario "${SCENARIO}" \\(HTTP 503`),
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

    await expect(gatherCandidates(SLUG, SCENARIO)).rejects.toThrow(
      /was not valid JSON/,
    );
  });

  it("throws when pool/dropped are missing (contract mismatch)", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ pool: [] }), // dropped missing
    });

    await expect(gatherCandidates(SLUG, SCENARIO)).rejects.toThrow(
      /missing the pool\/dropped lists/,
    );
  });
});

describe("applyFactAction", () => {
  it("POSTs the action to the node-scoped action URL and resolves on 200", async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, status: 200 });
    // @ts-ignore
    global.fetch = fetchMock;

    await expect(
      applyFactAction(SLUG, SCENARIO, NODE, "drop"),
    ).resolves.toBeUndefined();

    const [url, options] = fetchMock.mock.calls[0];
    expect(url).toContain(`${FACTS_URL}/${NODE}/action`);
    expect(options.method).toBe("POST");
    expect(JSON.parse(options.body)).toEqual({ action: "drop" });
  });

  it("throws with the action and status on a non-OK response", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({ ok: false, status: 404 });

    await expect(
      applyFactAction(SLUG, SCENARIO, NODE, "undrop"),
    ).rejects.toThrow(new RegExp(`undrop fact "${NODE}" .* \\(HTTP 404`));
  });

  it("surfaces the backend message on a 4xx", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 400,
      json: async () => ({ message: "unknown action token" }),
    });

    await expect(
      applyFactAction(SLUG, SCENARIO, NODE, "include"),
    ).rejects.toThrow(/unknown action token/);
  });
});
