/**
 * Tests for the fact card's field write (FACT_CARD_v2 §2).
 *
 * Mocks `global.fetch` because `authFetch` ultimately calls it — the pattern
 * `elementDetailService.test.ts` and `causesOfAction.test.ts` both use.
 *
 * The editor CLOSES on save and the row shows the new sentence, so what matters
 * most here is that a FAILURE is loud: the only alternative is a screen showing a
 * sentence the database does not hold, on a deck a witness reads from on the
 * stand (Standing Rule 1).
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { saveCardField } from "../factCards";
import { API_BASE_URL } from "../api";

const SLUG = "awad_v_catholic_family_service";
const SCENARIO = "f5849675-b0c7-4f5f-a383-f4aefedfa8eb";
const NODE = "doc-judge-tighe-opinion-and-order-041212:evidence:b49268dd";

function mockStatus(status: number) {
  const fetchMock = vi.fn().mockResolvedValue({
    ok: status >= 200 && status < 300,
    status,
  });
  // @ts-ignore — minimal mock of the fetch Response we use
  global.fetch = fetchMock;
  return fetchMock;
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("saveCardField", () => {
  it("PUTs one field to the card's own address", async () => {
    const fetchMock = mockStatus(200);
    await saveCardField(SLUG, SCENARIO, NODE, {
      field: "answer",
      value: "The money was Dad's.",
    });

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe(
      `${API_BASE_URL}/api/cases/${SLUG}/scenarios/${SCENARIO}/facts/${encodeURIComponent(NODE)}/card`,
    );
    expect(init.method).toBe("PUT");
    expect(JSON.parse(init.body)).toEqual({
      field: "answer",
      value: "The money was Dad's.",
    });
  });

  it("encodes a node id carrying colons", async () => {
    // An Evidence id is a content hash of the shape `doc-…:evidence:b49268dd`.
    // Unencoded, the colons would still resolve — and the day one carries a slash
    // it would address a different route.
    const fetchMock = mockStatus(200);
    await saveCardField(SLUG, SCENARIO, NODE, { field: "title", value: "x" });
    expect(fetchMock.mock.calls[0][0]).toContain("%3Aevidence%3A");
  });

  it("sends null to clear a field", async () => {
    const fetchMock = mockStatus(200);
    await saveCardField(SLUG, SCENARIO, NODE, { field: "watch_out", value: null });
    expect(JSON.parse(fetchMock.mock.calls[0][1].body)).toEqual({
      field: "watch_out",
      value: null,
    });
  });

  it("sends the accusation list as typed objects", async () => {
    const fetchMock = mockStatus(200);
    await saveCardField(SLUG, SCENARIO, NODE, {
      field: "supports",
      value: [{ allegation_id: "a-1", stance: "rebuts" }],
    });
    expect(JSON.parse(fetchMock.mock.calls[0][1].body).value).toEqual([
      { allegation_id: "a-1", stance: "rebuts" },
    ]);
  });

  it("throws a permission message on 403 rather than a fault message", async () => {
    // A 403 is not something broken — it is a permission the person does not
    // have, and telling them it failed sends them to an operator for nothing.
    mockStatus(403);
    await expect(
      saveCardField(SLUG, SCENARIO, NODE, { field: "answer", value: "x" }),
    ).rejects.toThrow(/permission/);
  });

  it("says the scenario is gone on a 404", async () => {
    mockStatus(404);
    await expect(
      saveCardField(SLUG, SCENARIO, NODE, { field: "answer", value: "x" }),
    ).rejects.toThrow(/no longer in this case/);
  });

  it("says the value was refused on a 400", async () => {
    mockStatus(400);
    await expect(
      saveCardField(SLUG, SCENARIO, NODE, {
        field: "supports",
        value: [
          { allegation_id: "a", stance: "supports" },
          { allegation_id: "b", stance: "supports" },
          { allegation_id: "c", stance: "supports" },
        ],
      }),
    ).rejects.toThrow(/refused that value/);
  });

  it("throws and names the status on any other non-2xx", async () => {
    mockStatus(500);
    await expect(
      saveCardField(SLUG, SCENARIO, NODE, { field: "title", value: "x" }),
    ).rejects.toThrow(/500/);
  });

  it("resolves silently on success — the caller re-reads the deck", async () => {
    mockStatus(200);
    await expect(
      saveCardField(SLUG, SCENARIO, NODE, { field: "title", value: "x" }),
    ).resolves.toBeUndefined();
  });
});
