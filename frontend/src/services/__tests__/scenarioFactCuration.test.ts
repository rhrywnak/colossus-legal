/**
 * Service tests for the two curation writes (task 2.13 slice 1).
 *
 * ## Why this file exists, written after the defect it would have caught
 *
 * The first build of this client requested `/cases/…` instead of `/api/cases/…`.
 * Every unit test passed, the backend routes were correct, the types were
 * correct — and on DEV both writes died with `405 Method Not Allowed`, because
 * the un-prefixed path fell through to the SPA's own routes, which answer GET
 * and not POST. The method error pointed away from the real cause, and the whole
 * feature was unusable.
 *
 * Nothing but an assertion on the URL string can catch that: it is a string this
 * module builds, and no compiler or backend contract constrains it.
 * `scenarioCards.test.ts` already had this guard; this module shipped without
 * it. So the URL shape is now the FIRST thing asserted for both writes.
 *
 * Mocks `global.fetch` because `authFetch` calls it — same pattern as the
 * sibling service tests.
 */
import { afterEach, describe, expect, it, vi } from "vitest";

import { API_BASE_URL } from "../api";
import { setFactOrder, setFactTier } from "../scenarioFactCuration";

const SLUG = "awad_v_catholic_family_service";
const SCENARIO = "11111111-1111-1111-1111-111111111111";
const NODE = "doc-cfs:evidence:6f4a2719";

/** The path both writes must build, up to the trailing verb.
 *
 * Built from `API_BASE_URL` for the same reason the client is: the origin is
 * environment-dependent (empty in the browser, a localhost origin under test),
 * and hardcoding it here would make this file pass or fail for a reason that has
 * nothing to do with the path. The `/api` segment is the part under test. */
const STEM =
  `${API_BASE_URL}/api/cases/${SLUG}/scenarios/${SCENARIO}` +
  `/facts/${encodeURIComponent(NODE)}`;

afterEach(() => {
  vi.restoreAllMocks();
});

/** A fetch mock that succeeds, returning the calls for inspection. */
function okFetch() {
  const fetchMock = vi.fn().mockResolvedValue({ ok: true, status: 200 });
  global.fetch = fetchMock;
  return fetchMock;
}

describe("setFactTier", () => {
  it("POSTs to the /api-prefixed tier path", async () => {
    const fetchMock = okFetch();

    await setFactTier(SLUG, SCENARIO, NODE, "carries");

    const [url, init] = fetchMock.mock.calls[0];
    // THE regression guard. A missing `/api` is a 405 on DEV and green here
    // without this line.
    expect(url).toContain("/api/cases/");
    expect(url).toBe(`${STEM}/tier`);
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body)).toEqual({ tier: "carries" });
  });

  it("encodes a node id containing colons", async () => {
    // Graph node ids are `doc-…:evidence:…`. An unencoded colon changes the
    // path the router sees, and the write lands somewhere else or nowhere.
    const fetchMock = okFetch();
    await setFactTier(SLUG, SCENARIO, NODE, "backup");
    const [url] = fetchMock.mock.calls[0];
    expect(url).toContain("%3Aevidence%3A");
    expect(url).not.toContain(":evidence:");
  });

  it("throws with the status and the backend's words on a non-2xx", async () => {
    // Standing Rule 1: a write that did not land must never look like one that
    // did. The message carries the status because that is what told us the path
    // was wrong — a bare "could not save" would have hidden the 405.
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 404,
      json: async () => ({ message: "that fact is not in this scenario" }),
    });

    await expect(setFactTier(SLUG, SCENARIO, NODE, "carries")).rejects.toThrow(/404/);
  });
});

describe("setFactOrder", () => {
  it("POSTs to the /api-prefixed order path", async () => {
    const fetchMock = okFetch();

    await setFactOrder(SLUG, SCENARIO, NODE, "ev-a", "ev-b");

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toContain("/api/cases/");
    expect(url).toBe(`${STEM}/order`);
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body)).toEqual({ after: "ev-a", before: "ev-b" });
  });

  it("omits a neighbour that is absent rather than sending null", async () => {
    // The backend body is `deny_unknown_fields` with `#[serde(default)]` on both
    // neighbours, so an omitted key and an explicit null mean the same thing
    // there. Omitting keeps a drop at the top or bottom readable in a network
    // log, which is where this class of bug is actually found.
    const fetchMock = okFetch();

    await setFactOrder(SLUG, SCENARIO, NODE, null, "ev-b");
    expect(JSON.parse(fetchMock.mock.calls[0][1].body)).toEqual({ before: "ev-b" });

    await setFactOrder(SLUG, SCENARIO, NODE, "ev-a", null);
    expect(JSON.parse(fetchMock.mock.calls[1][1].body)).toEqual({ after: "ev-a" });

    await setFactOrder(SLUG, SCENARIO, NODE, null, null);
    expect(JSON.parse(fetchMock.mock.calls[2][1].body)).toEqual({});
  });

  it("throws on a refusal so the row never looks moved when it was not", async () => {
    // A 409 is the interesting one: a neighbour has gone, or the gap is
    // exhausted. Both are things the human can act on, so the failure must
    // reach them rather than being swallowed into a silent no-op.
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 409,
      json: async () => ({ message: "there is no room left between those two facts" }),
    });

    await expect(setFactOrder(SLUG, SCENARIO, NODE, "ev-a", "ev-b")).rejects.toThrow(/409/);
  });
});
