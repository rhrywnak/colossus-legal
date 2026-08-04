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
  DEFER_QUICK_REASONS,
  initialQueueState,
  progress,
  queueReducer,
  type QueueState,
} from "../cardTriage";
// The §7 descriptor moved to its own module in 1.7E (Rule 17 — see that file's
// header). Same functions, same assertions; only the import line moved.
import { cardRows, missingElements, REQUIRED_CARD_ELEMENTS } from "../cardRows";
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
      // Task 1.7C (D6) added these four to the payload. FIXTURE ONLY — not one
      // assertion in this file changed: all 31 §7 reducer tests are byte-identical
      // and still green. Both flanks sentence-complete, so neither notice is set.
      context_before_complete: true,
      context_after_complete: true,
      context_before_notice: null,
      context_after_notice: null,
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
    status: "undecided",
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
    // Task 1.7E item 6 rewrote this sentence server-side: it no longer tells the
    // human to link the item, because nothing links an item until task 2.10. The
    // fixture follows the payload — no assertion in this file changed for it.
    defer_required_reason:
      "A scan scored this item, but it is not linked to any accusation yet, so " +
      "there is nothing for it to support or dispute. It can only be deferred for " +
      "now — it stays in the queue and returns when linking arrives.",
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
    //
    // RE-ANCHORED in 1.7E: the no-write half is untouched, and the refusal now
    // has to SAY so as well (see the notice test below). Nothing was removed.
    for (const key of ["i", "e"]) {
      const card = unrulableCard();
      const { state, effect } = press(stateOf([card]), key);
      expect(effect, `${key} must not rule an unrulable card`).toEqual({ kind: "none" });
      expect(state.index).toBe(0);
      expect(state.notice, `${key} must explain the refusal`).toBe(card.defer_required_reason);
    }
  });

  it("carries the BACKEND's sentence rather than one composed here", () => {
    // The second unrulable class — an item with no verbatim quote — has a
    // completely different reason. A message hardcoded in the frontend would be
    // wrong for it, and wrong in a way nobody would notice for months.
    const noQuote = unrulableCard("ev-noquote");
    noQuote.defer_required_reason =
      "This item has no verbatim quote, so it cannot be cited or matched back to " +
      "the record after re-processing. Defer it; it stays in the queue.";
    expect(press(stateOf([noQuote]), "i").state.notice).toBe(noQuote.defer_required_reason);
  });

  it("clears the notice as soon as the human does something else", () => {
    // A message about the LAST card, still on screen beside a different one, is a
    // lie the reader has no way to date. Every event that moves the human on, or
    // changes what is under them, has to clear it — so each is asserted, not just
    // the one that happens to be implemented first.
    function refused(): QueueState {
      const s = press(stateOf([unrulableCard(), fullCard({ graph_node_id: "ev-2" })]), "i").state;
      expect(s.notice, "the fixture must actually produce a notice").not.toBeNull();
      return s;
    }

    // Navigating away from the card the message is about.
    expect(press(refused(), "j").state.notice).toBeNull();

    // Clicking a different card.
    expect(
      queueReducer(refused(), { type: "select", graphNodeId: "ev-2" }).state.notice,
    ).toBeNull();

    // Focusing another card by position.
    expect(queueReducer(refused(), { type: "focus", index: 1 }).state.notice).toBeNull();

    // Changing the filter under it — the message may now sit beside a card the
    // human can no longer see.
    expect(
      queueReducer(refused(), { type: "visible", ids: ["ev-2"] }).state.notice,
    ).toBeNull();

    // A RELOAD. This is the one that matters most: a refused ruling re-reads the
    // pool, and the stale refusal must not outlive the screen it described.
    expect(
      queueReducer(refused(), { type: "cards_loaded", cards: [fullCard()] }).state.notice,
    ).toBeNull();
  });
});

// ─── The list: selection, navigation, and the filtered order (task 1.7E-a) ──

