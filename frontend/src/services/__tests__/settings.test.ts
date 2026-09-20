import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { fetchSettings, setSetting, setSettingGroup } from "../settings";

vi.mock("../auth", () => ({
  authFetch: vi.fn(),
}));

import { authFetch } from "../auth";

const mockFetch = vi.mocked(authFetch);

function ok(body: unknown): Response {
  return { ok: true, status: 200, json: async () => body } as unknown as Response;
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

describe("fetchSettings", () => {
  it("reads the parameter list from the settings endpoint", async () => {
    mockFetch.mockResolvedValue(ok({ settings: [], areas: [], groups: [] }));

    await fetchSettings();

    expect(String(mockFetch.mock.calls[0]?.[0])).toContain("/api/settings");
  });

  it("returns the parameters in the order the backend sent them", async () => {
    // Live parameters before dormant ones is a fact about the system, decided
    // server-side. Re-sorting here would override that judgement.
    mockFetch.mockResolvedValue(
      ok({
        settings: [
          { key: "talking_points_cap", dormant_note: null },
          { key: "readiness_item_threshold_n", dormant_note: "…no effect today." },
        ],
        areas: [{ id: "core", label: "Core", count: 2, note: null, blocks: [] }],
        groups: [],
      }),
    );

    const page = await fetchSettings();

    expect(page.settings.map((s) => s.key)).toEqual([
      "talking_points_cap",
      "readiness_item_threshold_n",
    ]);
  });

  it("throws with the backend message on a failure", async () => {
    mockFetch.mockResolvedValue(failure(500, "failed to load the settings"));

    await expect(fetchSettings()).rejects.toThrow(/500/);
  });

  it("throws when the payload carries no parameter list", async () => {
    mockFetch.mockResolvedValue(ok({}));

    await expect(fetchSettings()).rejects.toThrow(/contract mismatch/);
  });

  it("throws when the payload carries no rail", async () => {
    // Without the areas the page has no grouping and no counts, and would fall
    // back to the flat 863-row column this rebuild replaced. That is a contract
    // mismatch, not a degraded mode.
    mockFetch.mockResolvedValue(
      ok({ settings: [{ key: "talking_points_cap" }], groups: [] }),
    );

    await expect(fetchSettings()).rejects.toThrow(/carried no areas/);
  });

  it("accepts an empty rail, which is a real answer about an empty store", async () => {
    mockFetch.mockResolvedValue(ok({ settings: [], areas: [], groups: [] }));

    await expect(fetchSettings()).resolves.toEqual({
      settings: [],
      areas: [],
      groups: [],
    });
  });

  it("throws when the payload carries no coupled groups", async () => {
    // Without them, rows that must be edited together would render as single
    // fields whose save the backend refuses — the page would look like the one
    // this task replaced.
    mockFetch.mockResolvedValue(ok({ settings: [], areas: [] }));

    await expect(fetchSettings()).rejects.toThrow(/carried no coupled groups/);
  });
});

describe("setSetting", () => {
  it("PUTs the value as typed, to the key's own path", async () => {
    mockFetch.mockResolvedValue(
      ok({ key: "talking_points_cap", value: "5", message: "now 5" }),
    );

    await setSetting("talking_points_cap", "5");

    const [url, init] = mockFetch.mock.calls[0] ?? [];
    expect(String(url)).toContain("/api/settings/talking_points_cap");
    expect((init as RequestInit)?.method).toBe("PUT");
    expect((init as RequestInit)?.body).toBe(JSON.stringify({ value: "5" }));
  });

  it("sends a ratio unchanged rather than converting it", async () => {
    // "9/10" is the stored form. A browser that helpfully sent 0.9 would lose the
    // denominator, which is part of what the parameter means.
    mockFetch.mockResolvedValue(ok({ key: "card_test_ratio", value: "8/10", message: "ok" }));

    await setSetting("card_test_ratio", "8/10");

    expect((mockFetch.mock.calls[0]?.[1] as RequestInit)?.body).toBe(
      JSON.stringify({ value: "8/10" }),
    );
  });

  it("encodes a key that would otherwise change the path", async () => {
    mockFetch.mockResolvedValue(ok({ key: "a/b", value: "1", message: "ok" }));

    await setSetting("a/b", "1");

    expect(String(mockFetch.mock.calls[0]?.[0])).toContain("a%2Fb");
  });

  it("surfaces a validation refusal with the backend's own sentence", async () => {
    // The refusal names the parameter and the bound. That detail is the only
    // thing that tells a human what to type instead, so it must survive.
    mockFetch.mockResolvedValue(
      failure(400, "talking_points_cap must be at least 1, but the value is 0"),
    );

    await expect(setSetting("talking_points_cap", "0")).rejects.toThrow(/at least 1/);
  });

  it("names the parameter when a change is refused", async () => {
    mockFetch.mockResolvedValue(failure(404, "no such key"));

    await expect(setSetting("invented", "1")).rejects.toThrow(/invented/);
  });

  it("returns the backend's confirmation verbatim", async () => {
    mockFetch.mockResolvedValue(
      ok({
        key: "talking_points_cap",
        value: "5",
        message:
          "talking_points_cap is now 5. It takes effect on the next read — no rebuild, no redeploy.",
      }),
    );

    const changed = await setSetting("talking_points_cap", "5");

    expect(changed.message).toContain("no rebuild");
  });

  it("throws when the response carries no confirmation", async () => {
    // A 200 with no message means we cannot tell the human what happened, and
    // these values decide how the product behaves. Saying "saved" would be a guess.
    mockFetch.mockResolvedValue(ok({ key: "talking_points_cap" }));

    await expect(setSetting("talking_points_cap", "5")).rejects.toThrow(/Reload/);
  });
});

describe("setSettingGroup", () => {
  // Raised by the test-auditor gate: this is the only function the coupled
  // editor calls to write data, and `setSetting` beside it has seven tests.
  // The structural paths are the same ones, because the failure modes are.

  it("PUTs every entry to the group's own path, in one request", async () => {
    mockFetch.mockResolvedValue(
      ok({ key: "reviewer_bench", value: "2", message: "now lists 2 reviewers." }),
    );

    await setSettingGroup("reviewer_bench", [
      ["cpenzien", "Chuck"],
      ["roman", "Roman"],
    ]);

    const [url, init] = mockFetch.mock.calls[0] ?? [];
    expect(String(url)).toContain("/api/settings/group/reviewer_bench");
    expect((init as RequestInit)?.method).toBe("PUT");
    // ONE request carrying BOTH reviewers — the whole point. Two requests is
    // the deadlock this endpoint exists to end.
    expect(mockFetch.mock.calls.length).toBe(1);
    expect((init as RequestInit)?.body).toBe(
      JSON.stringify({
        entries: [
          ["cpenzien", "Chuck"],
          ["roman", "Roman"],
        ],
      }),
    );
  });

  it("sends an empty table as it is, and lets the backend refuse it", async () => {
    // The editor blocks Save on an empty table, but the REFUSAL is the
    // backend's: an empty bench means nobody can clear the review queue, and
    // that sentence is written where the rule is.
    mockFetch.mockResolvedValue(
      failure(400, "needs at least one reviewer"),
    );

    await expect(setSettingGroup("reviewer_bench", [])).rejects.toThrow(/400/);
    expect((mockFetch.mock.calls[0]?.[1] as RequestInit)?.body).toBe(
      JSON.stringify({ entries: [] }),
    );
  });

  it("encodes a group id that would otherwise change the path", async () => {
    mockFetch.mockResolvedValue(ok({ key: "a/b", value: "1", message: "ok" }));

    await setSettingGroup("a/b", [["x", "y"]]);

    expect(String(mockFetch.mock.calls[0]?.[0])).toContain("a%2Fb");
  });

  it("surfaces the backend's refusal, which names the column and the row", async () => {
    mockFetch.mockResolvedValue(
      failure(400, "Sign-in name is empty on reviewer 2"),
    );

    await expect(
      setSettingGroup("reviewer_bench", [["a", "A"], ["", "B"]]),
    ).rejects.toThrow(/Sign-in name is empty on reviewer 2/);
  });

  it("throws when the response carries no confirmation", async () => {
    // The change may or may not have landed, and saying so is the only honest
    // answer — the same guard `setSetting` carries.
    mockFetch.mockResolvedValue(ok({ key: "reviewer_bench", value: "2" }));

    await expect(setSettingGroup("reviewer_bench", [["a", "A"]])).rejects.toThrow(
      /may or may not have been saved/,
    );
  });
});
