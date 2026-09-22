/**
 * Service tests for the "For you" client (CC_TASK_FOR_YOU_v1 L1).
 *
 * ## The URL guards, and the failure class they exist for
 *
 * The .377 build shipped a client calling a path the router did not serve: a
 * whole feature answering 404 with nothing on either side saying why. The
 * server half of this guard is `api::route_table_tests`; this is the client
 * half. The assertions below SPELL THE PATHS OUT rather than building them from
 * the same constants the code under test uses, so the two can only agree by
 * being edited to agree.
 *
 * ## Standing Rule 1 on this client
 *
 * A failed request, a shape mismatch and an empty list are three different
 * things with three different outcomes. The third is not an error: a person
 * with nothing waiting resolves normally, and the page shows its empty state.
 * The first two throw a sentence naming what did not happen, because a page
 * that rendered an empty list after a failed read would be telling her nothing
 * is waiting — which is the one thing this feature must never say wrongly.
 *
 * Mocks `global.fetch` because `authFetch` calls it.
 */
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  fetchForYou,
  fetchForYouSummary,
  markQuestionSeen,
  wordingOf,
  type ForYouPage,
} from "../forYou";

const SLUG = "awad_v_catholic_family_service";
const QUESTION = "44444444-4444-4444-4444-444444444444";

afterEach(() => {
  vi.restoreAllMocks();
});

/** Mock `fetch` with one OK response, and hand back the spy. */
function okFetch(body: unknown = {}) {
  const mock = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => body });
  // @ts-ignore
  global.fetch = mock;
  return mock;
}

/** Mock `fetch` with one failing response. */
function failFetch(status: number) {
  // @ts-ignore
  global.fetch = vi.fn().mockResolvedValue({ ok: false, status, text: async () => "" });
}

/** A minimal but complete page payload, matching the backend DTO. */
function page(overrides: Partial<ForYouPage> = {}): ForYouPage {
  return {
    side: "witness",
    subtitle: "Chuck's notes on your answers.",
    tab_unread_label: "Unread · 0",
    tab_everything_label: "Everything · 0",
    unread_count: 0,
    unread: [],
    everything: [],
    empty_hint: "Chuck's notes and answers appear here as they arrive.",
    wording: { title: "For you", empty_title: "Nothing waiting for you." },
    ...overrides,
  };
}

describe("fetchForYou", () => {
  it("GETs the case-scoped For you URL", async () => {
    const mock = okFetch(page());
    await fetchForYou(SLUG);
    const [url] = mock.mock.calls[0];
    expect(url).toContain(`/api/cases/${SLUG}/for-you`);
  });

  it("resolves normally on an EMPTY list — that is a screen, not a failure", async () => {
    // A client that treated "nothing waiting" as an error would put a red
    // failure notice in front of the person who is up to date.
    okFetch(page());
    const payload = await fetchForYou(SLUG);
    expect(payload.unread).toEqual([]);
    expect(payload.unread_count).toBe(0);
  });

  it("throws with the status on a non-OK response", async () => {
    failFetch(500);
    await expect(fetchForYou(SLUG)).rejects.toThrow(/HTTP 500/);
  });

  it("throws when the payload carries no side", async () => {
    // Without it the page cannot tell "this is not your list" from "your list
    // is empty" — two different sentences for two different facts.
    const { side: _dropped, ...rest } = page();
    okFetch(rest);
    await expect(fetchForYou(SLUG)).rejects.toThrow(/contract mismatch/);
  });

  it("throws when either list is missing", async () => {
    // React renders `undefined` as nothing at all, so a missing list would draw
    // an empty page that claims nothing is waiting.
    const { unread: _a, ...noUnread } = page();
    okFetch(noUnread);
    await expect(fetchForYou(SLUG)).rejects.toThrow(/contract mismatch/);

    const { everything: _b, ...noEverything } = page();
    okFetch(noEverything);
    await expect(fetchForYou(SLUG)).rejects.toThrow(/contract mismatch/);
  });

  it("throws when the wording is missing or null", async () => {
    // Every label on the page would throw one at a time, in render, with the
    // first one taking the screen down. Better to say it once, here.
    const { wording: _dropped, ...rest } = page();
    okFetch(rest);
    await expect(fetchForYou(SLUG)).rejects.toThrow(/contract mismatch/);

    okFetch(page({ wording: null as unknown as ForYouPage["wording"] }));
    await expect(fetchForYou(SLUG)).rejects.toThrow(/contract mismatch/);
  });
});

describe("fetchForYouSummary", () => {
  it("GETs the summary URL", async () => {
    const mock = okFetch({ side: "witness", unread_count: 3 });
    const summary = await fetchForYouSummary(SLUG);
    const [url] = mock.mock.calls[0];
    expect(url).toContain(`/api/cases/${SLUG}/for-you/summary`);
    expect(summary.unread_count).toBe(3);
  });

  it("resolves on a count of ZERO — the badge's absence is the caller's decision", () => {
    okFetch({ side: "witness", unread_count: 0 });
    return expect(fetchForYouSummary(SLUG)).resolves.toMatchObject({ unread_count: 0 });
  });

  it("throws with the status on a non-OK response", async () => {
    failFetch(503);
    await expect(fetchForYouSummary(SLUG)).rejects.toThrow(/HTTP 503/);
  });

  it("throws when the count or the side is missing", async () => {
    // A missing count read as `undefined` would draw no badge — which is what a
    // count of zero draws, and the two must never be confused.
    okFetch({ side: "witness" });
    await expect(fetchForYouSummary(SLUG)).rejects.toThrow(/contract mismatch/);

    okFetch({ unread_count: 2 });
    await expect(fetchForYouSummary(SLUG)).rejects.toThrow(/contract mismatch/);
  });
});

describe("markQuestionSeen", () => {
  it("POSTs to the question's seen address", async () => {
    const mock = okFetch({ question_id: QUESTION, marked: 2 });
    const marked = await markQuestionSeen(QUESTION);
    const [url, options] = mock.mock.calls[0];
    expect(url).toContain(`/api/practice/questions/${QUESTION}/seen`);
    expect(options.method).toBe("POST");
    expect(marked).toBe(2);
  });

  it("resolves on ZERO marked — she had already read it", async () => {
    // A second visit writes nothing. That is idempotence working, not a
    // failure, and the caller is told the number rather than a boolean.
    okFetch({ question_id: QUESTION, marked: 0 });
    await expect(markQuestionSeen(QUESTION)).resolves.toBe(0);
  });

  it("throws with the status on a non-OK response", async () => {
    failFetch(500);
    await expect(markQuestionSeen(QUESTION)).rejects.toThrow(/HTTP 500/);
  });

  it("throws when the response carries no count", async () => {
    okFetch({ question_id: QUESTION });
    await expect(markQuestionSeen(QUESTION)).rejects.toThrow(/contract mismatch/);
  });
});

describe("wordingOf", () => {
  it("returns the stored string", () => {
    expect(wordingOf({ title: "For you" }, "title")).toBe("For you");
  });

  it("throws BY NAME on a key the payload does not carry", () => {
    // The .407 failure, in one line: a key nobody serialized renders as nothing
    // at all, and a blank page is the hardest thing to diagnose from a report.
    expect(() => wordingOf({}, "group_today")).toThrow(/group_today/);
  });

  it("throws on a blank value rather than rendering an empty label", () => {
    expect(() => wordingOf({ group_today: "   " }, "group_today")).toThrow(/group_today/);
  });
});
