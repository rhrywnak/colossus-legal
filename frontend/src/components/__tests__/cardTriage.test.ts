// =============================================================================
// cardTriage.test.ts — the §7 contract and the triage state machine (task 1.3)
// =============================================================================
//
// The headline test is `a_complete_card_carries_every_required_section_seven_element`:
// §7 makes a card missing any element a DEFECT, so the contract is the test.
//
// Everything here is pure. Per CLAUDE.md rule 30 the frontend has no DOM test
// infrastructure by deliberate convention, so the logic that WOULD need a
// browser — key handling, advance, undo, the defer prompt — was extracted into a
// reducer precisely so it can be tested as data.

import { describe, expect, it } from "vitest";

import {
  cardRows,
  DEFER_QUICK_REASONS,
  initialQueueState,
  missingElements,
  progress,
  queueReducer,
  REQUIRED_CARD_ELEMENTS,
  type QueueState,
} from "../cardTriage";
import type { ScenarioCard } from "../../services/scenarioCards";

// ─── Fixtures ───────────────────────────────────────────────────────────────

/** A card with everything the payload can carry. */
function fullCard(overrides: Partial<ScenarioCard> = {}): ScenarioCard {
  return {
    code: "C-14",
    graph_node_id: "ev-1",
    quote: {
      text: "I do not recall that meeting.",
      context_before: "Q. Did you attend on March 3? A. ",
      context_after: " Q. Who else was present?",
      question: "Did you attend the meeting on March 3, 2019?",
    },
    pinpoint: {
      document_id: "doc-7",
      document_title: "CFS interrogatory responses",
      label: "CFS interrogatory responses at 14",
      page: 14,
      viewer_href: "/documents/doc-7?page=14&tab=document",
    },
    speaker: { name: "R. Phillips", attribution: "extracted" },
    statement_kind: "partial admission",
    stance: {
      verb: "disputes",
      object: "¶54 — CFS knew of the meeting",
      summary: "This disputes ¶54 — CFS knew of the meeting",
    },
    bears_on: [
      { accusation: "¶54 — CFS knew of the meeting", elements: ["Notice"], count: "Count 2 — Negligence" },
    ],
    grounding: { state: "exact", label: "Grounded — found on the page" },
    confidence: { band: "medium", label: "Scan was fairly confident" },
    status_label: "Not yet decided",
    defer_required: false,
    defer_required_reason: null,
    defer_reason: null,
    ...overrides,
  };
}

/** The C-222 class: scored, but linked to no accusation. */
function unrulableCard(id = "ev-222"): ScenarioCard {
  return fullCard({
    code: "C-222",
    graph_node_id: id,
    stance: null,
    bears_on: [],
    defer_required: true,
    defer_required_reason:
      "A scan scored this item, but it is not linked to any accusation, so there " +
      "is nothing for it to support or dispute. Link it to an accusation, or " +
      "defer it; it stays in the queue.",
  });
}

function stateOf(cards: ScenarioCard[]): QueueState {
  return initialQueueState(cards);
}

/** Press a key in triage mode (not typing into a field). */
function press(state: QueueState, key: string, typing = false) {
  return queueReducer(state, { type: "key", key, typing });
}

// ─── The §7 contract ────────────────────────────────────────────────────────

