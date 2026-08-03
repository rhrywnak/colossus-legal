/**
 * Tests for the working view's pure core (task 1.4).
 *
 * The row anatomy and the client-side search. Per CLAUDE.md rule 30 the frontend
 * has no DOM test infrastructure by deliberate convention, so the logic lives in
 * pure functions and is tested as data.
 */
import { describe, expect, it } from "vitest";

import { filterRows, includedRows, type WorkingRow } from "../factsTable";
import type { ScenarioCard } from "../../services/scenarioCards";

function card(overrides: Partial<ScenarioCard> = {}): ScenarioCard {
  return {
    code: "C-14",
    graph_node_id: "ev-1",
    quote: {
      text: "I do not recall that meeting.",
      context_before: "",
      context_after: "",
      // Task 1.7C (D6). An empty flank did not stop mid-sentence, so it counts as
      // complete and carries no page-edge notice — the backend's own rule for the
      // no-context case (`ContextFlank::absent`).
      context_before_complete: true,
      context_after_complete: true,
      context_before_notice: null,
      context_after_notice: null,
      question: null,
    },
    pinpoint: {
      document_id: "doc-7",
      document_title: "CFS responses",
      label: "CFS responses at 26",
      page: 26,
      viewer_href: "/documents/doc-7?page=26&tab=document",
    },
    speaker: { name: "R. Phillips", attribution: "extracted" },
    statement_kind: "admission",
    stance: null,
    bears_on: [{ accusation: "¶54 — CFS knew of the meeting", elements: [], count: null }],
    grounding: null,
    confidence: { band: "unscored", label: "Not scored by a scan" },
    status: "included",
    status_label: "In the scenario",
    defer_required: false,
    defer_required_reason: null,
    defer_reason: null,
    ...overrides,
  };
}

describe("includedRows", () => {
  it("keeps only the items a human put in the scenario", () => {
    const rows = includedRows([
      card({ graph_node_id: "in", status: "included" }),
      card({ graph_node_id: "undecided", status: "undecided" }),
      card({ graph_node_id: "aside", status: "dropped" }),
    ]);
    expect(rows.map((r) => r.graphNodeId)).toEqual(["in"]);
  });

  it("filters on the state token, never on the display label", () => {
    // The regression guard for the field pair added in 1.4: a card whose label
    // was reworded must still be included, because the STATE is what decides.
    const rows = includedRows([
      card({ status: "included", status_label: "Some future wording" }),
    ]);
    expect(rows).toHaveLength(1);
  });

  it("carries the payload's own strings into the row", () => {
    const source = card();
    const [row] = includedRows([source]);
    expect(row.text).toBe(source.quote.text);
    expect(row.pinpointLabel).toBe(source.pinpoint.label);
    expect(row.pinpointHref).toBe(source.pinpoint.viewer_href);
    expect(row.statusLabel).toBe(source.status_label);
    expect(row.bearsOn).toEqual(["¶54 — CFS knew of the meeting"]);
  });

  it("returns no rows when nothing has been included yet", () => {
    // A real state — the scenario is curated but nothing kept — not an error.
    expect(includedRows([card({ status: "undecided" })])).toEqual([]);
  });
});

describe("filterRows", () => {
  const rows: WorkingRow[] = [
    {
      code: "C-1",
      graphNodeId: "ev-1",
      text: "I do not recall that meeting.",
      bearsOn: ["¶54 — CFS knew of the meeting"],
      pinpointLabel: "CFS responses at 26",
      pinpointHref: "#",
      statusLabel: "In the scenario",
    },
    {
      code: "C-2",
      graphNodeId: "ev-2",
      text: "That would be correct.",
      bearsOn: ["¶12 — the contract was undisclosed"],
      pinpointLabel: "Phillips deposition at 88",
      pinpointHref: "#",
      statusLabel: "In the scenario",
    },
  ];

  it("an empty term shows everything", () => {
    // "No filter" and "a filter matching nothing" are different states.
    expect(filterRows(rows, "")).toHaveLength(2);
    expect(filterRows(rows, "   ")).toHaveLength(2);
  });

  it("matches the quote text, case-insensitively", () => {
    expect(filterRows(rows, "RECALL")).toHaveLength(1);
  });

  it("matches an accusation", () => {
    const found = filterRows(rows, "undisclosed");
    expect(found.map((r) => r.code)).toEqual(["C-2"]);
  });

  it("matches the pinpoint and the code", () => {
    expect(filterRows(rows, "deposition").map((r) => r.code)).toEqual(["C-2"]);
    expect(filterRows(rows, "c-1").map((r) => r.code)).toEqual(["C-1"]);
  });

  it("returns nothing when nothing matches — an honest empty result", () => {
    expect(filterRows(rows, "zzz")).toEqual([]);
  });

  it("never matches on a field the row does not display", () => {
    // Searching a hidden field would give a hit whose reason is invisible, which
    // reads as a broken filter.
    expect(filterRows(rows, "ev-1")).toEqual([]);
  });
});
