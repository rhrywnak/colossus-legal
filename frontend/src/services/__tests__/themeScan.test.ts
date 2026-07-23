/**
 * Service tests for the background Theme Scan client (start + poll).
 * Mocks global.fetch (authFetch calls it). Standing Rule 1: a non-OK response
 * throws with the backend message surfaced VERBATIM. Mirrors scenarioGather.test.ts.
 */
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  deleteScanRun,
  fetchScanModels,
  fetchScanRuns,
  getScanRun,
  mergeScanRun,
  startThemeScan,
} from "../themeScan";

const SLUG = "awad";
const SCENARIO = "11111111-1111-1111-1111-111111111111";
const RUN = "22222222-2222-2222-2222-222222222222";

afterEach(() => {
  vi.restoreAllMocks();
});

describe("startThemeScan", () => {
  it("POSTs the theme-scan URL with the model body", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ run_id: RUN, status: "running", candidates_total: 94 }),
    });
    // @ts-ignore — minimal fetch mock
    global.fetch = fetchMock;

    await expect(
      startThemeScan(SLUG, SCENARIO, { model_id: "qwen-14b" }),
    ).resolves.toEqual({ run_id: RUN, status: "running", candidates_total: 94 });

    const [url, options] = fetchMock.mock.calls[0];
    expect(url).toContain(`/api/cases/${SLUG}/scenarios/${SCENARIO}/theme-scan`);
    expect(options.method).toBe("POST");
    // Exactly the model — no dry_run. The backend rejects unknown keys with a 400,
    // so sending a retired field would be a hard failure, not a harmless extra.
    expect(JSON.parse(options.body)).toEqual({ model_id: "qwen-14b" });
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
      startThemeScan(SLUG, SCENARIO, {}),
    ).rejects.toThrow(/selected 'qwen-14b' but loaded 'qwen-7b'/);
  });
});

describe("getScanRun", () => {
  it("GETs the scan-runs URL and returns the typed status", async () => {
    const status = {
      run_id: RUN,
      status: "running",
      model_id: "qwen-14b",
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

describe("fetchScanRuns", () => {
  const header = {
    run_id: RUN,
    model_id: "qwen-14b",
    status: "completed",
    candidates_total: 94,
    candidates_judged: 94,
    relevant_count: 31,
    irrelevant_count: 60,
    failed_count: 3,
    computed_cost: 0.0125,
    duration_ms: 45000,
    started_at: "2026-07-16T14:32:00Z",
  };

  it("GETs the scan-runs list URL and unwraps { runs }", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ runs: [header] }),
    });
    // @ts-ignore — minimal fetch mock
    global.fetch = fetchMock;

    await expect(fetchScanRuns(SLUG, SCENARIO)).resolves.toEqual([header]);
    // The list URL is the scan-runs collection (no :run_id suffix).
    expect(fetchMock.mock.calls[0][0]).toContain(
      `/api/cases/${SLUG}/scenarios/${SCENARIO}/scan-runs`,
    );
    expect(fetchMock.mock.calls[0][0]).not.toContain(`/scan-runs/${RUN}`);
  });

  it("returns [] when the runs key is absent (unscanned scenario)", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => ({}) });
    await expect(fetchScanRuns(SLUG, SCENARIO)).resolves.toEqual([]);
  });

  it("throws on a non-OK response", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({ ok: false, status: 500, json: async () => ({}) });
    await expect(fetchScanRuns(SLUG, SCENARIO)).rejects.toThrow(/Failed to load scan history/);
  });
});

describe("deleteScanRun", () => {
  it("DELETEs the per-run URL and resolves on a 204", async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, status: 204, json: async () => ({}) });
    // @ts-ignore — minimal fetch mock
    global.fetch = fetchMock;

    await expect(deleteScanRun(SLUG, SCENARIO, RUN)).resolves.toBeUndefined();

    const [url, options] = fetchMock.mock.calls[0];
    expect(url).toContain(`/api/cases/${SLUG}/scenarios/${SCENARIO}/scan-runs/${RUN}`);
    expect(options.method).toBe("DELETE");
  });

  it("throws with the backend message on a non-2xx (e.g. 404 not found)", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 404,
      json: async () => ({ message: "scan run not found" }),
    });
    await expect(deleteScanRun(SLUG, SCENARIO, RUN)).rejects.toThrow(
      /Failed to delete scan run.*scan run not found/,
    );
  });
});

describe("mergeScanRun", () => {
  it("POSTs the merge URL and returns the merged count", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ merged: 12 }),
    });
    // @ts-ignore — minimal fetch mock
    global.fetch = fetchMock;

    await expect(
      mergeScanRun(SLUG, SCENARIO, RUN, ["ev-a", "ev-b"]),
    ).resolves.toEqual({ merged: 12 });

    const [url, options] = fetchMock.mock.calls[0];
    expect(url).toContain(`/api/cases/${SLUG}/scenarios/${SCENARIO}/scan-runs/${RUN}/merge`);
    expect(options.method).toBe("POST");
    // The selected picks ride in the body as graph_node_ids (selective merge).
    expect(JSON.parse(options.body)).toEqual({ graph_node_ids: ["ev-a", "ev-b"] });
  });

  it("throws with the backend message on a non-2xx (e.g. 404 not found)", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 404,
      json: async () => ({ message: "scan run not found" }),
    });
    await expect(mergeScanRun(SLUG, SCENARIO, RUN, ["ev-a"])).rejects.toThrow(
      /Failed to merge scan run.*scan run not found/,
    );
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
    const fetchMock = global.fetch as unknown as ReturnType<typeof vi.fn>;
    await expect(fetchScanModels()).resolves.toEqual([
      { model_id: "m1", display_name: "M1", is_default: true },
    ]);
    expect(fetchMock.mock.calls[0][0]).toContain("/api/scan/models");

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
