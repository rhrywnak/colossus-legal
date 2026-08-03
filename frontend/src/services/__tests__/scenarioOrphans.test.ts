/**
 * Service tests for the orphan-strip client (task 1.7C, defect D9).
 *
 * Standing Rule 1 carries unusual weight on this endpoint: it reports how many
 * curated rulings have dead graph pointers — 26 on DEV today, six of them human
 * INCLUDE decisions. A failure that resolved to an empty payload would render as
 * "nothing was lost", which is the one thing this surface must never say by
 * accident. So both throw paths are pinned here.
 *
 * Mocks `global.fetch` because `authFetch` calls it. Pattern mirrors
 * scenarioFacts.test.ts.
 */
import { afterEach, describe, expect, it, vi } from "vitest";

import { fetchScenarioOrphans, type ScenarioOrphansDto } from "../scenarioOrphans";

const SLUG = "awad_v_catholic_family_service";
const SCENARIO = "11111111-1111-1111-1111-111111111111";

/** The URL the strip reads, for assertions. */
const ORPHANS_URL = `/api/cases/${SLUG}/scenarios/${SCENARIO}/facts/orphans`;

/** The measured DEV split: 16 / 8 / 2, the third with no recoverable title. */
function liveResponse(): ScenarioOrphansDto {
  return {
    total: 26,
    groups: [
      {
        document_id: "doc-george-phillips-response-to-discovery",
        label: "GEORGE PHILLIPS RESPONSE TO DISCOVERY",
        title_state: "resolved",
        count: 16,
      },
      {
        document_id: "doc-cfs-interrogatory-response-08-08-16",
        label: "CFS INTERROGATORY RESPONSE 08 08 16",
        title_state: "resolved",
        count: 8,
      },
      {
        document_id: "doc-court-of-appeals-ruling-01-12-2012",
        label: "court-of-appeals-ruling-01-12-2012",
        title_state: "unrecoverable",
        count: 2,
      },
    ],
  };
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("fetchScenarioOrphans", () => {
  it("GETs the orphans URL and resolves the grouped payload", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => liveResponse(),
    });
    // @ts-ignore — minimal fetch mock
    global.fetch = fetchMock;

    const data = await fetchScenarioOrphans(SLUG, SCENARIO);

    expect(fetchMock.mock.calls[0][0]).toContain(ORPHANS_URL);
    expect(data.total).toBe(26);
    expect(data.groups.map((g) => g.count)).toEqual([16, 8, 2]);
    // The grouping and every label arrive COMPOSED — the client parses no ids and
    // invents no titles (§2.8).
    expect(data.groups[2].title_state).toBe("unrecoverable");
    expect(data.groups[2].label).toBe("court-of-appeals-ruling-01-12-2012");
  });

  it("throws with the scenario id and status on a non-OK response", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({ ok: false, status: 500 });

    await expect(fetchScenarioOrphans(SLUG, SCENARIO)).rejects.toThrow(
      new RegExp(`scenario "${SCENARIO}" \\(HTTP 500`),
    );
  });

  it("explains WHY a failure here matters rather than failing quietly", async () => {
    // The message is the strip's last line of defence: if it renders at all, the
    // human must understand that saved rulings may be unaccounted for.
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({ ok: false, status: 503 });

    await expect(fetchScenarioOrphans(SLUG, SCENARIO)).rejects.toThrow(
      /not skipped silently/,
    );
  });

  it("throws a CONTRACT-MISMATCH error on a 200 with the wrong shape", async () => {
    // A 2xx carrying the wrong body is the dangerous case: it would otherwise flow
    // into the component and blow up as `undefined.map` far from the cause. Throw
    // here, naming the scenario and the missing fields.
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ groups: [] }), // `total` missing
    });

    await expect(fetchScenarioOrphans(SLUG, SCENARIO)).rejects.toThrow(
      /missing total\/groups/,
    );
  });

  it("rejects a payload whose groups is not an array", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ total: 26, groups: null }),
    });

    await expect(fetchScenarioOrphans(SLUG, SCENARIO)).rejects.toThrow(
      /contract\s+mismatch/,
    );
  });

  it("accepts a genuinely empty strip — nothing orphaned is good news", async () => {
    // Distinct from a failure (Standing Rule 1): zero orphans is a real, and very
    // welcome, state. It must NOT be mistaken for a contract mismatch.
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ total: 0, groups: [] }),
    });

    const data = await fetchScenarioOrphans(SLUG, SCENARIO);
    expect(data.total).toBe(0);
    expect(data.groups).toEqual([]);
  });

  it("encodes both path parameters", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ total: 0, groups: [] }),
    });
    // @ts-ignore
    global.fetch = fetchMock;

    await fetchScenarioOrphans("slug with spaces", "id/with/slashes");

    const url = fetchMock.mock.calls[0][0] as string;
    expect(url).toContain("slug%20with%20spaces");
    expect(url).toContain("id%2Fwith%2Fslashes");
  });
});
