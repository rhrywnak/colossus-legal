import { describe, it, expect } from "vitest";
import { getClaimsStub } from "../claims";

describe("getClaimsStub", () => {
  it("returns a non-empty array of stubbed claims", async () => {
    const claims = await getClaimsStub();

    expect(Array.isArray(claims)).toBe(true);
    expect(claims.length).toBeGreaterThan(0);
    expect(claims[0]).toMatchObject({
      id: "claim-1",
      title: "Breach of contract",
      status: "open",
    });
  });
});
