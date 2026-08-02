import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { fetchSettings, setSetting } from "../settings";

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
    mockFetch.mockResolvedValue(ok({ settings: [] }));

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