describe("navigation", () => {
  const three = () => [
    fullCard({ graph_node_id: "ev-1" }),
    fullCard({ graph_node_id: "ev-2" }),
    fullCard({ graph_node_id: "ev-3" }),
  ];

  it("moves the selection and WRITES NOTHING", () => {
    // THE TEST THIS FEATURE STANDS ON. Browsing a 148-card list is free; a ruling
    // happens only when I, E or D is pressed. An effect here would mean arrowing
    // through the pool silently ruled it.
    for (const key of ["ArrowDown", "j", "ArrowUp", "k"]) {
      const { effect } = press(stateOf(three()), key);
      expect(effect, `${key} must not rule anything`).toEqual({ kind: "none" });
    }
  });

  it("↓ and j move forward, ↑ and k move back", () => {
    let s = press(stateOf(three()), "ArrowDown").state;
    expect(s.index).toBe(1);
    s = press(s, "j").state;
    expect(s.index).toBe(2);
    s = press(s, "ArrowUp").state;
    expect(s.index).toBe(1);
    s = press(s, "k").state;
    expect(s.index).toBe(0);
  });

  it("stops at both ends rather than wrapping", () => {
    // Same reason auto-advance does not wrap: the ends of the list are how a human
    // knows where they are in it.
    expect(press(stateOf(three()), "ArrowUp").state.index).toBe(0);
    let s = stateOf(three());
    for (let i = 0; i < 5; i += 1) s = press(s, "j").state;
    expect(s.index).toBe(2);
  });

  it("moves the selection to already-RULED cards too", () => {
    // Auto-advance skips what is decided; deliberate navigation does not — that is
    // how a human reaches a card they want to take back with U.
    const cards = [fullCard({ graph_node_id: "ev-1" }), fullCard({ graph_node_id: "ev-2", status: "included" })];
    expect(press(stateOf(cards), "j").state.index).toBe(1);
  });

  it("does nothing on an empty list", () => {
    expect(press(stateOf([]), "j")).toEqual({ state: stateOf([]), effect: { kind: "none" } });
  });

  it("does not move while the human is typing", () => {
    // The typing guard covers navigation as well: the arrow keys belong to the
    // text cursor while a defer reason is being written.
    expect(press(stateOf(three()), "ArrowDown", true).state.index).toBe(0);
  });
});

describe("selection", () => {
  it("a click selects by identity and rules nothing", () => {
    const cards = [fullCard({ graph_node_id: "ev-1" }), fullCard({ graph_node_id: "ev-2" })];
    const { state, effect } = queueReducer(stateOf(cards), {
      type: "select",
      graphNodeId: "ev-2",
    });
    expect(state.index).toBe(1);
    expect(effect).toEqual({ kind: "none" });
  });

  it("ignores a click on a card the pool no longer holds", () => {
    // Moving the selection somewhere the human did not point is worse than
    // ignoring one click that landed as a reload arrived.
    const s = stateOf([fullCard({ graph_node_id: "ev-1" })]);
    expect(queueReducer(s, { type: "select", graphNodeId: "gone" }).state.index).toBe(0);
  });
});

describe("the filtered order", () => {
  const pool = () => [
    fullCard({ graph_node_id: "ev-1" }),
    fullCard({ graph_node_id: "ev-2" }),
    fullCard({ graph_node_id: "ev-3" }),
  ];

  function visible(state: QueueState, ids: string[]): QueueState {
    return queueReducer(state, { type: "visible", ids }).state;
  }

  it("navigation walks the VISIBLE cards, skipping what the filter hides", () => {
    // Without this the keyboard would select a card the filter is hiding — the
    // same class of defect as ruling a card inside a collapsed region (R7).
    let s = visible(stateOf(pool()), ["ev-1", "ev-3"]);
    s = press(s, "j").state;
    expect(s.cards[s.index].graph_node_id).toBe("ev-3");
  });

  it("a filter that hides the selected card selects the first visible one", () => {
    let s = queueReducer(stateOf(pool()), { type: "select", graphNodeId: "ev-2" }).state;
    s = visible(s, ["ev-3"]);
    expect(s.cards[s.index].graph_node_id).toBe("ev-3");
  });

  it("leaves the selection alone when it survives the filter", () => {
    let s = queueReducer(stateOf(pool()), { type: "select", graphNodeId: "ev-2" }).state;
    s = visible(s, ["ev-1", "ev-2"]);
    expect(s.cards[s.index].graph_node_id).toBe("ev-2");
  });

  it("a ruling advances within the filtered set, not the whole pool", () => {
    let s = visible(stateOf(pool()), ["ev-1", "ev-3"]);
    s = press(s, "i").state;
    expect(s.cards[s.index].graph_node_id).toBe("ev-3");
  });

  it("changing the filter writes nothing", () => {
    expect(queueReducer(stateOf(pool()), { type: "visible", ids: ["ev-2"] }).effect).toEqual({
      kind: "none",
    });
  });
});

