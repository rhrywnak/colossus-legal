/**
 * Pure-helper tests for the Proof Matrix row (PROOF_MATRIX_v2 §3).
 *
 * No DOM / RTL — pure functions only (CLAUDE.md §30). Everything asserted here
 * is something that ONLY shows up on screen: which items are behind a control,
 * which are hidden, and whether a line claims a human has read it.
 */
import { describe, expect, it, vi } from "vitest";
import {
  authorMark,
  confidenceLabel,
  hiddenToggleLabel,
  moreLabel,
  quoteText,
  reasonLine,
  splitEvidence,
} from "../matrixRow";
import { MATRIX_WORDING_FIXTURE as wording } from "../../testFixtures/matrixWording";
import type { AllegationEvidence } from "../../services/elementDetailService";

const item = (
  id: string,
  overrides: Partial<AllegationEvidence> = {},
): AllegationEvidence => ({
  id,
  verbatim_quote: `the words of ${id}`,
  page_number: 22,
  paragraph: null,
  page_note: null,
  source_document_id: "doc-phillips",
  source_document_title: "Phillips Discovery Response",
  statement_type: null,
  evidence_strength: null,
  speaker: null,
  question: null,
  answer: null,
  rank: null,
  role: null,
  confidence: null,
  rank_reason: null,
  why: null,
  conflict: false,
  duplicate_of_card_id: null,
  document_date: null,
  ruling: null,
  ruled_by: null,
  hidden_reason: null,
  rfa_line: null,
  occurrences: 1,
  ...overrides,
});

const ids = (items: AllegationEvidence[]) => items.map((i) => i.id);

describe("splitEvidence", () => {
  it("shows the first `limit` visible items and puts the rest behind more", () => {
    const items = ["a", "b", "c", "d", "e", "f", "g"].map((id) => item(id));
    const split = splitEvidence(items, 5);
    expect(ids(split.shown)).toEqual(["a", "b", "c", "d", "e"]);
    expect(ids(split.behindMore)).toEqual(["f", "g"]);
    expect(split.hidden).toHaveLength(0);
  });

  it("counts only VISIBLE items towards the limit", () => {
    // The regression this catches: taking five and then filtering would show two
    // rows where the reader expects five, silently.
    const items = [
      item("x", { hidden_reason: "removed" }),
      item("y", { hidden_reason: "does_not_belong" }),
      ...["a", "b", "c", "d", "e", "f"].map((id) => item(id)),
    ];
    const split = splitEvidence(items, 5);
    expect(ids(split.shown)).toEqual(["a", "b", "c", "d", "e"]);
    expect(ids(split.behindMore)).toEqual(["f"]);
    expect(ids(split.hidden)).toEqual(["x", "y"]);
  });

  it("loses nothing: every item lands in exactly one of the three groups", () => {
    const items = [
      item("a"),
      item("b", { hidden_reason: "removed" }),
      item("c"),
      item("d"),
    ];
    const split = splitEvidence(items, 1);
    const total =
      split.shown.length + split.behindMore.length + split.hidden.length;
    expect(total).toBe(items.length);
  });

  it("does not reorder what the backend ordered", () => {
    // The backend applied §3's four sort keys. A second sort here would be a
    // ranking free to disagree with the one the Word export prints from.
    const items = [item("z"), item("a"), item("m")];
    expect(ids(splitEvidence(items, 5).shown)).toEqual(["z", "a", "m"]);
  });

  it("treats a nonsense limit as zero rather than slicing from the end", () => {
    // A negative index counts from the END in `slice`, which would show the LAST
    // few items — a silently wrong short list, on a page whose order is a claim.
    const items = ["a", "b", "c"].map((id) => item(id));
    for (const limit of [-2, Number.NaN]) {
      const split = splitEvidence(items, limit);
      expect(split.shown).toHaveLength(0);
      expect(ids(split.behindMore)).toEqual(["a", "b", "c"]);
    }
  });
});

describe("moreLabel", () => {
  it("fills the served template with the number behind it", () => {
    expect(moreLabel([item("f"), item("g")], wording)).toBe("2 more");
  });

  it("returns nothing when there is nothing behind it", () => {
    // A control that opens nothing is one a reader clicks once and distrusts.
    expect(moreLabel([], wording)).toBeNull();
  });
});

