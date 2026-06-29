/**
 * Service tests for the scenario CRUD client.
 *
 * `createScenario` exercises: a valid create (right method/URL/body + parsed
 * response), a non-2xx that surfaces the backend's field-named message, and an
 * unparseable body. `listScenarios` exercises the happy path. Mocks
 * `global.fetch` because `authFetch` calls it. Pattern mirrors trialPrep.test.ts.
 */
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  createScenario,
  listScenarios,
  type ScenarioCreatePayload,
  type ScenarioDto,
} from "../scenarioCrud";

const SLUG = "awad_v_catholic_family_service";

const validDto: ScenarioDto = {
  scenario_id: "00000000-0000-0000-0000-000000000000",
  name: "Marie is obstructive and uncooperative",
  direction: "defense",
  status: "draft",
  case_slug: SLUG,
  feeds_count_id: null,
  anchor_allegation_ids: ["doc-x:allegation:abc"],
  definition: {},
};

describe("createScenario", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("POSTs the payload to the right URL and parses a well-formed response", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 201,
      json: async () => validDto,
    });
    // @ts-ignore — minimal mock of the fetch Response we use
    global.fetch = fetchMock;

    const payload: ScenarioCreatePayload = {
      name: "Marie is obstructive and uncooperative",
      direction: "defense",
      status: "draft",
      anchor_allegation_ids: ["doc-x:allegation:abc"],
    };

    await expect(createScenario(SLUG, payload)).resolves.toEqual(validDto);

    // The request used POST, the slug-scoped URL, and the exact JSON body.
    const [url, options] = fetchMock.mock.calls[0];
    expect(url).toContain(`/api/cases/${SLUG}/scenarios`);
    expect(options.method).toBe("POST");
    expect(JSON.parse(options.body)).toEqual(payload);
  });

  it("throws with the status and the backend's message on a non-2xx", async () => {
    // @ts-ignore — the BadRequest body the backend returns on validation failure
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 400,
      json: async () => ({
        error: "validation_error",
        message: "name must not be empty",
        details: { field: "name" },
      }),
    });

    await expect(
      createScenario(SLUG, { name: "", direction: "defense" }),
    ).rejects.toThrow(/HTTP 400.*name must not be empty/);
  });

  it("throws when the body is not valid JSON", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 201,
      json: async () => {
        throw new Error("Unexpected token < in JSON");
      },
    });

    await expect(
      createScenario(SLUG, { name: "x", direction: "offense" }),
    ).rejects.toThrow(/was not valid JSON/);
  });

  it("throws on a contract mismatch (response missing load-bearing fields)", async () => {
    // @ts-ignore — valid JSON, wrong shape (no scenario_id / anchor_allegation_ids)
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 201,
      json: async () => ({ name: "x" }),
    });

    await expect(
      createScenario(SLUG, { name: "x", direction: "offense" }),
    ).rejects.toThrow(/contract mismatch/);
  });
});

describe("listScenarios", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("returns the array of scenarios on success", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => [validDto],
    });

    await expect(listScenarios(SLUG)).resolves.toEqual([validDto]);
  });

  it("throws with the status on a non-2xx", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      json: async () => {
        throw new Error("should not be called on non-OK");
      },
    });

    await expect(listScenarios(SLUG)).rejects.toThrow(/HTTP 500/);
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

    await expect(listScenarios(SLUG)).rejects.toThrow(/was not valid JSON/);
  });

  it("throws when the body is not an array", async () => {
    // @ts-ignore — valid JSON, wrong shape
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ not: "an array" }),
    });

    await expect(listScenarios(SLUG)).rejects.toThrow(/was not an array/);
  });
});
