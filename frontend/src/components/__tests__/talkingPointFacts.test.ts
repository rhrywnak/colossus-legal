/**
 * Which facts sit under which talking point (v2.1, change F).
 *
 * Pure helpers only — no DOM, no RTL (CLAUDE.md Rule 30). Everything asserted
 * here is something that ONLY shows up on screen: which rows appear under point
 * 2, what order they arrive in, and what a row says when the card has no title.
 */
import { describe, expect, it } from "vitest";

import { factsForPoint, pointFactLine } from "../talkingPointFacts";
import { includedRows } from "../factsTable";
import { FACT_CARD_WORDING_FIXTURE as wording } from "../../testFixtures/factCardWording";
import type { FactCardBlock, ScenarioCard } from "../../services/scenarioCards";

const noDrafts = {
  title: false,
  backs: false,
  supports: false,
  watch_out: false,
  answer: false,
};

function block(over: Partial<FactCardBlock> = {}): FactCardBlock {
  return {
    title: "They admitted I was the only heir asked to pay",
    backs: null,
    backs_position: null,
    supports: [],
    supports_refs: [],
    watch_out: null,
    answer: null,
    drafts: noDrafts,
    count_tags: [],
    ...over,
  } as FactCardBlock;
}

type CardOver = {
  code?: string | null;
  id?: string;
  quote?: string;
  cite?: string;
  status?: ScenarioCard["status"];
  displayOrdinal?: number | null;
  block?: FactCardBlock | null;
};

function card(over: CardOver = {}): ScenarioCard {
  return {
    code: over.code === undefined ? "C-40" : over.code,
    graph_node_id: over.id ?? "ev-1",
    quote: {
      text: over.quote ?? "Admitted.",
      context_before: "",
      context_after: "",
      context_before_complete: true,
      context_after_complete: true,
      context_before_notice: null,
      context_after_notice: null,
      question: null,
    },
    pinpoint: {
      document_id: "doc-7",
      document_title: "CFS responses",
      label: over.cite ?? "CFS responses at 26",
      page: 26,
      viewer_href: "/documents/doc-7?page=26",
    },
    speaker: { name: "George Phillips", attribution: "extracted" },
    statement_kind: "sworn discovery answer",
    stance: null,
    bears_on: [],
    grounding: { state: "exact", label: "Grounded" },
    confidence: { band: "high", label: "high" },
    status: over.status ?? "included",
    status_label: "In the scenario",
    defer_required: false,
    defer_required_reason: null,
    defer_reason: null,
    human_links: [],
    human_link_summary: null,
    display_ordinal: over.displayOrdinal === undefined ? null : over.displayOrdinal,
    card: over.block === undefined ? block() : over.block,
  } as unknown as ScenarioCard;
}

// ─── Which facts appear under a point ────────────────────────────────────────