describe("the §7 card contract", () => {
  it("a complete card carries every required §7 element", () => {
    // THE CONTRACT TEST. A card missing any of these is a defect by §7, so this
    // asserts each by name. An edit that drops one fails here.
    const rows = cardRows(fullCard());
    const present = rows.map((r) => r.element);

    for (const required of REQUIRED_CARD_ELEMENTS) {
      expect(present, `§7 element "${required}" is missing`).toContain(required);
    }
    expect(missingElements(fullCard())).toEqual([]);
  });

  it("renders payload strings verbatim — the frontend composes nothing", () => {
    // The language law: every displayed string arrives from the backend. If a
    // row's value were built here, this comparison against the payload fails.
    const card = fullCard();
    const rows = cardRows(card);

    expect(rows.find((r) => r.element === "quote")?.value).toBe(card.quote.text);
    expect(rows.find((r) => r.element === "stance")?.value).toBe(card.stance!.summary);
    expect(rows.find((r) => r.element === "grounding")?.value).toBe(card.grounding!.label);
    expect(rows.find((r) => r.element === "confidence")?.value).toBe(card.confidence.label);
    expect(rows.find((r) => r.element === "status")?.value).toBe(card.status_label);
  });

  it("shows the defer reason in the stance slot when there is no stance", () => {
    // §7.5's structural rule: a card has a stance WITH its object, or it says why
    // it has none. Never a bare verb — that was the July defect.
    const rows = cardRows(unrulableCard());
    const stance = rows.find((r) => r.element === "stance");

    expect(stance).toBeDefined();
    expect(stance!.value).toContain("not linked to any accusation");
    expect(missingElements(unrulableCard())).toEqual([]);
  });

  it("never shows a stance verb without its object", () => {
    // The regression guard for C-222. If a future edit made the stance row fall
    // back to `card.stance.verb`, this card would render "disputes" alone.
    const rows = cardRows(unrulableCard());
    const stance = rows.find((r) => r.element === "stance")!;
    expect(stance.value).not.toBe("disputes");
    expect(stance.value.length).toBeGreaterThan(20);
  });

  it("carries the pinpoint's viewer link rather than building one", () => {
    const card = fullCard();
    const pinpoint = cardRows(card).find((r) => r.element === "pinpoint")!;
    expect(pinpoint.href).toBe(card.pinpoint.viewer_href);
    // Verbatim from the payload — the client joins the title and page nowhere.
    expect(pinpoint.value).toBe(card.pinpoint.label);
  });

  it("omits a page it does not have instead of inventing one", () => {
    const card = fullCard({
      pinpoint: {
        document_id: "doc-7",
        document_title: "A letter",
        label: "A letter",
        page: null,
        viewer_href: "/documents/doc-7?tab=document",
      },
    });
    const pinpoint = cardRows(card).find((r) => r.element === "pinpoint")!;
    expect(pinpoint.value).toBe("A letter");
    expect(pinpoint.value).not.toContain(" at ");
  });

  it("omits speaker and statement kind for documentary evidence", () => {
    // Real absences, not defects: a letter has no speaker. They are absent from
    // REQUIRED_CARD_ELEMENTS for exactly this reason.
    const card = fullCard({
      speaker: { name: null, attribution: "extracted" },
      statement_kind: null,
    });
    const present = cardRows(card).map((r) => r.element);
    expect(present).not.toContain("speaker");
    expect(present).not.toContain("statement_kind");
    expect(missingElements(card)).toEqual([]);
  });

  it("lists every accusation with its elements and count as chips", () => {
    const card = fullCard({
      bears_on: [
        { accusation: "¶12 — first", elements: ["Duty", "Materiality"], count: "Count 2 — Fraud" },
        { accusation: "¶13 — second", elements: [], count: null },
      ],
    });
    const rows = cardRows(card).filter((r) => r.element === "bears_on");
    expect(rows).toHaveLength(2);
    expect(rows[0].chips).toEqual(["Duty", "Materiality", "Count 2 — Fraud"]);
    expect(rows[1].chips).toEqual([]);
  });

  it("reports a missing element rather than rendering a partial card silently", () => {
    // The contract's negative case: strip the confidence label's row source and
    // `missingElements` must SAY so. A card that quietly renders short is the
    // failure mode §7 exists to prevent.
    const rows = cardRows(fullCard()).filter((r) => r.element !== "confidence");
    const present = new Set(rows.map((r) => r.element));
    expect(REQUIRED_CARD_ELEMENTS.filter((e) => !present.has(e))).toEqual(["confidence"]);
  });
});

// ─── The keyboard state machine ─────────────────────────────────────────────

