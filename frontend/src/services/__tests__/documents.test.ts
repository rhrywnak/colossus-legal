import { describe, it, expect, vi, afterEach } from "vitest";
import { getDocuments } from "../documents";

describe("getDocuments", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("returns documents when fetch succeeds with data", async () => {
    const mockData = [
      { id: "1", title: "Doc 1", doc_type: "complaint", created_at: "2024-01-01" },
      { id: "2", title: "Doc 2", doc_type: "exhibit" },
    ];

    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => mockData,
    });

    const documents = await getDocuments();

    expect(documents).toEqual([
      { id: "1", title: "Doc 1", docType: "complaint", createdAt: "2024-01-01" },
      { id: "2", title: "Doc 2", docType: "exhibit", createdAt: undefined },
    ]);
  });

  it("returns empty array when fetch succeeds with no documents", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => [],
    });

    const documents = await getDocuments();

    expect(documents).toEqual([]);
  });

  it("throws when fetch responds with non-OK status", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      statusText: "Server error",
      json: async () => {
        throw new Error("Should not be called");
      },
    });

    await expect(getDocuments()).rejects.toThrow(/Failed to fetch documents: 500/);
  });

  it("throws when response body is not JSON array", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ message: "not an array" }),
    });

    await expect(getDocuments()).rejects.toThrow(/Invalid documents response shape/);
  });

  it("throws when fetch rejects", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockRejectedValue(new Error("network down"));

    await expect(getDocuments()).rejects.toThrow(/network down/);
  });
});
