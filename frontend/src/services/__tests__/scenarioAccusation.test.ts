/**
 * Service tests for the accusation client (task 2.11 B1).
 *
 * ## The URL guards, and the failure class they exist for
 *
 * Roman's .377 build shipped a client calling a path the router did not serve: a
 * whole feature answering 404, with nothing on either side saying why. The server
 * half of this guard lives in `api::scenario_accusation`'s tests; this is the
 * client half. Between them a path can only drift if BOTH are edited to agree —
 * which is the point, and why the assertions below spell the paths out rather than
 * building them from the same helper the code under test uses.
 *
 * Standing Rule 1: a failed load and a shape mismatch each produce a distinct,
 * observable error. An empty gap list and a broken read look identical on screen
 * and are very different states.
 *
 * Mocks `global.fetch` because `authFetch` calls it. Pattern mirrors
 * scenarioCards.test.ts.
 */
import { afterEach, describe, expect, it, vi } from "vitest";

import {
  fetchAccusationPanel,
  fillDetail,
  markInstance,
  pairAnswer,
  setAccusationText,
  unmarkInstance,
  unpairAnswer,
  type AccusationPanelDto,
} from "../scenarioAccusation";

const SLUG = "awad_v_catholic_family_service";
const SCENARIO = "11111111-1111-1111-1111-111111111111";
const NODE = "ev-42";
const BASE = `/api/cases/${SLUG}/scenarios/${SCENARIO}/accusation`;

afterEach(() => {
  vi.restoreAllMocks();
});

/** A minimal but complete panel, matching the backend DTO. */
function panel(): AccusationPanelDto {
  return {
    accusation_text: "They say Marie refused to divide the property.",
    count_line: "Said 2 times, in 2 documents.",
    no_instances_notice: null,
    instances: [
      {
        graph_node_id: NODE,
        code: "C-42",
        answers_graph_node_id: null,
        answer_code: null,
        answer_present: false,
      },
    ],
    gaps: [
      {
        kind: "no_answer_prepared",
        graph_node_id: NODE,
        message: "NO ANSWER PREPARED — C-42",
      },
    ],
    wording: {
      section_heading: "The accusation, and every time they made it",
      section_hint: "Write the accusation in plain words.",
      text_label: "In plain words",
      text_placeholder: "They say…",
      text_missing_notice: "Nobody has written this accusation yet.",
      text_edit_label: "Edit",
      text_save_label: "Save",
      text_clear_label: "Withdraw it",
      text_cancel_label: "Cancel",
      mark_label: "Mark an instance",
      unmark_label: "Not an instance",
      pair_label: "Pair our answer",
      repair_label: "Change our answer",
      unpair_label: "Unpair",
      answer_label: "Our answer:",
      picker_prompt: "Choose one of this scenario's included facts.",
      picker_cancel_label: "Never mind",
      picker_no_match_notice: "No included fact matches what you typed.",
      picker_empty_notice: "There is nothing left to choose.",
      gaps_heading: "What still needs preparing",
      no_gaps_notice: "Every instance has an answer.",
      save_failed_template: "That did not save: {detail}",
    },
  };
}

/** Mock `fetch` with one OK response, and hand back the spy. */
function okFetch(body: unknown = {}) {
  const mock = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => body });
  // @ts-ignore
  global.fetch = mock;
  return mock;
}

describe("fetchAccusationPanel", () => {
  it("GETs the scenario-scoped accusation URL and returns the payload", async () => {
    const mock = okFetch(panel());

    await expect(fetchAccusationPanel(SLUG, SCENARIO)).resolves.toEqual(panel());

    const [url] = mock.mock.calls[0];
    expect(url).toContain(BASE);
  });

  it("throws with the scenario and status on a non-OK response", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({ ok: false, status: 500 });

    await expect(fetchAccusationPanel(SLUG, SCENARIO)).rejects.toThrow(
      new RegExp(`accusation for scenario "${SCENARIO}".*\\(HTTP 500`),
    );
  });

  it("surfaces the backend message on a 4xx", async () => {
    // @ts-ignore
    global.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 404,
      json: async () => ({ message: "scenario not found" }),
    });

    await expect(fetchAccusationPanel(SLUG, SCENARIO)).rejects.toThrow(/scenario not found/);
  });

  it("throws on a missing wording block rather than rendering unlabelled controls", async () => {
    // R4: there is no literal to fall back to. A panel with no words is a column
    // of blank buttons, which is worse than a stated failure.
    const { wording: _dropped, ...rest } = panel();
    okFetch(rest);

    await expect(fetchAccusationPanel(SLUG, SCENARIO)).rejects.toThrow(
      /missing instances\/gaps\/wording — backend\/frontend contract mismatch/,
    );
  });

  it("throws on a missing gap list rather than showing a clean prep list", async () => {
    // The failure this guard exists for: a renamed list would render as "nothing
    // still needs preparing", which is the most dangerous sentence this page can
    // say wrongly.
    const { gaps: _dropped, ...rest } = panel();
    okFetch(rest);

    await expect(fetchAccusationPanel(SLUG, SCENARIO)).rejects.toThrow(
      /backend\/frontend contract mismatch/,
    );
  });

  it("accepts an empty, nothing-marked panel as a valid distinct answer", async () => {
    // A fresh scenario. It must NOT throw — only a malformed shape does.
    const fresh: AccusationPanelDto = {
      ...panel(),
      accusation_text: null,
      count_line: null,
      no_instances_notice: "No instances marked yet. 46 included facts are waiting here.",
      instances: [],
      gaps: [],
    };
    okFetch(fresh);

    await expect(fetchAccusationPanel(SLUG, SCENARIO)).resolves.toEqual(fresh);
  });
});