describe("hiddenToggleLabel", () => {
  it("says how many are hidden when they are hidden", () => {
    expect(hiddenToggleLabel([item("x")], false, wording)).toBe("Show hidden (1)");
  });

  it("becomes the put-them-back control once they are showing", () => {
    expect(hiddenToggleLabel([item("x")], true, wording)).toBe("Hide those again");
  });

  it("returns nothing when nothing is hidden", () => {
    expect(hiddenToggleLabel([], false, wording)).toBeNull();
  });
});

describe("authorMark", () => {
  it("says `machine` on an item nobody has ruled on", () => {
    // THE load-bearing case. This word is what keeps an unread machine ranking
    // from being mistaken for reviewed work.
    const mark = authorMark(item("a"), wording);
    expect(mark.label).toBe("machine");
    expect(mark.confirmed).toBe(false);
  });

  it("names the human who kept it", () => {
    const mark = authorMark(
      item("a", { ruling: "keep", ruled_by: "roman" }),
      wording,
    );
    expect(mark.label).toBe("kept · roman");
    expect(mark.confirmed).toBe(true);
  });

  it("says `machine` on a REMOVED item — a removal is not a confirmation", () => {
    const mark = authorMark(
      item("a", { ruling: "remove", ruled_by: "roman", hidden_reason: "removed" }),
      wording,
    );
    expect(mark.confirmed).toBe(false);
  });

  it("does not claim a confirmation with nobody attached to it", () => {
    // Unreachable through the API — the column is NOT NULL. If it ever arrived,
    // an anonymous "kept · " is worse than saying the machine ranked it.
    const mark = authorMark(item("a", { ruling: "keep", ruled_by: null }), wording);
    expect(mark.label).toBe("machine");
    expect(mark.confirmed).toBe(false);
  });
});

describe("confidenceLabel", () => {
  it("maps each known token to its stored word", () => {
    expect(confidenceLabel("high", wording)).toBe("high");
    expect(confidenceLabel("medium", wording)).toBe("medium");
  });

  it("has a word for `low` before any pass emits one", () => {
    // No edge carries `low` today. Without this case the FIRST low-confidence
    // item would render with no mark — which is exactly how an unrated item
    // reads — and the reader would be told nothing rather than told "low".
    expect(confidenceLabel("low", wording)).toBe("low");
  });

  it("says `unrated` for an item the linking pass never reached", () => {
    // 286 edges predate the pass. No mark at all would make them read as items
    // the pass had considered and rated in the middle.
    expect(confidenceLabel(null, wording)).toBe("unrated");
  });

  it("prints nothing for an unknown token, and says so in the console", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    expect(confidenceLabel("certain", wording)).toBeNull();
    expect(warn).toHaveBeenCalledTimes(1);
    expect(warn.mock.calls[0][0]).toContain("certain");
    warn.mockRestore();
  });
});

describe("reasonLine", () => {
  it("prefers the linking pass's rank_reason", () => {
    expect(
      reasonLine(item("a", { rank_reason: "His own sworn answer.", why: "older" })),
    ).toBe("His own sworn answer.");
  });

  it("falls back to the older pass's `why`", () => {
    // 36 edges carry a `why` and no rank at all. A reason nobody reads is worse
    // than an untidy rule.
    expect(reasonLine(item("a", { why: "Supports the allegation." }))).toBe(
      "Supports the allegation.",
    );
  });

  it("returns nothing when there is neither, rather than an empty line", () => {
    expect(reasonLine(item("a"))).toBeNull();
    expect(reasonLine(item("a", { rank_reason: "   " }))).toBeNull();
  });
});

describe("quoteText", () => {
  it("prints the backend-composed RFA line when there is one", () => {
    // A Q&A card's verbatim_quote is the ANSWER alone — "Admitted" — and a column
    // of those under an accusation says nothing about what was admitted.
    expect(
      quoteText(
        item("a", {
          verbatim_quote: "Admitted",
          rfa_line: "RFA 8 — Admit that you were engaged. — Admitted",
        }),
      ),
    ).toBe("RFA 8 — Admit that you were engaged. — Admitted");
  });

  it("prints the verbatim quote for an ordinary card", () => {
    expect(quoteText(item("a"))).toBe("the words of a");
  });

  it("returns nothing for an item with no words at all", () => {
    expect(quoteText(item("a", { verbatim_quote: null }))).toBeNull();
    expect(quoteText(item("a", { verbatim_quote: "  " }))).toBeNull();
  });
});
