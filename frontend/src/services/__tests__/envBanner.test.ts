// =============================================================================
// envBanner.test.ts — the one fetch behind the test-system bar
// =============================================================================
//
// The service layer IS testable here — `global.fetch` can be replaced — and this
// one has three distinct failure arms, all of which end the same way: `null`,
// and the bar keeps its compiled sentence. That equivalence is the thing worth
// pinning, because the bar going wordless would be the one failure a witness
// would actually be harmed by.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { fetchEnvBannerWords } from "../envBanner";

const WORDS = {
  text: "TEST SYSTEM — practice here is not saved for trial.",
  link_label: "Go to the real system",
  print_line: "TEST SYSTEM — NOT FOR TRIAL",
  real_url: "https://colossus-legal.cogmai.com",
};

const ok = (body: unknown) => {
  const mock = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => body });
  vi.stubGlobal("fetch", mock);
  return mock;
};

beforeEach(() => {
  // Every arm below logs; the console is silenced so a passing run is quiet and
  // the assertions are about the RETURN, not the noise.
  vi.spyOn(console, "warn").mockImplementation(() => {});
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("the bar's stored words", () => {
  it("returns all four fields on the happy path", async () => {
    const mock = ok(WORDS);
    await expect(fetchEnvBannerWords()).resolves.toEqual(WORDS);
    expect(String(mock.mock.calls[0][0])).toContain("/api/env-banner");
  });

  it("gives up quietly on an HTTP failure — the bar keeps the compiled sentence", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({ ok: false, status: 503, text: async () => "", json: async () => ({}) }),
    );
    await expect(fetchEnvBannerWords()).resolves.toBeNull();
    expect(console.warn).toHaveBeenCalled();
  });

  it("refuses a payload missing a field rather than rendering undefined", async () => {
    for (const missing of ["text", "link_label", "print_line", "real_url"] as const) {
      const partial: Record<string, unknown> = { ...WORDS };
      delete partial[missing];
      ok(partial);
      await expect(fetchEnvBannerWords(), `missing ${missing}`).resolves.toBeNull();
    }
  });

  it("refuses a field of the wrong type", async () => {
    ok({ ...WORDS, real_url: 42 });
    await expect(fetchEnvBannerWords()).resolves.toBeNull();
  });

  it("gives up quietly when the call never returns", async () => {
    vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new Error("network down")));
    await expect(fetchEnvBannerWords()).resolves.toBeNull();
    expect(console.warn).toHaveBeenCalled();
  });
});
