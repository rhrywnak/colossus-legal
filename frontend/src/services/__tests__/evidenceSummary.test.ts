/**
 * Service tests for the evidence-summary override client (task 1.7F Part B).
 *
 * Covers both calls — save (PUT) and revert (DELETE) — against the right URL,
 * plus their throw paths. Standing Rule 1: every non-OK response produces a
 * distinct, observable error carrying the backend's own words where it sent
 * any. Mocks `global.fetch` because `authFetch` calls it; pattern mirrors
 * scenarioFacts.test.ts.
 *
 * The URL assertions are load-bearing beyond typo-catching: the route has NO
 * scenario segment, because the override is case-wide (ruling R1). A future edit
 * that nests it under a scenario would make one C-code mean two things depending
 * on which page it was corrected from, and it would fail here first.
 */
import { afterEach, describe, expect, it, vi } from "vitest";

import { revertQuestionOverride, saveQuestionOverride } from "../evidenceSummary";

const SLUG = "awad_v_catholic_family_service";
const NODE = "ev-113";
const SUMMARY_URL = `/api/cases/${SLUG}/evidence/${NODE}/summary`;

afterEach(() => {
  vi.restoreAllMocks();
});

describe("saveQuestionOverride", () => {
  it("PUTs the text to the case-wide evidence URL", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        graph_node_id: NODE,
        summary_text: "Did you receive the certified letter?",
        authorship_label: "roman · 4 Aug 2026",
        overridden: true,
      }),
    });
    // @ts-ignore — minimal fetch mock
    global.fetch = fetchMock;

    const result = await saveQuestionOverride(SLUG, NODE, "Did you receive the certified letter?");

    const [url, options] = fetchMock.mock.calls[0];
    expect(url).toContain(SUMMARY_URL);
    expect(url, "the override is case-wide — no scenario in the path").not.toContain("/scenarios/");
    expect(options.method).toBe("PUT");
    expect(JSON.parse(options.body)).toEqual({
      summary_text: "Did you receive the certified letter?",
    });
    expect(result.overridden).toBe(true);
    expect(result.authorship_label).toBe("roman · 4 Aug 2026");
  });

  it("throws with the backend's own refusal when the text is blank", async () => {
    // The backend refuses a blank correction with a sentence naming the
    // alternative. Dropping it for a bare status code would leave the human
    // guessing what to do next.
    const fetchMock = vi.fn().mockResolvedValue({
      ok: false,
      status: 400,
      json: async () => ({
        message:
          "A corrected question cannot be blank. To restore the system's own wording, use revert instead.",
      }),
    });
    // @ts-ignore — minimal fetch mock
    global.fetch = fetchMock;

    await expect(saveQuestionOverride(SLUG, NODE, "   ")).rejects.toThrow(/use revert instead/);
  });

  it("says the text was NOT stored when the save fails", async () => {
    // The human still has their words on screen; the message must not imply
    // otherwise, or they will retype work that is still in the editor.
    const fetchMock = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      json: async () => ({ message: "failed to save the corrected question" }),
    });
    // @ts-ignore — minimal fetch mock
    global.fetch = fetchMock;

    await expect(saveQuestionOverride(SLUG, NODE, "text")).rejects.toThrow(/was not stored/);
  });

  it("still throws a usable error when the body is not JSON", async () => {
    // A proxy or a gateway can answer with HTML. The status code still has to
    // reach the human rather than a parse exception from inside the error path.
    const fetchMock = vi.fn().mockResolvedValue({
      ok: false,
      status: 502,
      json: async () => {
        throw new Error("Unexpected token < in JSON");
      },
    });
    // @ts-ignore — minimal fetch mock
    global.fetch = fetchMock;

    await expect(saveQuestionOverride(SLUG, NODE, "text")).rejects.toThrow(/HTTP 502/);
  });
});

describe("revertQuestionOverride", () => {
  it("DELETEs the override and reports that none is in force", async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ graph_node_id: NODE, overridden: false }),
    });
    // @ts-ignore — minimal fetch mock
    global.fetch = fetchMock;

    const result = await revertQuestionOverride(SLUG, NODE);

    const [url, options] = fetchMock.mock.calls[0];
    expect(url).toContain(SUMMARY_URL);
    expect(options.method).toBe("DELETE");
    expect(result.overridden).toBe(false);
    // Nothing comes back to render: the machine's sentence is in the graph and
    // the caller re-reads the card for it (ruling R2 — there is no copy here).
    expect(result.summary_text).toBeUndefined();
  });

  it("says the correction is still in place when the revert fails", async () => {
    // The opposite of the save path's message, and for the same reason: a human
    // must know which state they are actually in.
    const fetchMock = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      json: async () => ({ message: "failed to restore the system's question" }),
    });
    // @ts-ignore — minimal fetch mock
    global.fetch = fetchMock;

    await expect(revertQuestionOverride(SLUG, NODE)).rejects.toThrow(/still in place/);
  });
});
