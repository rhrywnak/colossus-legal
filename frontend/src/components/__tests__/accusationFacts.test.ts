/**
 * Tests for `includedPickableFacts` — what the accusation section may offer.
 *
 * The rule this guards is not cosmetic. Only facts a human RULED IN may be marked
 * as instances of the accusation or paired as our answers: the rehearsal page is
 * built on the included evidence, and an undecided candidate reaching it would put
 * an unreviewed statement into the count, or into Marie's mouth as our answer.
 */
import { describe, expect, it } from "vitest";

import { includedPickableFacts } from "../accusationFacts";
import type { ScenarioCard } from "../../services/scenarioCards";

/** A card with just the fields this helper reads, plus a status. */
function card(
  graphNodeId: string,
  status: ScenarioCard["status"],
  code: string | null,
  text: string,
): ScenarioCard {
  return {
    code,
    graph_node_id: graphNodeId,
    quote: {
      text,
      context_before: "before",
      context_after: "after",
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
    speaker: { name: "CFS", attribution: "extracted" },
    statement_kind: null,
    stance: null,
    bears_on: [],
    grounding: null,
    confidence: { band: "unscored", label: "Not scored by a scan" },
    status,
    status_label: "whatever",
    defer_required: false,
    defer_required_reason: null,
    defer_reason: null,
    human_links: [],
    human_link_summary: null,
  } as ScenarioCard;
}

describe("includedPickableFacts", () => {
  it("offers only the facts a human ruled in", () => {
    const facts = includedPickableFacts([
      card("ev-in", "included", "C-1", "She refused to cooperate."),
      card("ev-undecided", "undecided", "C-2", "Nobody has ruled on this."),
      card("ev-dropped", "dropped", "C-3", "A human said no to this."),
    ]);

    expect(facts.map((f) => f.graphNodeId)).toEqual(["ev-in"]);
  });

  it("carries the handle and the statement's own words", () => {
    // The words are what a human recognises the statement by, and the handle is
    // what they say out loud. Both are needed for the row to be choosable.
    const [fact] = includedPickableFacts([
      card("ev-in", "included", "C-14", "She refused to cooperate."),
    ]);

    expect(fact).toEqual({
      graphNodeId: "ev-in",
      code: "C-14",
      text: "She refused to cooperate.",
    });
  });

  it("keeps a candidate nothing has numbered, with a null handle", () => {
    // A fact gather has not numbered yet is still a fact a human ruled in. Its
    // words identify it; dropping it would hide part of the scenario from the
    // picker with nothing saying so.
    const [fact] = includedPickableFacts([
      card("ev-in", "included", null, "An unnumbered but included statement."),
    ]);

    expect(fact.code).toBeNull();
    expect(fact.graphNodeId).toBe("ev-in");
  });

  it("preserves the payload's order so the picker matches the list above it", () => {
    const facts = includedPickableFacts([
      card("ev-b", "included", "C-9", "second"),
      card("ev-a", "included", "C-1", "first"),
    ]);

    expect(facts.map((f) => f.graphNodeId)).toEqual(["ev-b", "ev-a"]);
  });

  it("returns an empty list for a scenario with nothing ruled in", () => {
    // A real state, not an error: the section shows the stored "nothing marked
    // yet, N waiting" notice rather than an empty control with no explanation.
    expect(includedPickableFacts([])).toEqual([]);
    expect(includedPickableFacts([card("ev-x", "undecided", "C-1", "x")])).toEqual([]);
  });
});
