/**
 * Service + helper tests for the Case Health inventory client.
 *
 * `getCaseHealthInventory` exercises the four outcomes of the
 * GET /api/cases/:slug/case-health/inventory client: a valid payload, and each
 * of the three throw paths (non-OK status, unparseable body, missing
 * `current`/`edge_classes`). Standing Rule 1 (no silent failures): every failure
 * path produces a distinct, observable error. Mocks `global.fetch` because
 * `authFetch` calls it. Pattern matches proofMatrix.test.ts.
 *
 * `formatRate` and `edgeClassCount` are pure display helpers — tested directly.
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  edgeClassCount,
  formatRate,
  getCaseHealthInventory,
  type CaseHealthInventoryResponse,
  type DocumentConnection,
} from "../caseHealth";

const transcriptRow: DocumentConnection = {
  document_id: "doc-hearing-2009",
  title: "Hearing to approve plan for administration",
  doc_type: "court_transcript",
  evidence_total: 138,
  probative_connected: 8,
  topical_connected: 12,
  inert: 126,
  probative_rate_pct: 5.8,
  topical_rate_pct: 8.7,
  inert_rate_pct: 91.3,
  by_edge_class: [
    { rel_type: "CORROBORATES", item_count: 3 },
    { rel_type: "REBUTS", item_count: 4 },
    { rel_type: "CHARACTERIZES", item_count: 2 },
    { rel_type: "ABOUT", item_count: 6 },
  ],
};

const validResponse: CaseHealthInventoryResponse = {
  case_slug: "awad_v_catholic_family_service",
  connection_tier_lookup_v: 1,
  edge_classes: ["CORROBORATES", "REBUTS", "CHARACTERIZES", "ABOUT"],
  current: {
    computed_at: "2026-07-27T12:00:00+00:00",
    corpus: {
      evidence_total: 525,
      probative_connected: 103,
      topical_connected: 126,
      inert: 399,
      probative_rate_pct: 19.6,
      topical_rate_pct: 24.0,
      inert_rate_pct: 76.0,
      evidence_without_document: 0,
    },
    node_labels: [{ label: "Evidence", node_count: 525 }],
    unlabeled_node_count: 0,
    edge_triples: [
      {
        from_label: "Evidence",
        rel_type: "ABOUT",
        to_label: "Person",
        edge_count: 614,
      },
    ],
    documents: [transcriptRow],
  },
};

describe("getCaseHealthInventory", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("returns the typed payload when the response is valid", async () => {
    // @ts-ignore — minimal mock of the fetch Response we use
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => validResponse,
    });

    await expect(getCaseHealthInventory()).resolves.toEqual(validResponse);
  });

  it("throws a contextual error naming the slug on a non-OK status", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      json: async () => {
        throw new Error("should not be called on non-OK");
      },
    });

    await expect(getCaseHealthInventory("some_case")).rejects.toThrow(
      /Failed to load case health inventory for "some_case" \(HTTP 500\)/,
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

    await expect(getCaseHealthInventory("some_case")).rejects.toThrow(
      /was not valid JSON/,
    );
  });

  it("throws a contract-mismatch error when `current` is missing", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        case_slug: "some_case",
        connection_tier_lookup_v: 1,
        edge_classes: ["CORROBORATES"],
      }),
    });

    await expect(getCaseHealthInventory("some_case")).rejects.toThrow(
      /missing its "current" measurement or "edge_classes" list/,
    );
  });

  it("throws a contract-mismatch error when `edge_classes` is missing", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        case_slug: "some_case",
        connection_tier_lookup_v: 1,
        current: validResponse.current,
      }),
    });

    await expect(getCaseHealthInventory("some_case")).rejects.toThrow(
      /missing its "current" measurement or "edge_classes" list/,
    );
  });

  it("throws when `current.documents` is not an array", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        case_slug: "some_case",
        connection_tier_lookup_v: 1,
        edge_classes: ["CORROBORATES"],
        current: { ...validResponse.current, documents: undefined },
      }),
    });

    await expect(getCaseHealthInventory("some_case")).rejects.toThrow(
      /backend\/frontend contract mismatch/,
    );
  });

  it("accepts an honestly empty graph — zeroed counts and null rates are not an error", async () => {
    const empty: CaseHealthInventoryResponse = {
      case_slug: "new_case",
      connection_tier_lookup_v: 1,
      edge_classes: ["CORROBORATES", "REBUTS", "CHARACTERIZES", "ABOUT"],
      current: {
        computed_at: "2026-07-27T12:00:00+00:00",
        corpus: {
          evidence_total: 0,
          probative_connected: 0,
          topical_connected: 0,
          inert: 0,
          probative_rate_pct: null,
          topical_rate_pct: null,
          inert_rate_pct: null,
          evidence_without_document: 0,
        },
        node_labels: [],
        unlabeled_node_count: 0,
        edge_triples: [],
        documents: [],
      },
    };
    // @ts-ignore
    global.fetch = vi
      .fn()
      .mockResolvedValue({ ok: true, status: 200, json: async () => empty });

    await expect(getCaseHealthInventory("new_case")).resolves.toEqual(empty);
  });

  it("URL-encodes the slug", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValue({ ok: true, status: 200, json: async () => validResponse });
    // @ts-ignore
    global.fetch = fetchMock;

    await getCaseHealthInventory("a b/c");

    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(String(fetchMock.mock.calls[0][0])).toContain(
      "/api/cases/a%20b%2Fc/case-health/inventory",
    );
  });
});

describe("formatRate", () => {
  it("renders a computed rate to one decimal place with a percent sign", () => {
    expect(formatRate(19.6)).toBe("19.6%");
    expect(formatRate(24)).toBe("24.0%");
    expect(formatRate(100)).toBe("100.0%");
  });

  /**
   * The distinction the backend went to the trouble of preserving: `null` means
   * "there was nothing to measure", which is NOT the same statement as 0.0%.
   * Collapsing them would erase a real difference between a document that
   * produced no Evidence and one whose Evidence all failed to connect.
   */
  it("renders an absent rate as an em-dash, never as 0.0%", () => {
    expect(formatRate(null)).toBe("—");
    expect(formatRate(undefined)).toBe("—");
  });

  it("renders a genuine zero as 0.0%, distinct from the em-dash", () => {
    expect(formatRate(0)).toBe("0.0%");
  });
});

describe("edgeClassCount", () => {
  it("resolves a class by name, not by position", () => {
    expect(edgeClassCount(transcriptRow, "CORROBORATES")).toBe(3);
    expect(edgeClassCount(transcriptRow, "REBUTS")).toBe(4);
    expect(edgeClassCount(transcriptRow, "CHARACTERIZES")).toBe(2);
    expect(edgeClassCount(transcriptRow, "ABOUT")).toBe(6);
  });

  /**
   * A class the row has no entry for yields `null`, which the table renders as
   * an em-dash. Returning 0 would assert a measurement that was never made —
   * the same fabrication Standing Rule 1 forbids on the backend.
   */
  it("returns null for a class the row does not carry", () => {
    expect(edgeClassCount(transcriptRow, "CONTRADICTS")).toBeNull();
  });

  it("returns a genuine zero as 0, distinct from a missing entry", () => {
    const row: DocumentConnection = {
      ...transcriptRow,
      by_edge_class: [{ rel_type: "CORROBORATES", item_count: 0 }],
    };
    expect(edgeClassCount(row, "CORROBORATES")).toBe(0);
    expect(edgeClassCount(row, "REBUTS")).toBeNull();
  });
});