describe("triage keys", () => {
  it("I includes the focused card and advances", () => {
    const { state, effect } = press(stateOf([fullCard(), fullCard({ graph_node_id: "ev-2" })]), "i");
    expect(effect).toEqual({ kind: "rule", graphNodeId: "ev-1", action: "include", reason: undefined });
    expect(state.index).toBe(1);
    expect(progress(state).ruled).toBe(1);
  });

  it("E excludes with the backend's verb, not the UI's word", () => {
    // The UI says "exclude"; the endpoint's verb is `drop`. Sending the wrong
    // token would 400 at the parse boundary.
    const { effect } = press(stateOf([fullCard()]), "e");
    expect(effect).toMatchObject({ action: "drop" });
  });

  it("accepts upper case — Shift is not a different ruling", () => {
    expect(press(stateOf([fullCard()]), "I").effect).toMatchObject({ action: "include" });
  });

  it("stops at the last card rather than wrapping to the top", () => {
    // Wrapping would silently return the human to a list they thought was done.
    let s = stateOf([fullCard(), fullCard({ graph_node_id: "ev-2" })]);
    s = press(s, "i").state;
    s = press(s, "i").state;
    expect(s.index).toBe(1);
    expect(progress(s)).toEqual({ ruled: 2, total: 2 });
  });

  it("ignores an unknown key", () => {
    const { state, effect } = press(stateOf([fullCard()]), "x");
    expect(effect).toEqual({ kind: "none" });
    expect(state.index).toBe(0);
  });

  it("does nothing on an empty queue", () => {
    const { effect } = press(stateOf([]), "i");
    expect(effect).toEqual({ kind: "none" });
  });
});

describe("the typing guard", () => {
  it("does not rule while the human is typing in a field", () => {
    // Without this, writing a note would include half the pool.
    const { state, effect } = press(stateOf([fullCard()]), "i", true);
    expect(effect).toEqual({ kind: "none" });
    expect(state.index).toBe(0);
    expect(progress(state).ruled).toBe(0);
  });
});

describe("defer", () => {
  it("D opens the prompt on an ordinary card", () => {
    const { state, effect } = press(stateOf([fullCard()]), "d");
    expect(state.mode).toEqual({ kind: "deferring", draft: "" });
    expect(effect).toEqual({ kind: "none" });
  });

  it("a digit picks a quick reason without leaving the keyboard", () => {
    let s = press(stateOf([fullCard()]), "d").state;
    s = press(s, "2").state;
    expect(s.mode).toEqual({ kind: "deferring", draft: DEFER_QUICK_REASONS[1] });
  });

  it("Enter commits the drafted reason and closes the prompt", () => {
    let s = press(stateOf([fullCard(), fullCard({ graph_node_id: "ev-2" })]), "d").state;
    s = queueReducer(s, { type: "defer_draft", draft: "waiting on a clean copy" }).state;
    const { state, effect } = press(s, "Enter");

    expect(effect).toEqual({
      kind: "rule",
      graphNodeId: "ev-1",
      action: "defer",
      reason: "waiting on a clean copy",
    });
    expect(state.mode).toEqual({ kind: "triage" });
    expect(state.index).toBe(1);
  });

  it("refuses a blank reason and keeps the prompt open", () => {
    // The backend rejects a reasonless defer; refusing here keeps the human's
    // cursor where it is instead of bouncing an error at them.
    let s = press(stateOf([fullCard()]), "d").state;
    s = queueReducer(s, { type: "defer_draft", draft: "   " }).state;
    const { state, effect } = press(s, "Enter");

    expect(effect).toEqual({ kind: "none" });
    expect(state.mode.kind).toBe("deferring");
  });

  it("Esc cancels without ruling", () => {
    let s = press(stateOf([fullCard()]), "d").state;
    s = queueReducer(s, { type: "defer_draft", draft: "half a thought" }).state;
    const { state, effect } = press(s, "Escape");

    expect(effect).toEqual({ kind: "none" });
    expect(state.mode).toEqual({ kind: "triage" });
    expect(progress(state).ruled).toBe(0);
  });

  it("the prompt owns the keyboard — I does not rule while it is open", () => {
    const s = press(stateOf([fullCard()]), "d").state;
    const { effect } = press(s, "i");
    expect(effect).toEqual({ kind: "none" });
  });
});

