/**
 * Service tests for the §7 card-payload client (task 1.2/1.3).
 *
 * Covers `fetchScenarioCards` against the right URL plus both throw paths.
 * Standing Rule 1 (no silent failures): a failed load and a shape mismatch each
 * produce a distinct, observable error rather than an empty queue — an empty
 * queue and a broken one look identical on screen, and they are very different
 * states. Mocks `global.fetch` because `authFetch` calls it. Pattern mirrors
 * scenarioGather.test.ts.
 */
import { afterEach, describe, expect, it, vi } from "vitest";

import { fetchScenarioCards, type ScenarioCardsResponse } from "../scenarioCards";

const SLUG = "awad_v_catholic_family_service";
const SCENARIO = "11111111-1111-1111-1111-111111111111";

afterEach(() => {
  vi.restoreAllMocks();
});

/** A minimal but complete card, matching the backend DTO. */
function card(): ScenarioCardsResponse["pool"][number] {
  return {
    code: "C-1",
    graph_node_id: "ev-1",
    quote: { text: "Yes.", context_before: "", context_after: "", question: null },
    pinpoint: {
      document_id: "doc-7",
      document_title: "CFS responses",
      label: "CFS responses at 26",
      page: 26,
      viewer_href: "/documents/doc-7?page=26&tab=document",
    },
    speaker: { name: "CFS", attribution: "extracted" },
    statement_kind: "admission",
    stance: null,
    bears_on: [],
    grounding: { state: "exact", label: "Grounded — found on the page" },
    confidence: { band: "unscored", label: "Not scored by a scan" },
    status: "undecided",
    status_label: "Not yet decided",
    defer_required: true,
    defer_required_reason: "This item is not linked to any accusation…",
    defer_reason: null,
  };
}

describe("fetchScenarioCards", () => {
  it("GETs the scenario-scoped cards URL and returns both lists", async () => {
    const response: ScenarioCardsResponse = { pool: [card()], set_aside: [] };
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => response,
    });
    // @ts-ignore
    global.fetch = fetchMock;

    await expect(fetchScenarioCards(SLUG, SCENARIO)).resolves.toEqual(response);

    const [url] = fetchMock.mock.calls[0];
    expect(url).toContain(`/api/cases/${SLUG}/scenarios/${SCENARIO}/facts/cards`);
  });

  it("throws with the scenario and status on a non-OK response", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({ ok: false, status: 500 });

    await expect(fetchScenarioCards(SLUG, SCENARIO)).rejects.toThrow(
      new RegExp(`candidate cards for scenario "${SCENARIO}".*\\(HTTP 500`),
    );
  });

  it("surfaces the backend message on a 4xx", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 404,
      json: async () => ({ message: "scenario not found" }),
    });

    await expect(fetchScenarioCards(SLUG, SCENARIO)).rejects.toThrow(/scenario not found/);
  });

  it("throws on a shape mismatch rather than returning an empty queue", async () => {
    // The failure this guard exists for: a backend change that renames a list
    // would otherwise render as "nothing to rule" — indistinguishable from a
    // finished pool, and the human would never know they had been shown nothing.
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ pool: [card()] }),
    });

    await expect(fetchScenarioCards(SLUG, SCENARIO)).rejects.toThrow(
      /missing pool\/set_aside — backend\/frontend contract mismatch/,
    );
  });

  it("accepts an empty pool as a valid, distinct answer", async () => {
    // An empty pool is a real state (every candidate ruled), and it must NOT
    // throw — only a malformed shape does.
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ pool: [], set_aside: [] }),
    });

    await expect(fetchScenarioCards(SLUG, SCENARIO)).resolves.toEqual({
      pool: [],
      set_aside: [],
    });
  });
});
