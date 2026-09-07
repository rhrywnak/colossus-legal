/**
 * Pure-helper tests for the witness card (FACT_CARD_v2 §2).
 *
 * No DOM / RTL — pure functions only (CLAUDE.md §30). Everything asserted here is
 * something that ONLY shows up on screen: which rows carry an em dash, which
 * carry a draft mark, which cards are open, and what an Include is prefilled
 * with.
 */
import { describe, expect, it } from "vitest";
import {
  anyDraft,
  cardRows,
  cardTitle,
  deckLine,
  includePrefill,
  opensInFull,
  rowText,
} from "../factCard";
import { FACT_CARD_WORDING_FIXTURE as wording } from "../../testFixtures/factCardWording";
import type {
  FactCardBlock,
  FactCardDrafts,
  ScenarioCard,
} from "../../services/scenarioCards";

const noDrafts: FactCardDrafts = {
  title: false,
  backs: false,
  supports: false,
  watch_out: false,
  answer: false,
};

const block = (overrides: Partial<FactCardBlock> = {}): FactCardBlock => ({
  title: "The court ordered the $50,000 back",
  backs: "Point 1 — A judge ordered that money put back.",
  backs_position: 1,
  supports: ["Supports A-21 — CFS could have returned it."],
  supports_refs: [
    { allegation_id: "doc-x:allegation:45984d77", stance: "supports", code: "A-21" },
  ],
  watch_out: "They will say the judge approved it.",
  answer: "The money was Dad's.",
  drafts: noDrafts,
  count_tags: ["Count 1 — Breach of Fiduciary Duty"],
  ...overrides,
});

const card = (overrides: Partial<ScenarioCard> = {}): ScenarioCard =>
  ({ graph_node_id: "n1", card: block(), ...overrides }) as ScenarioCard;

// ─── The one row that renders (v2.1, ruling R37) ─────────────────────────────

describe("cardRows", () => {
  it("returns SUPPORTS and nothing else", () => {
    // The suite this replaces asserted four rows in §2's order. Three of them —
    // Backs, Watch out, Answer — were the machine's PROSE about the evidence,
    // and v2.1 took them off the page: this is Roman's working file, not a
    // witness's deck. The fields are untouched on the payload and in the tables;
    // see the module header.
    const fields = cardRows(block(), wording).map((r) => r.field);
    expect(fields).toEqual(["supports"]);
  });

  it("does not smuggle the three removed fields back as lines", () => {
    // The regression that would actually be made: re-adding a row by widening
    // this function rather than by re-arguing the ruling. Asserting the FIELD
    // list alone would not catch a fourth row appended tomorrow with a label
    // read from somewhere else.
    const printed = cardRows(block(), wording)
      .flatMap((r) => [r.label, ...r.lines])
      .join(" | ");
    expect(printed).not.toContain("Point 1");
    expect(printed).not.toContain("They will say the judge approved it.");
    expect(printed).not.toContain("The money was Dad's.");
    expect(printed).not.toContain(wording.watch_out_label);
    expect(printed).not.toContain(wording.answer_label);
  });

  it("STILL returns the row when Supports is empty", () => {
    // The half of §2's reasoning that SURVIVES: an empty row is work somebody
    // owes, and hiding it hides the work rather than the gap. A card that names
    // no accusation renders the row with the stored em dash and stays.
    const rows = cardRows(
      block({ backs: null, supports: [], watch_out: null, answer: null }),
      wording,
    );
    expect(rows).toHaveLength(1);
    expect(rows[0].lines).toEqual([]);
    expect(rowText(rows[0], wording)).toEqual(["—"]);
  });

  it("labels the row from the store", () => {
    expect(cardRows(block(), wording).map((r) => r.label)).toEqual(["Supports"]);
  });

  it("carries both Supports lines when a card names two accusations", () => {
    const rows = cardRows(
      block({ supports: ["Supports A-21 — one", "Disputes A-44 — two"] }),
      wording,
    );
    const supports = rows.find((r) => r.field === "supports");
    expect(supports?.lines).toHaveLength(2);
  });

  it("carries the row's own draft mark", () => {
    // Per-field authorship still rides the block — `anyDraft` below reads all
    // five — and the one rendered row reports its own, not the card's.
    expect(
      cardRows(block({ drafts: { ...noDrafts, supports: true } }), wording)[0].draft,
    ).toBe(true);
    expect(
      cardRows(block({ drafts: { ...noDrafts, answer: true } }), wording)[0].draft,
    ).toBe(false);
  });
});