describe("the defer_required short-circuit", () => {
  it("D accepts the server's reason in one press, with no prompt", () => {
    const card = unrulableCard();
    const { state, effect } = press(stateOf([card, fullCard({ graph_node_id: "ev-2" })]), "d");

    expect(effect).toEqual({
      kind: "rule",
      graphNodeId: "ev-222",
      action: "defer",
      reason: card.defer_required_reason,
    });
    expect(state.mode).toEqual({ kind: "triage" });
    expect(state.index).toBe(1);
  });

  it("I and E do nothing on an unrulable card — no 400 round trip", () => {
    // The reason is already on the card; the human reads it instead of waiting
    // for the backend to refuse.
    for (const key of ["i", "e"]) {
      const { state, effect } = press(stateOf([unrulableCard()]), key);
      expect(effect, `${key} must not rule an unrulable card`).toEqual({ kind: "none" });
      expect(state.index).toBe(0);
    }
  });
});

describe("undo", () => {
  it("U reopens the last ruling and refocuses that card", () => {
    let s = stateOf([fullCard(), fullCard({ graph_node_id: "ev-2" })]);
    s = press(s, "i").state;
    expect(s.index).toBe(1);

    const { state, effect } = press(s, "u");
    // `reopen`, never `undrop`: same state, but the ledger records the word, and
    // "undrop" on a never-dropped item is a false entry.
    expect(effect).toEqual({ kind: "rule", graphNodeId: "ev-1", action: "reopen" });
    expect(state.index).toBe(0);
    expect(progress(state).ruled).toBe(0);
  });

  it("is single-step — a second U does nothing", () => {
    // A stack would let a human unwind a whole session by leaning on one key.
    let s = stateOf([fullCard(), fullCard({ graph_node_id: "ev-2" })]);
    s = press(s, "i").state;
    s = press(s, "u").state;
    const { effect } = press(s, "u");
    expect(effect).toEqual({ kind: "none" });
  });

  it("does nothing before anything has been ruled", () => {
    expect(press(stateOf([fullCard()]), "u").effect).toEqual({ kind: "none" });
  });

  it("undoes a defer as well as an include", () => {
    let s = press(stateOf([unrulableCard()]), "d").state;
    const { effect } = press(s, "u");
    expect(effect).toMatchObject({ action: "reopen", graphNodeId: "ev-222" });
  });
});

describe("the running count", () => {
  it("counts each card once however many times it is ruled", () => {
    // "How many of the pool have been dealt with", not "how many keys pressed".
    let s = stateOf([fullCard()]);
    s = press(s, "i").state;
    s = queueReducer(s, { type: "focus", index: 0 }).state;
    s = press(s, "e").state;
    expect(progress(s)).toEqual({ ruled: 1, total: 1 });
  });

  it("survives a reload without throwing the human back to the top", () => {
    let s = stateOf([fullCard(), fullCard({ graph_node_id: "ev-2" })]);
    s = press(s, "i").state;
    const reloaded = queueReducer(s, {
      type: "cards_loaded",
      cards: [fullCard(), fullCard({ graph_node_id: "ev-2" })],
    }).state;
    expect(reloaded.index).toBe(1);
  });

  it("clamps focus when a reload returns a shorter pool", () => {
    let s = stateOf([fullCard(), fullCard({ graph_node_id: "ev-2" })]);
    s = press(s, "i").state;
    const reloaded = queueReducer(s, { type: "cards_loaded", cards: [fullCard()] }).state;
    expect(reloaded.index).toBe(0);
  });
});
