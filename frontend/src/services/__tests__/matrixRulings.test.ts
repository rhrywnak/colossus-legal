/**
 * Tests for the Proof Matrix's ruling writes (PROOF_MATRIX_v2 §2).
 *
 * Mocks `global.fetch` because `authFetch` ultimately calls it — the pattern
 * `elementDetailService.test.ts` and `causesOfAction.test.ts` both use.
 *
 * The row updates optimistically, so what matters most here is that a FAILURE is
 * loud: every non-2xx, unparseable or unrecognisable reply must throw, because
 * the only alternative is a screen that disagrees with the database and says
 * nothing (Standing Rule 1).
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  countExportUrl,
  saveRuling,
  withdrawRuling,
} from "../matrixRulings";
import { API_BASE_URL } from "../api";

const SLUG = "awad_v_catholic_family_service";
const EVIDENCE = "doc-george-phillips-admissions-response:evidence:01d3e125";
const ALLEGATION = "allegation-41";

function mockJson(status: number, body: unknown) {
  const fetchMock = vi.fn().mockResolvedValue({
    ok: status >= 200 && status < 300,
    status,
    json: async () => body,
  });
  // @ts-ignore — minimal mock of the fetch Response we use
  global.fetch = fetchMock;
  return fetchMock;
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("saveRuling", () => {
  it("PUTs the pair and the verdict, and returns what the server did", async () => {
    const fetchMock = mockJson(200, {
      evidence_id: EVIDENCE,
      allegation_id: ALLEGATION,
      action: "rule",
      changed: true,
    });

    const result = await saveRuling(SLUG, EVIDENCE, ALLEGATION, "keep");
    expect(result.action).toBe("rule");
    expect(result.changed).toBe(true);

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe(
      `${API_BASE_URL}/api/cases/${SLUG}/proof-matrix/rulings`,
    );
    expect(init.method).toBe("PUT");
    // The pair travels in the BODY, on purpose: an Evidence id is a content hash
    // carrying colons, and two of those in a path would make every click depend
    // on the browser and the router agreeing about percent-encoding.
    expect(JSON.parse(init.body)).toEqual({
      evidence_id: EVIDENCE,
      allegation_id: ALLEGATION,
      ruling: "keep",
    });
  });

  it("throws a permission message on 403 rather than a fault message", async () => {
    // A 403 is not something broken — it is a permission the person does not
    // have, and telling them it is a failure sends them to an operator.
    mockJson(403, {});
    await expect(saveRuling(SLUG, EVIDENCE, ALLEGATION, "keep")).rejects.toThrow(
      /permission/,
    );
  });

  it("throws and names the status on any other non-2xx", async () => {
    mockJson(500, {});
    await expect(saveRuling(SLUG, EVIDENCE, ALLEGATION, "remove")).rejects.toThrow(
      /500/,
    );
  });

  it("throws when the reply is not JSON", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => {
        throw new SyntaxError("not json");
      },
    });
    // @ts-ignore — minimal mock
    global.fetch = fetchMock;
    await expect(saveRuling(SLUG, EVIDENCE, ALLEGATION, "keep")).rejects.toThrow(
      /valid JSON/,
    );
  });

  it("throws when the reply does not say what it did", async () => {
    // A 200 this build cannot read means the write MAY have happened. Leaving
    // the row where the click put it, on the strength of a reply nobody parsed,
    // is the one outcome that cannot be recovered by looking at the screen.
    mockJson(200, { ok: "sure" });
    await expect(saveRuling(SLUG, EVIDENCE, ALLEGATION, "keep")).rejects.toThrow(
      /did not say what it did/,
    );
  });
});

describe("withdrawRuling", () => {
  it("DELETEs the pair and omits the verdict", async () => {
    const fetchMock = mockJson(200, {
      evidence_id: EVIDENCE,
      allegation_id: ALLEGATION,
      action: "withdraw",
      changed: true,
    });

    await withdrawRuling(SLUG, EVIDENCE, ALLEGATION);
    const [, init] = fetchMock.mock.calls[0];
    expect(init.method).toBe("DELETE");
    // No `ruling` key: a withdrawal asserts nothing, and the ledger stores NULL
    // there for the same reason.
    expect(JSON.parse(init.body)).toEqual({
      evidence_id: EVIDENCE,
      allegation_id: ALLEGATION,
    });
  });

  it("reports a withdrawal that found nothing as changed:false", async () => {
    // Not an error — the caller wanted the item unruled and unruled is what it
    // is. It IS different: only this outcome means the view was stale.
    mockJson(200, {
      evidence_id: EVIDENCE,
      allegation_id: ALLEGATION,
      action: "none",
      changed: false,
    });
    const result = await withdrawRuling(SLUG, EVIDENCE, ALLEGATION);
    expect(result.changed).toBe(false);
    expect(result.action).toBe("none");
  });
});

describe("countExportUrl", () => {
  it("builds the export link for one Count", () => {
    expect(countExportUrl(SLUG, 3)).toBe(
      `${API_BASE_URL}/api/cases/${SLUG}/proof-matrix/export.docx?count=3`,
    );
  });

  it("encodes a slug that needs it", () => {
    expect(countExportUrl("a b/c", 1)).toContain("/api/cases/a%20b%2Fc/");
  });
});
