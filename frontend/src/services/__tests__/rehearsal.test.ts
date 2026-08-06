import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { fetchRehearsal, setScenarioReady } from "../rehearsal";

// The house service-test pattern: stub `authFetch`, assert the URL, the method,
// the body, and — the part that matters — that a non-2xx THROWS with context
// rather than resolving to something the caller reads as success.
vi.mock("../auth", () => ({
  authFetch: vi.fn(),
}));

import { authFetch } from "../auth";

const mockFetch = vi.mocked(authFetch);

function ok(body: unknown): Response {
  return {
    ok: true,
    status: 200,
    json: async () => body,
  } as unknown as Response;
}

function failure(status: number, message: string): Response {
  return {
    ok: false,
    status,
    json: async () => ({ message }),
    text: async () => JSON.stringify({ message }),
  } as unknown as Response;
}

beforeEach(() => {
  mockFetch.mockReset();
});

afterEach(() => {
  vi.restoreAllMocks();
});

/** A minimal but complete payload, matching the backend DTO. */
function payload(overrides: Record<string, unknown> = {}) {
  return {
    scenarios: [],
    positions: [],
    always: { heading: "Always", lines: ["Tell the truth."] },
    collapse: {
      accusation_open: true,
      timeline_open: false,
      points_open: true,
      watch_for_open: true,
    },
    wording: {
      answer_label: "Our answer:",
      page_heading: "Rehearsal",
      purpose_line: "Your testimony-prep view.",
      previous_label: "Back",
      next_label: "Next",
      nothing_ready_notice: "Nothing is ready to rehearse yet.",
      not_ready_notice: "{code} is not ready to rehearse yet.",
      expand_all_label: "Open everything",
      collapse_all_label: "Fold everything",
      block_what_heading: "What this is",
      block_accusation_heading: "The accusation",
      block_timeline_heading: "The timeline",
      block_points_heading: "Your points",
      block_watch_heading: "Watch for",
    },
    ...overrides,
  };
}

describe("fetchRehearsal", () => {
  it("asks the case-level endpoint and sends no status filter", async () => {
    // The gate is the SERVER's. A client that could ask for drafted scenarios is
    // a client that could put one in front of a witness — so there is no
    // parameter to ask with, and this asserts there never quietly becomes one.
    mockFetch.mockResolvedValue(ok(payload()));

    await fetchRehearsal("awad_v_catholic_family_service");

    const [url] = mockFetch.mock.calls[0];
    expect(url).toContain("/api/cases/awad_v_catholic_family_service/rehearsal");
    expect(url).not.toContain("status");
    expect(url).not.toContain("ready");
  });

  it("percent-encodes the slug", async () => {
    mockFetch.mockResolvedValue(ok(payload()));
    await fetchRehearsal("a/b");
    expect(String(mockFetch.mock.calls[0][0])).toContain("a%2Fb");
  });

  it("names no path the gateway already adds", async () => {
    // The .377 failure class:  is reachable by nothing, and on screen
    // it is indistinguishable from a feature nobody built.
    mockFetch.mockResolvedValue(ok(payload()));
    await fetchRehearsal("awad_v_catholic_family_service");
    expect(String(mockFetch.mock.calls[0][0])).not.toContain("/api/api/");
  });

  it("throws with the case and status on a non-OK response", async () => {
    mockFetch.mockResolvedValue(failure(500, "boom"));
    await expect(fetchRehearsal("awad")).rejects.toThrow(/rehearsal mode for "awad".*HTTP 500/);
  });

  it("returns the payload whole, including the standing card", async () => {
    mockFetch.mockResolvedValue(
      ok(payload({ always: { heading: "Always", lines: ["Tell the truth.", "Don't guess."] } })),
    );

    const loaded = await fetchRehearsal("awad");
    expect(loaded.always.lines).toHaveLength(2);
    expect(loaded.always.heading).toBe("Always");
  });

  it("accepts an empty rehearsal as a valid, distinct answer", async () => {
    // Nobody has declared a scenario ready. A real state with its own stored
    // sentence — it must NOT throw, or the page would report a failure for a
    // case that is simply not started.
    mockFetch.mockResolvedValue(ok(payload()));
    await expect(fetchRehearsal("awad")).resolves.toMatchObject({ scenarios: [] });
  });

  it("throws when the wording block is missing rather than rendering blank headings", async () => {
    // R4: there is no literal to fall back to. A page with no words is a column
    // of unlabelled sections, which is worse than a stated failure.
    const { wording: _dropped, ...rest } = payload();
    mockFetch.mockResolvedValue(ok(rest));

    await expect(fetchRehearsal("awad")).rejects.toThrow(
      /missing scenarios\/positions\/always\/wording\/collapse/,
    );
  });

  it("throws when the standing card is missing rather than dropping it silently", async () => {
    // §10 makes the Always card the one block never scrolled away from. Losing it
    // quietly is the failure this guard exists for.
    const { always: _dropped, ...rest } = payload();
    mockFetch.mockResolvedValue(ok(rest));

    await expect(fetchRehearsal("awad")).rejects.toThrow(/backend\/frontend contract mismatch/);
  });

  it("throws when the standing card has no lines array", async () => {
    // A heading with nothing under it renders a bordered box and a title — which
    // looks deliberate, and is the worst way for this block to fail.
    mockFetch.mockResolvedValue(ok(payload({ always: { heading: "Always" } })));
    await expect(fetchRehearsal("awad")).rejects.toThrow(/contract mismatch/);
  });

  it("throws when the positions are missing rather than blanking the position line", async () => {
    const { positions: _dropped, ...rest } = payload();
    mockFetch.mockResolvedValue(ok(rest));
    await expect(fetchRehearsal("awad")).rejects.toThrow(/contract mismatch/);
  });
});

