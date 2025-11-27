import { describe, it, expect, vi, afterEach } from "vitest";
import { getClaims } from "../claims";

describe("getClaims", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("returns claims when fetch succeeds with data", async () => {
    const mockData = [
      { id: "1", title: "Claim 1", description: "desc", status: "open" },
      { id: "2", title: "Claim 2", status: "pending" },
    ];

    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => mockData,
    });

    const claims = await getClaims();

    expect(claims).toEqual(mockData);
  });

  it("returns empty array when fetch succeeds with no claims", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => [],
    });

    const claims = await getClaims();

    expect(claims).toEqual([]);
  });

  it("throws when fetch responds with non-OK status", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      json: async () => {
        throw new Error("Should not be called");
      },
    });

    await expect(getClaims()).rejects.toThrow(/Failed to fetch claims: 500/);
  });

  it("throws when response body is not JSON array", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ message: "not an array" }),
    });

    await expect(getClaims()).rejects.toThrow(/Invalid claims response shape/);
  });

  it("throws when fetch rejects", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockRejectedValue(new Error("network down"));

    await expect(getClaims()).rejects.toThrow(/network down/);
  });
});
