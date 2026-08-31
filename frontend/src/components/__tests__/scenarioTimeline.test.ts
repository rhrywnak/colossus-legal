/**
 * Service tests for the scenario dock's client (task 3).
 *
 * ## ⚑ THE HALF THESE EXIST FOR
 *
 * The same one its two siblings name: the .377 build shipped a client calling
 * an API path the axum router did not serve — well-typed, syntactically
 * perfect, and pointing nowhere. So these pin the METHOD and the PATH of all
 * three calls against the routes `api::timeline_subsets::scenario_links`
 * declares, and every distinct failure as its own sentence.
 *
 * ## And one this module has that the siblings do not
 *
 * `GET /cases/:slug/scenarios/:id/subsets` carries the dock's WHOLE VOCABULARY
 * as well as its list. A payload arriving without `wording` would leave every
 * control on the dock throwing by key, one at a time, from inside a render — so
 * the shape check refuses it once, at the boundary, and says which half is
 * missing. That is the case `a payload with no wording is refused` covers.
 *
 * `fetch` is stubbed at the global, which is what `authFetch` calls. No DOM.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  attachSubset,
  detachSubset,
  getScenarioSubsets,
} from "../scenario-timeline/scenarioTimeline";

const SLUG = "awad_v_catholic_family_service";
const SCENARIO = "f5849675-b0c7-4f5f-a383-f4aefedfa8eb";
const SUBSET = "f32dd338-f5fd-4e7e-9a10-b379d70af196";

/** One attached subset, as the button's read lists it. */
const ATTACHED = {
  id: SUBSET,
  name: "The $50,000",
  event_count: 9,
  gap_count: 0,
  position: 0,
};

/** A wording block only has to be a non-empty object to pass the boundary. */
const WORDING = { scenario_view_timeline_button: "View Timeline" };

type Call = { url: string; init: RequestInit | undefined };
let calls: Call[];

function stubFetch(response: {
  ok: boolean;
  status?: number;
  body?: unknown;
  throws?: Error;
  badJson?: boolean;
}): void {
  vi.stubGlobal(
    "fetch",
    vi.fn((url: string, init?: RequestInit) => {
      calls.push({ url, init });
      if (response.throws) return Promise.reject(response.throws);
      return Promise.resolve({
        ok: response.ok,
        status: response.status ?? (response.ok ? 200 : 500),
        json: () =>
          response.badJson
            ? Promise.reject(new Error("not json"))
            : Promise.resolve(response.body),
      } as unknown as Response);
    }),
  );
}

