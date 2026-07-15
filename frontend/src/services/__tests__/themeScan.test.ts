/**
 * Service tests for the background Theme Scan client (start + poll).
 * Mocks global.fetch (authFetch calls it). Standing Rule 1: a non-OK response
 * throws with the backend message surfaced VERBATIM. Mirrors scenarioGather.test.ts.
 */
import { afterEach, describe, expect, it, vi } from "vitest";

import { fetchScanModels, getScanRun, startThemeScan } from "../themeScan";

const SLUG = "awad";
const SCENARIO = "11111111-1111-1111-1111-111111111111";
const RUN = "22222222-2222-2222-2222-222222222222";

afterEach(() => {
  vi.restoreAllMocks();
});

describe("startThemeScan", () => {
  it("POSTs the theme-scan URL with the model + dry_run body", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ run_id: RUN, status: "running", candidates_total: 94 }),
    });
    // @ts-ignore — minimal fetch mock
    global.fetch = fetchMock;

    await expect(
      startThemeScan(SLUG, SCENARIO, { model_id: "qwen-14b", dry_run: true }),
    ).resolves.toEqual({ run_id: RUN, status: "running", candidates_total: 94 });

    const [url, options] = fetchMock.mock.calls[0];
    expect(url).toContain(`/api/cases/${SLUG}/scenarios/${SCENARIO}/theme-scan`);
    expect(options.method).toBe("POST");
    expect(JSON.parse(options.body)).toEqual({ model_id: "qwen-14b", dry_run: true });
  });

  it("surfaces the backend hard-gate message VERBATIM on a 503", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 503,
      json: async () => ({
        message:
          "vLLM endpoint 'http://10.10.100.200:8000' has the wrong model loaded: selected 'qwen-14b' but loaded 'qwen-7b'",
      }),
    });

    await expect(
      startThemeScan(SLUG, SCENARIO, { dry_run: true }),
    ).rejects.toThrow(/selected 'qwen-14b' but loaded 'qwen-7b'/);
  });
});

describe("getScanRun", () => {
  it("GETs the scan-runs URL and returns the typed status", async () => {
    const status = {
      run_id: RUN,
      status: "running",
      model_id: "qwen-14b",
      dry_run: true,
      candidates_total: 94,
      candidates_judged: 37,
      relevant_count: 5,
      irrelevant_count: 30,
      failed_count: 2,
    };
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => status });
    // @ts-ignore
    global.fetch = fetchMock;

    await expect(getScanRun(SLUG, SCENARIO, RUN)).resolves.toEqual(status);
    expect(fetchMock.mock.calls[0][0]).toContain(
      `/api/cases/${SLUG}/scenarios/${SCENARIO}/scan-runs/${RUN}`,
    );
  });

  it("throws on a non-OK poll response", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({ ok: false, status: 500, json: async () => ({}) });
    await expect(getScanRun(SLUG, SCENARIO, RUN)).rejects.toThrow(/Failed to read scan run/);
  });
});

describe("fetchScanModels", () => {
  it("returns the models array, and [] when the key is absent", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ models: [{ model_id: "m1", display_name: "M1", is_default: true }] }),
    });
    await expect(fetchScanModels()).resolves.toEqual([
      { model_id: "m1", display_name: "M1", is_default: true },
    ]);

    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => ({}) });
    await expect(fetchScanModels()).resolves.toEqual([]);
  });

  it("throws on a non-OK response", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({ ok: false, status: 403, json: async () => ({}) });
    await expect(fetchScanModels()).rejects.toThrow(/Failed to load models/);
  });
});