describe("rowText", () => {
  it("prints the stored em dash for an empty row", () => {
    const rows = cardRows(block({ supports: [] }), wording);
    expect(rowText(rows[0], wording)).toEqual(["—"]);
  });

  it("prints the lines when there are any", () => {
    const rows = cardRows(block(), wording);
    expect(rowText(rows[0], wording)).toEqual([
      "Supports A-21 — CFS could have returned it.",
    ]);
  });
});

// ─── The title ───────────────────────────────────────────────────────────────

describe("cardTitle", () => {
  it("returns the title and its draft mark", () => {
    const result = cardTitle(
      card({ card: block({ drafts: { ...noDrafts, title: true } }) }),
      wording,
    );
    expect(result.text).toBe("The court ordered the $50,000 back");
    expect(result.draft).toBe(true);
  });

  it("prints the em dash for a card with no title", () => {
    expect(cardTitle(card({ card: block({ title: null }) }), wording).text).toBe("—");
  });

  it("prints the em dash — and NO draft mark — for a card nobody has drafted", () => {
    // A statement nobody has written about is not a draft. Marking it would
    // claim the machine had tried and failed.
    const result = cardTitle(card({ card: undefined }), wording);
    expect(result.text).toBe("—");
    expect(result.draft).toBe(false);
  });
});

// ─── The deck line ───────────────────────────────────────────────────────────

describe("deckLine", () => {
  it("fills all three numbers from the served template", () => {
    expect(deckLine(10, 3, wording)).toBe("3 of 10 shown · 7 more collapsed");
  });

  it("says nothing when the whole deck is open", () => {
    // "10 of 10 shown · 0 more collapsed" is noise on a deck that is entirely
    // open, and a line reporting nothing is a line a reader learns to skip.
    expect(deckLine(10, 10, wording)).toBeNull();
    expect(deckLine(3, 10, wording)).toBeNull();
  });

  it("says nothing for an empty deck", () => {
    expect(deckLine(0, 10, wording)).toBeNull();
  });

  it("clamps a nonsense visible count rather than reporting a negative", () => {
    expect(deckLine(10, -4, wording)).toBe("0 of 10 shown · 10 more collapsed");
  });
});

// ─── The fold ────────────────────────────────────────────────────────────────

describe("opensInFull", () => {
  it("opens the first N and collapses the rest", () => {
    expect(opensInFull(0, 10)).toBe(true);
    expect(opensInFull(9, 10)).toBe(true);
    expect(opensInFull(10, 10)).toBe(false);
  });

  it("opens nothing for a nonsense count rather than inverting the fold", () => {
    // A negative served count must not open everything, and must not throw. The
    // value is served, and a store edited around the API can hold anything.
    expect(opensInFull(0, -1)).toBe(false);
    expect(opensInFull(0, 0)).toBe(false);
  });
});

// ─── The Include prefill ─────────────────────────────────────────────────────

describe("includePrefill", () => {
  it("takes the accusation and stance from the card's first support", () => {
    // §2: "prefilled from the card's supports[0]". The machine's own first answer
    // to "what does this bear on", so in the ordinary case nobody types anything.
    expect(includePrefill(card())).toEqual({
      allegation_id: "doc-x:allegation:45984d77",
      stance: "supports",
    });
  });

  it("returns nothing when the card names no accusation", () => {
    // The human then picks one, and the Include is refused until they do — which
    // is the backend's rule, not a guess made here.
    expect(includePrefill(card({ card: block({ supports_refs: [] }) }))).toBeNull();
  });

  it("returns nothing for a card nobody has drafted", () => {
    expect(includePrefill(card({ card: undefined }))).toBeNull();
  });
});

describe("anyDraft", () => {
  it("is false when nothing is a draft and true for each field alone", () => {
    expect(anyDraft(noDrafts)).toBe(false);
    for (const field of ["title", "backs", "supports", "watch_out", "answer"] as const) {
      expect(anyDraft({ ...noDrafts, [field]: true })).toBe(true);
    }
  });
});