describe("setScenarioReady", () => {
  it("states the target rather than toggling", async () => {
    mockFetch.mockResolvedValue(
      ok({ status: "ready", in_rehearsal: true, message: "S-2 is ready" }),
    );

    await setScenarioReady("awad-v-cfs", "abc", true);

    const [url, init] = mockFetch.mock.calls[0] ?? [];
    expect(String(url)).toContain("/scenarios/abc/ready");
    expect((init as RequestInit)?.method).toBe("POST");
    expect((init as RequestInit)?.body).toBe(JSON.stringify({ ready: true }));
  });

  it("sends ready:false to withdraw, on the same route", async () => {
    // One recorded path for BOTH directions — that is what makes "who took S-2
    // out of rehearsal?" answerable.
    mockFetch.mockResolvedValue(
      ok({ status: "draft", in_rehearsal: false, message: "S-2 removed" }),
    );

    await setScenarioReady("awad-v-cfs", "abc", false);

    const [url, init] = mockFetch.mock.calls[0] ?? [];
    expect(String(url)).toContain("/ready");
    expect((init as RequestInit)?.body).toBe(JSON.stringify({ ready: false }));
  });

  it("returns the backend's confirmation verbatim", async () => {
    mockFetch.mockResolvedValue(
      ok({
        status: "draft",
        in_rehearsal: false,
        message: "S-2 removed from rehearsal — nothing else changed.",
      }),
    );

    const change = await setScenarioReady("awad-v-cfs", "abc", false);

    expect(change.message).toBe("S-2 removed from rehearsal — nothing else changed.");
    expect(change.in_rehearsal).toBe(false);
  });

  it("throws when the change is refused, naming the direction attempted", async () => {
    mockFetch.mockResolvedValue(failure(400, "already ready"));

    await expect(setScenarioReady("awad-v-cfs", "abc", true)).rejects.toThrow(/declare/);
  });

  it("throws when the response carries no confirmation", async () => {
    // A 200 with no message means we cannot tell the human what happened. Saying
    // "saved" would be a guess about a state that decides what a witness sees.
    mockFetch.mockResolvedValue(ok({ status: "ready" }));

    await expect(setScenarioReady("awad-v-cfs", "abc", true)).rejects.toThrow(/Reload/);
  });
});