describe("factsForPoint", () => {
  it("returns only the facts whose backs_position matches", () => {
    const cards = [
      card({ id: "a", block: block({ backs_position: 1 }) }),
      card({ id: "b", block: block({ backs_position: 2 }) }),
      card({ id: "c", block: block({ backs_position: 1 }) }),
    ];
    expect(factsForPoint(cards, 1).map((r) => r.graphNodeId)).toEqual(["a", "c"]);
    expect(factsForPoint(cards, 2).map((r) => r.graphNodeId)).toEqual(["b"]);
  });

  it("shows a fact with NO backs_position under no point at all", () => {
    // The five hand-added facts on S-7 are exactly this shape. They are not
    // lost — they are in the Scenario facts list with a picker to aim them —
    // and a point must not claim evidence nobody filed under it.
    const cards = [card({ id: "a", block: block({ backs_position: null }) })];
    expect(factsForPoint(cards, 1)).toHaveLength(0);
    expect(factsForPoint(cards, 2)).toHaveLength(0);
    expect(includedRows(cards), "…but it IS in the facts list").toHaveLength(1);
  });

  it("shows a fact with no card block under no point either", () => {
    // A statement nobody has drafted a card for has nothing to point WITH. The
    // optional chain in `factsForPoint` is what makes this a filter rather than
    // a crash, and this is the case that would crash it.
    const cards = [card({ id: "a", block: null })];
    expect(factsForPoint(cards, 1)).toHaveLength(0);
  });

  it("silently drops a STALE pointer rather than inventing a point for it", () => {
    // `backs_position` is an address, not a foreign key: deleting point 3 leaves
    // every fact that pointed at it aiming at nothing. Nothing errors and
    // nothing dangles — the fact appears under no point, and the picker on its
    // card is how a human re-aims it.
    const cards = [card({ id: "a", block: block({ backs_position: 7 }) })];
    expect(factsForPoint(cards, 1)).toHaveLength(0);
    expect(factsForPoint(cards, 7), "…and it is not hidden from a point that DOES exist")
      .toHaveLength(1);
  });

  it("never shows a candidate or a set-aside fact", () => {
    // A point renders the scenario's EVIDENCE. An undecided card is the queue's
    // business and a dropped one is a judgment already made; either appearing
    // under Marie's argument would put a fact on screen the scenario does not
    // hold. The three tokens are the whole of `FactStatus` — a fourth added to
    // the column tomorrow fails to compile here, which is the point of naming
    // them rather than testing "not included".
    const cards = [
      card({ id: "in", status: "included", block: block({ backs_position: 1 }) }),
      card({ id: "undecided", status: "undecided", block: block({ backs_position: 1 }) }),
      card({ id: "dropped", status: "dropped", block: block({ backs_position: 1 }) }),
    ];
    expect(factsForPoint(cards, 1).map((r) => r.graphNodeId)).toEqual(["in"]);
  });

  it("uses the FACTS LIST's order, not the payload's", () => {
    // The sequence is the argument — which is why the list has a drag control
    // and a Reset order beside it. A second ordering here would mean the same
    // three facts read one way under the point and another in the list.
    const cards = [
      card({ id: "third", displayOrdinal: 30720, block: block({ backs_position: 1 }) }),
      card({ id: "first", displayOrdinal: 10240, block: block({ backs_position: 1 }) }),
      card({ id: "second", displayOrdinal: 20480, block: block({ backs_position: 1 }) }),
    ];
    expect(factsForPoint(cards, 1).map((r) => r.graphNodeId)).toEqual([
      "first",
      "second",
      "third",
    ]);
  });
});

// ─── What one collapsed row says ─────────────────────────────────────────────

describe("pointFactLine", () => {
  const lineFor = (over: CardOver) =>
    pointFactLine(includedRows([card(over)])[0], wording);

  it("reads code · title · cite", () => {
    const line = lineFor({
      code: "C-40",
      cite: "GEORGE PHILLIPS ADMISSIONS RESPONSE at 1",
    });
    expect(line).toEqual({
      code: "C-40",
      title: "They admitted I was the only heir asked to pay",
      cite: "GEORGE PHILLIPS ADMISSIONS RESPONSE at 1",
    });
  });

  it("falls back to the quote's first line when the card has no title", () => {
    // NOT the em dash `cardTitle` returns. On the card itself an empty title is
    // work somebody owes and the gap should show; here the row is the only thing
    // naming the fact, and "C-40 · — · CFS responses at 26" tells a reader
    // nothing about what is under it.
    const line = lineFor({
      quote: "The money was returned in April.\nThat is what the order said.",
      block: block({ title: null }),
    });
    expect(line.title).toBe("The money was returned in April.");
    expect(line.title).not.toBe(wording.empty_value);
  });

  it("falls back for a card with no block at all", () => {
    const line = lineFor({ quote: "Yes.", block: null });
    expect(line.title).toBe("Yes.");
  });

  it("keeps the record's words verbatim — the fallback is never groomed", () => {
    // §12: an anchor is sacred. The fallback trims surrounding whitespace and
    // takes the first LINE; it does not truncate, re-case or add an ellipsis.
    const quote = "  I DON'T RECALL saying that — not in those words.  ";
    expect(lineFor({ quote, block: block({ title: null }) }).title).toBe(
      "I DON'T RECALL saying that — not in those words.",
    );
  });

  it("reports no code for a card nothing has numbered", () => {
    // `code` is minted by gather. A card that arrived before its ordinal shows
    // no handle rather than an empty cell that reads as a loss.
    expect(lineFor({ code: null }).code).toBeNull();
  });

  it("yields an empty title for an all-whitespace quote, and says nothing else", () => {
    // The honest answer: the row is still there, still carries its code and its
    // cite, and the gap is visible rather than papered over with a placeholder
    // that would read as content.
    const line = lineFor({ quote: "   \n  ", block: block({ title: null }) });
    expect(line.title).toBe("");
    expect(line.cite).toBe("CFS responses at 26");
  });
});