beforeEach(() => {
  calls = [];
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("every call reaches the route the backend declares", () => {
  it("reads with GET /api/cases/:slug/scenarios/:id/subsets", async () => {
    stubFetch({ ok: true, body: { subsets: [ATTACHED], wording: WORDING } });
    const out = await getScenarioSubsets(SLUG, SCENARIO);
    expect(calls[0].url).toContain(`/api/cases/${SLUG}/scenarios/${SCENARIO}/subsets`);
    expect(calls[0].init?.method).toBeUndefined();
    expect(out.subsets).toHaveLength(1);
    expect(out.subsets[0].event_count).toBe(9);
    // The whole point of the envelope: the words arrive with the list.
    expect(out.wording.scenario_view_timeline_button).toBe("View Timeline");
  });

  it("attaches with POST to the same path, carrying subset_id", async () => {
    stubFetch({ ok: true, body: [ATTACHED] });
    const out = await attachSubset(SLUG, SCENARIO, SUBSET);
    expect(calls[0].url).toContain(`/api/cases/${SLUG}/scenarios/${SCENARIO}/subsets`);
    expect(calls[0].init?.method).toBe("POST");
    expect(JSON.parse(String(calls[0].init?.body))).toEqual({ subset_id: SUBSET });
    expect(out).toHaveLength(1);
  });

  it("detaches with DELETE …/subsets/:subset_id and no body", async () => {
    stubFetch({ ok: true, body: [] });
    const out = await detachSubset(SLUG, SCENARIO, SUBSET);
    expect(calls[0].url).toContain(
      `/api/cases/${SLUG}/scenarios/${SCENARIO}/subsets/${SUBSET}`,
    );
    expect(calls[0].init?.method).toBe("DELETE");
    expect(calls[0].init?.body).toBeUndefined();
    expect(out).toEqual([]);
  });

  it("encodes every path parameter rather than pasting it in", async () => {
    stubFetch({ ok: true, body: [] });
    await detachSubset("a/b", "c d", "e/f");
    expect(calls[0].url).toContain("a%2Fb");
    expect(calls[0].url).toContain("c%20d");
    expect(calls[0].url).toContain("e%2Ff");
  });

  it("bounds every call with a timeout signal (Rule 13)", async () => {
    stubFetch({ ok: true, body: { subsets: [], wording: WORDING } });
    await getScenarioSubsets(SLUG, SCENARIO);
    expect(calls[0].init?.signal).toBeDefined();

    calls = [];
    stubFetch({ ok: true, body: [] });
    await attachSubset(SLUG, SCENARIO, SUBSET);
    expect(calls[0].init?.signal).toBeDefined();
  });

  it("an empty list is a normal answer — it is what hides the button", async () => {
    // NOT a 404, which would mean "no such scenario". A surface collapsing the
    // two would draw a working dock over a scenario that does not exist.
    stubFetch({ ok: true, body: { subsets: [], wording: WORDING } });
    await expect(getScenarioSubsets(SLUG, SCENARIO)).resolves.toMatchObject({ subsets: [] });
  });
});

describe("nothing is swallowed — every failure is its own sentence", () => {
  it("a network failure names the resource and the cause", async () => {
    stubFetch({ ok: false, throws: new Error("connection reset") });
    await expect(getScenarioSubsets(SLUG, SCENARIO)).rejects.toThrow(
      /Failed to load this scenario's timeline subsets \(connection reset\)/,
    );
  });

  it("a non-2xx carries the status", async () => {
    stubFetch({ ok: false, status: 404, body: {} });
    await expect(getScenarioSubsets(SLUG, SCENARIO)).rejects.toThrow(/HTTP 404/);
  });

  it("a non-2xx write carries the server's OWN message", async () => {
    // T1 answers 409 with a message; this is what puts it in front of the
    // person who caused it rather than leaving it in a console.
    stubFetch({
      ok: false,
      status: 409,
      body: { error: "conflict", message: "that subset is already attached" },
    });
    await expect(attachSubset(SLUG, SCENARIO, SUBSET)).rejects.toThrow(
      /HTTP 409 — that subset is already attached/,
    );
  });

  it("an unparseable body is a DIFFERENT sentence from a bad status", async () => {
    stubFetch({ ok: true, badJson: true });
    await expect(detachSubset(SLUG, SCENARIO, SUBSET)).rejects.toThrow(
      /did not come back as valid JSON/,
    );
  });

  it("a payload with no WORDING is refused at the boundary", async () => {
    // ⚑ The failure this module has that its siblings do not. Without this the
    // dock renders and every control throws by key, one at a time, from inside
    // a render — a screen that looks like ten faults instead of one contract.
    stubFetch({ ok: true, body: { subsets: [ATTACHED] } });
    await expect(getScenarioSubsets(SLUG, SCENARIO)).rejects.toThrow(
      /without their list or their wording/,
    );
  });

  it("a payload with no SUBSETS list is refused too", async () => {
    stubFetch({ ok: true, body: { wording: WORDING } });
    await expect(getScenarioSubsets(SLUG, SCENARIO)).rejects.toThrow(
      /disagree about the payload shape/,
    );
  });

  it("a write answering something other than a list yields an empty list", async () => {
    // The writes return the list as it now stands; a malformed answer must not
    // become `undefined.map` inside the row.
    stubFetch({ ok: true, body: { not: "a list" } });
    await expect(attachSubset(SLUG, SCENARIO, SUBSET)).resolves.toEqual([]);
  });
});
