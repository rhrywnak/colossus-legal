import { describe, it, expect, vi, afterEach } from "vitest";
import {
  getDocument,
  getDocuments,
  updateDocument,
  DocumentUpdateRequest,
} from "../documents";

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

describe("getDocument", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("returns a document when fetch succeeds", async () => {
    const mockData = { id: "1", title: "Doc 1", doc_type: "pdf", created_at: "2024-01-01" };
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => mockData,
    });

    const document = await getDocument("1");

    expect(document).toEqual({
      id: "1",
      title: "Doc 1",
      doc_type: "pdf",
      created_at: "2024-01-01",
      description: undefined,
      file_path: undefined,
      uploaded_at: undefined,
      related_claim_id: undefined,
      source_url: undefined,
    });
  });

  it("throws not_found on 404", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 404,
      json: async () => ({}),
    });

    await expect(getDocument("missing")).rejects.toMatchObject({ kind: "not_found" });
  });

  it("throws network on non-OK", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      json: async () => ({}),
    });

    await expect(getDocument("1")).rejects.toMatchObject({ kind: "network" });
  });
});

describe("updateDocument", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("returns updated document on success", async () => {
    const payload: DocumentUpdateRequest = { title: "Updated", doc_type: "motion" };
    const mockData = { id: "1", title: "Updated", doc_type: "motion" };

    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => mockData,
    });

    const updated = await updateDocument("1", payload);

    expect(updated.title).toBe("Updated");
    expect(updated.doc_type).toBe("motion");
  });

  it("throws validation on 400 with field info", async () => {
    const payload: DocumentUpdateRequest = { title: "", doc_type: "pdf" };
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 400,
      json: async () => ({
        error: "validation_error",
        message: "title must not be empty",
        details: { field: "title" },
      }),
    });

    await expect(updateDocument("1", payload)).rejects.toMatchObject({
      kind: "validation",
      field: "title",
    });
  });

  it("throws not_found on 404", async () => {
    const payload: DocumentUpdateRequest = { title: "Updated" };
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 404,
      json: async () => ({}),
    });

    await expect(updateDocument("missing", payload)).rejects.toMatchObject({
      kind: "not_found",
    });
  });

  it("throws network on 500", async () => {
    const payload: DocumentUpdateRequest = { title: "Updated" };
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      json: async () => ({}),
    });

    await expect(updateDocument("1", payload)).rejects.toMatchObject({
      kind: "network",
    });
  });
});
