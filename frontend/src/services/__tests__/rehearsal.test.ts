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

describe("fetchRehearsal", () => {
  it("asks the case-level endpoint and sends no status filter", () => {
    // The gate is the SERVER's. A client that could ask for drafted scenarios is
    // a client that could put one in front of a witness.
    mockFetch.mockResolvedValue(ok({ scenarios: [], standing_card: [] }));

    void fetchRehearsal("awad-v-cfs");

    const url = String(mockFetch.mock.calls[0]?.[0]);
    expect(url).toContain("/cases/awad-v-cfs/rehearsal");
    expect(url).not.toContain("status");
  });

  it("encodes a slug that would otherwise change the path", async () => {
    mockFetch.mockResolvedValue(ok({ scenarios: [], standing_card: [] }));

    await fetchRehearsal("a/b");

    expect(String(mockFetch.mock.calls[0]?.[0])).toContain("a%2Fb");
  });

  it("returns an empty list as a real state, not an error", async () => {
    // Nobody has declared a scenario ready yet. That is a legitimate reading of
    // the case, and the page says so in words.
    mockFetch.mockResolvedValue(ok({ scenarios: [], standing_card: ["Tell the truth."] }));

    const payload = await fetchRehearsal("awad-v-cfs");

    expect(payload.scenarios).toEqual([]);
    expect(payload.standing_card).toHaveLength(1);
  });

  it("throws with the backend message on a failure", async () => {
    mockFetch.mockResolvedValue(failure(500, "failed to load"));

    await expect(fetchRehearsal("awad-v-cfs")).rejects.toThrow(/500/);
  });

  it("throws when the payload is missing its scenarios", async () => {
    // A contract mismatch must fail HERE with context, not as an
    // `undefined.map` in the middle of a rehearsal.
    mockFetch.mockResolvedValue(ok({ standing_card: [] }));

    await expect(fetchRehearsal("awad-v-cfs")).rejects.toThrow(/contract/);
  });

  it("throws when the standing card is missing", async () => {
    // §10 makes the card always-shown. A payload without it is not a rehearsal.
    mockFetch.mockResolvedValue(ok({ scenarios: [] }));

    await expect(fetchRehearsal("awad-v-cfs")).rejects.toThrow(/standing card/);
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
