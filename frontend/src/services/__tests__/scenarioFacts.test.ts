/**
 * Service tests for the scenario fact-curation client (Phase A).
 *
 * Covers the three calls — add (POST), remove (DELETE), list (GET) — against
 * the right URLs, plus their throw paths. Standing Rule 1 (no silent failures):
 * every non-OK / unparseable / wrong-shape response produces a distinct,
 * observable error. Mocks `global.fetch` because `authFetch` calls it. Pattern
 * mirrors trialPrep.test.ts.
 */
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  addScenarioFact,
  listScenarioFacts,
  removeScenarioFact,
  type ScenarioFactDto,
} from "../scenarioFacts";

const SLUG = "awad_v_catholic_family_service";
const SCENARIO = "11111111-1111-1111-1111-111111111111";
const NODE = "ev-42";

/** The URL the facts collection lives at, for assertions. */
const FACTS_URL = `/api/cases/${SLUG}/scenarios/${SCENARIO}/facts`;

afterEach(() => {
  vi.restoreAllMocks();
});

describe("addScenarioFact", () => {
  it("POSTs the node id to the facts URL and resolves on 201", async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, status: 201 });
    // @ts-ignore — minimal fetch mock
    global.fetch = fetchMock;

    await expect(
      addScenarioFact(SLUG, SCENARIO, { graph_node_id: NODE }),
    ).resolves.toBeUndefined();

    const [url, options] = fetchMock.mock.calls[0];
    expect(url).toContain(FACTS_URL);
    expect(options.method).toBe("POST");
    expect(JSON.parse(options.body)).toEqual({ graph_node_id: NODE });
  });

  it("throws with the scenario id and status on a non-OK response", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({ ok: false, status: 404 });

    await expect(
      addScenarioFact(SLUG, SCENARIO, { graph_node_id: NODE }),
    ).rejects.toThrow(new RegExp(`scenario "${SCENARIO}" \\(HTTP 404`));
  });
});

describe("removeScenarioFact", () => {
  it("DELETEs the node-scoped URL and resolves on 204", async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, status: 204 });
    // @ts-ignore
    global.fetch = fetchMock;

    await expect(
      removeScenarioFact(SLUG, SCENARIO, NODE),
    ).resolves.toBeUndefined();

    const [url, options] = fetchMock.mock.calls[0];
    expect(url).toContain(`${FACTS_URL}/${NODE}`);
    expect(options.method).toBe("DELETE");
  });

  it("throws on a non-OK response (e.g. the fact was already gone)", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({ ok: false, status: 404 });

    await expect(removeScenarioFact(SLUG, SCENARIO, NODE)).rejects.toThrow(
      /HTTP 404/,
    );
  });
});

describe("listScenarioFacts", () => {
  const facts: ScenarioFactDto[] = [
    {
      graph_node_id: NODE,
      role: null,
      note: null,
      content: {
        evidence_id: NODE,
        title: "A statement",
        pattern_tags: ["disparagement"],
        about: [],
      },
    },
  ];

  it("GETs the facts URL and returns the typed array on 200", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => facts,
    });
    // @ts-ignore
    global.fetch = fetchMock;

    await expect(listScenarioFacts(SLUG, SCENARIO)).resolves.toEqual(facts);
    expect(fetchMock.mock.calls[0][0]).toContain(FACTS_URL);
  });

  it("returns an empty array for a scenario with no saved facts", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => [],
    });

    await expect(listScenarioFacts(SLUG, SCENARIO)).resolves.toEqual([]);
  });

  it("throws with the scenario id and status on a non-OK response", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({ ok: false, status: 404 });

    await expect(listScenarioFacts(SLUG, SCENARIO)).rejects.toThrow(
      new RegExp(`scenario "${SCENARIO}" \\(HTTP 404`),
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

    await expect(listScenarioFacts(SLUG, SCENARIO)).rejects.toThrow(
      /was not valid JSON/,
    );
  });

  it("throws when the payload is not a list", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ not: "an array" }),
    });

    await expect(listScenarioFacts(SLUG, SCENARIO)).rejects.toThrow(
      /was not a list/,
    );
  });
});
