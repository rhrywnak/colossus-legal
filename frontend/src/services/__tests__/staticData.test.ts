/**
 * Service tests for the static-JSON loaders.
 *
 * Covers the shared fetch helper (fetchStaticJson) and the two thin validators
 * built on it (getCaseSummaryDoc, getCaseTimeline). Standing Rule 1 (no silent
 * failures): every distinct failure path — network/timeout, non-OK status,
 * unparseable body, and missing-required-shape — produces its own observable
 * error. Mocks `global.fetch` (these loaders call raw fetch, not authFetch,
 * because static files are same-origin and uncredentialed).
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { fetchStaticJson } from "../staticData";
import { getCaseSummaryDoc } from "../caseSummaryDoc";
import { getCaseTimeline } from "../caseTimeline";

afterEach(() => {
  vi.restoreAllMocks();
});

describe("fetchStaticJson", () => {
  it("returns the parsed body on a 2xx with valid JSON", async () => {
    // @ts-ignore — minimal Response mock
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ hello: "world" }),
    });

    await expect(fetchStaticJson("/data/x.json", "thing")).resolves.toEqual({
      hello: "world",
    });
  });

  it("throws a contextual error on a non-OK status", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({ ok: false, status: 404, json: async () => ({}) });

    await expect(fetchStaticJson("/data/x.json", "thing")).rejects.toThrow(
      /Failed to load thing from \/data\/x\.json \(HTTP 404\)/,
    );
  });

  it("throws when the body is not valid JSON", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => {
        throw new Error("Unexpected token < in JSON");
      },
    });

    await expect(fetchStaticJson("/data/x.json", "thing")).rejects.toThrow(
      /thing at \/data\/x\.json was not valid JSON/,
    );
  });

  it("throws a contextual error when fetch rejects (network/timeout)", async () => {
    global.fetch = vi.fn().mockRejectedValue(new Error("aborted"));

    await expect(fetchStaticJson("/data/x.json", "thing")).rejects.toThrow(
      /Failed to load thing from \/data\/x\.json \(aborted\)/,
    );
  });
});

describe("getCaseSummaryDoc", () => {
  const valid = {
    summary: "s",
    venue: "v",
    filed: "f",
    status: "active",
    count_descriptions: { "1": "first count" },
  };

  it("returns the validated doc when all fields are present", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => valid });

    await expect(getCaseSummaryDoc()).resolves.toEqual(valid);
  });

  it("accepts an empty count_descriptions object (per-key entries are optional)", async () => {
    const doc = { ...valid, count_descriptions: {} };
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => doc });

    await expect(getCaseSummaryDoc()).resolves.toEqual(doc);
  });

  it("throws when a required string field is missing", async () => {
    // @ts-ignore — missing `status`
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ summary: "s", venue: "v", filed: "f", count_descriptions: {} }),
    });

    await expect(getCaseSummaryDoc()).rejects.toThrow(/missing required fields/);
  });

  it("throws when count_descriptions is absent", async () => {
    // @ts-ignore — all strings present, container missing
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ summary: "s", venue: "v", filed: "f", status: "active" }),
    });

    await expect(getCaseSummaryDoc()).rejects.toThrow(/missing required fields/);
  });

  it("throws when count_descriptions is an array, not an object", async () => {
    // @ts-ignore — arrays are objects in JS; the validator must reject them
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ ...valid, count_descriptions: [] }),
    });

    await expect(getCaseSummaryDoc()).rejects.toThrow(/missing required fields/);
  });
});

describe("getCaseTimeline", () => {
  it("returns the validated timeline when phases and events are arrays", async () => {
    const valid = { phases: [], events: [] };
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => valid });

    await expect(getCaseTimeline()).resolves.toEqual(valid);
  });

  it("throws when phases is not an array (left branch of the guard)", async () => {
    // @ts-ignore — phases missing, events valid
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ events: [] }),
    });

    await expect(getCaseTimeline()).rejects.toThrow(/missing required arrays/);
  });

  it("throws when events is not an array (right branch of the guard)", async () => {
    // @ts-ignore — phases valid, events missing: exercises the second operand
    // of the `||` independently so both shape-failures are covered.
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ phases: [] }),
    });

    await expect(getCaseTimeline()).rejects.toThrow(/missing required arrays/);
  });
});