describe("the write URLs and methods", () => {
  it("PUTs the accusation sentence to the section's own URL", async () => {
    const mock = okFetch();
    await setAccusationText(SLUG, SCENARIO, "They say Marie was unreasonable.");

    const [url, init] = mock.mock.calls[0];
    expect(url).toContain(BASE);
    expect(init.method).toBe("PUT");
    expect(JSON.parse(init.body)).toEqual({ text: "They say Marie was unreasonable." });
  });

  it("sends null to withdraw the sentence, never an empty string", async () => {
    // The backend refuses "" on purpose: withdrawing and mistyping are different
    // intentions. A client that sent "" would turn a real act into a 400.
    const mock = okFetch();
    await setAccusationText(SLUG, SCENARIO, null);

    expect(JSON.parse(mock.mock.calls[0][1].body)).toEqual({ text: null });
  });

  it("POSTs to mark an instance and DELETEs to withdraw one", async () => {
    const mark = okFetch();
    await markInstance(SLUG, SCENARIO, NODE);
    expect(mark.mock.calls[0][0]).toContain(`${BASE}/instances/${NODE}`);
    expect(mark.mock.calls[0][1].method).toBe("POST");

    const unmark = okFetch();
    await unmarkInstance(SLUG, SCENARIO, NODE);
    expect(unmark.mock.calls[0][0]).toContain(`${BASE}/instances/${NODE}`);
    expect(unmark.mock.calls[0][1].method).toBe("DELETE");
  });

  it("PUTs the answer to the instance's own answer URL and DELETEs to un-pair", async () => {
    const pair = okFetch();
    await pairAnswer(SLUG, SCENARIO, NODE, "ev-99");
    expect(pair.mock.calls[0][0]).toContain(`${BASE}/instances/${NODE}/answer`);
    expect(pair.mock.calls[0][1].method).toBe("PUT");
    expect(JSON.parse(pair.mock.calls[0][1].body)).toEqual({
      answers_graph_node_id: "ev-99",
    });

    const unpair = okFetch();
    await unpairAnswer(SLUG, SCENARIO, NODE);
    expect(unpair.mock.calls[0][0]).toContain(`${BASE}/instances/${NODE}/answer`);
    expect(unpair.mock.calls[0][1].method).toBe("DELETE");
  });

  it("percent-encodes every path segment", async () => {
    // A graph node id is opaque and a slug is data. Either could carry a
    // character that changes what path is requested.
    const mock = okFetch();
    await markInstance("a/b", SCENARIO, "ev/42?x");

    const [url] = mock.mock.calls[0];
    expect(url).toContain("a%2Fb");
    expect(url).toContain("ev%2F42%3Fx");
  });

  it("throws on every non-OK write rather than reporting a save that did not happen", async () => {
    // Standing Rule 1, applied to all five writes at once: a silent failure here
    // leaves a mark on screen with nothing behind it.
    for (const call of [
      () => setAccusationText(SLUG, SCENARIO, "x"),
      () => markInstance(SLUG, SCENARIO, NODE),
      () => unmarkInstance(SLUG, SCENARIO, NODE),
      () => pairAnswer(SLUG, SCENARIO, NODE, "ev-99"),
      () => unpairAnswer(SLUG, SCENARIO, NODE),
    ]) {
      // @ts-ignore
      global.fetch = vi.fn().mockResolvedValue({ ok: false, status: 409 });
      await expect(call()).rejects.toThrow(/HTTP 409/);
    }
  });

  it("names no path the gateway already adds", async () => {
    // The .377 failure class from the client side: `/api/api/…` is reachable by
    // nothing, and on screen it is indistinguishable from a feature nobody built.
    const mock = okFetch();
    await pairAnswer(SLUG, SCENARIO, NODE, "ev-99");
    expect(String(mock.mock.calls[0][0])).not.toContain("/api/api/");
  });
});

describe("fillDetail", () => {
  it("puts the failure's own words into the stored sentence", () => {
    expect(fillDetail("That did not save: {detail} Reload.", "HTTP 500")).toBe(
      "That did not save: HTTP 500 Reload.",
    );
  });

  it("leaves a template with no slot untouched rather than appending", () => {
    // The stored value is Roman's to edit, and the write path guarantees the slot
    // survives. If one ever did not, the honest render is the sentence as written
    // — not a sentence with a detail bolted onto the end of it.
    expect(fillDetail("That did not save.", "HTTP 500")).toBe("That did not save.");
  });
});