describe("auto-advance", () => {
  it("skips cards that were already ruled before this session", () => {
    // RE-ANCHORED in 1.7E. Until the list existed, every card in the queue was
    // unruled, so "the next card" and "the next card needing a decision" were the
    // same thing. They are not any more, and landing the human on a decided card
    // after every ruling is the difference between minutes and an afternoon.
    const cards = [
      fullCard({ graph_node_id: "ev-1" }),
      fullCard({ graph_node_id: "ev-2", status: "included", status_label: "Included" }),
      fullCard({ graph_node_id: "ev-3" }),
    ];
    const s = press(stateOf(cards), "i").state;
    expect(s.cards[s.index].graph_node_id).toBe("ev-3");
  });

  it("skips a card a human parked", () => {
    const cards = [
      fullCard({ graph_node_id: "ev-1" }),
      fullCard({ graph_node_id: "ev-2", defer_reason: "waiting on a clean copy" }),
      fullCard({ graph_node_id: "ev-3" }),
    ];
    const s = press(stateOf(cards), "i").state;
    expect(s.cards[s.index].graph_node_id).toBe("ev-3");
  });

  it("stays put when nothing after the selection needs a decision", () => {
    const cards = [
      fullCard({ graph_node_id: "ev-1" }),
      fullCard({ graph_node_id: "ev-2", status: "dropped", status_label: "Set aside" }),
    ];
    const s = press(stateOf(cards), "i").state;
    expect(s.cards[s.index].graph_node_id).toBe("ev-1");
  });
});

describe("the card's own state, on screen, at once", () => {
  it("an include marks the card included without waiting for a reload", () => {
    // Every card wears a state chip now. Without the optimistic patch the human
    // presses I and watches the card go on saying "Not ruled" — the screen telling
    // them their ruling did not happen.
    const s = press(stateOf([fullCard({ graph_node_id: "ev-1" })]), "i").state;
    expect(s.cards[0].status).toBe("included");
  });

  it("an exclude marks it dropped, and a defer records the reason", () => {
    expect(press(stateOf([fullCard()]), "e").state.cards[0].status).toBe("dropped");

    let s = press(stateOf([fullCard()]), "d").state;
    s = queueReducer(s, { type: "defer_draft", draft: "waiting on a clean copy" }).state;
    s = press(s, "Enter").state;
    expect(s.cards[0].defer_reason).toBe("waiting on a clean copy");
    // A defer lands in `undecided` WITH a reason — that pair is what distinguishes
    // "parked" from "never looked at".
    expect(s.cards[0].status).toBe("undecided");
  });

  it("U puts the card's state back, not just the selection", () => {
    let s = press(stateOf([fullCard({ graph_node_id: "ev-1" })]), "i").state;
    s = press(s, "u").state;
    expect(s.cards[0].status).toBe("undecided");
    expect(s.cards[0].defer_reason).toBeNull();
  });

  it("a reload overwrites the optimistic state with the server's", () => {
    // The optimism is only honest because the server's answer wins: a refused
    // ruling re-reads the pool (`useQueueReducer`) and this is where that lands.
    let s = press(stateOf([fullCard({ graph_node_id: "ev-1" })]), "i").state;
    s = queueReducer(s, { type: "cards_loaded", cards: [fullCard({ graph_node_id: "ev-1" })] }).state;
    expect(s.cards[0].status).toBe("undecided");
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

  it("counts what the POOL says is ruled, not only this session's keystrokes", () => {
    // 1.7E: the list shows every card with its state chip, so a progress bar
    // reading "0 of 3 ruled" beside two coloured chips is one screen disagreeing
    // with itself (§9). A card counts once however it came to be ruled.
    const s = stateOf([
      fullCard({ graph_node_id: "ev-1", status: "included", status_label: "Included" }),
      fullCard({ graph_node_id: "ev-2", defer_reason: "waiting on a clean copy" }),
      fullCard({ graph_node_id: "ev-3" }),
    ]);
    expect(progress(s)).toEqual({ ruled: 2, total: 3 });
    // …and ruling the one that is left does not double-count the other two.
    const onLast = queueReducer(s, { type: "select", graphNodeId: "ev-3" }).state;
    expect(progress(press(onLast, "i").state)).toEqual({ ruled: 3, total: 3 });
  });

  it("keeps `total` at the WHOLE pool while a filter narrows the view", () => {
    // The section summary is a statement about the work, not about what is on
    // screen: a bar that shrank to the filter would tell Roman he was nearly done
    // every time he narrowed it.
    const s = queueReducer(stateOf([fullCard(), fullCard({ graph_node_id: "ev-2" })]), {
      type: "visible",
      ids: ["ev-2"],
    }).state;
    expect(progress(s).total).toBe(2);
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
