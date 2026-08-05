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
import { cardRows, metaChips, missingElements, REQUIRED_CARD_ELEMENTS } from "../cardRows";
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
    // Task 2.10: no human has linked this one — the ordinary state.
    human_links: [],
    human_link_summary: null,
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

  it("RULING R2: a human-linked card fills the §7.5 slot without a stance verb", () => {
    // The contract's third branch (task 2.10). A card the extraction never linked,
    // that a human HAS linked, has no stance and no defer reason — and without this
    // branch it would have no §7.5 row at all, failing the completeness test above
    // on a card that is behaving exactly as designed.
    const card = fullCard({
      stance: null,
      defer_required: false,
      defer_required_reason: null,
      human_links: [
        {
          allegation_id: "a-41",
          label: "¶41 — refused to divide the property",
          cut: "against",
          cut_label: "They'll use it against us",
        },
      ],
      human_link_summary: "You linked this to ¶41 — refused to divide the property · they'll use it against us.",
    });

    expect(missingElements(card)).toEqual([]);

    const slot = cardRows(card).filter((r) => r.element === "stance");
    expect(slot).toHaveLength(1);
    expect(slot[0].value).toBe(card.human_link_summary);
    // And it does not speak in the machine's voice.
    for (const verb of ["supports", "disputes", "comments on", "mentions"]) {
      expect(slot[0].value).not.toContain(`This ${verb}`);
    }
  });

  it("the machine's stance still wins the §7.5 slot when both exist", () => {
    const card = fullCard({
      human_link_summary: "You linked this to ¶92 · it supports us.",
    });
    const slot = cardRows(card).filter((r) => r.element === "stance");
    expect(slot).toHaveLength(1);
    expect(slot[0].value).toBe(card.stance?.summary);
  });
});

// ─── The keyboard state machine ─────────────────────────────────────────────

// ─── The metadata chip row (task 2.12, item C) ──────────────────────────────

describe("the metadata chips", () => {
  it("keeps the source and the kind of statement", () => {
    const chips = metaChips(fullCard()).map((c) => c.text);
    expect(chips).toContain("R. Phillips");
    expect(chips).toContain("extracted");
    expect(chips).toContain("partial admission");
  });

  it("KEEPS 'extracted' — it is provenance, not clutter", () => {
    // The §3 sequencing ruling: Phase-1 surfaces show raw speaker strings VISIBLY
    // LABELLED as extracted until canonical identity lands in B0 (2.1/2.2). This
    // test exists so nobody deletes it as repetitive before then.
    expect(metaChips(fullCard()).map((c) => c.text)).toContain("extracted");
  });

  it("says nothing when the quote WAS found", () => {
    // §7.7 makes grounding a warning. A chip announcing that nothing is wrong is
    // that contract read backwards.
    for (const state of ["exact", "normalized"]) {
      const card = fullCard({ grounding: { state, label: "Grounded — found on the page" } });
      expect(metaChips(card).some((c) => c.text.includes("Grounded"))).toBe(false);
    }
  });

  it("speaks, in warning styling, when the quote was NOT found", () => {
    const card = fullCard({
      grounding: { state: "not_found", label: "Not found on the page" },
    });
    const chip = metaChips(card).find((c) => c.text === "Not found on the page");
    expect(chip).toBeDefined();
    expect(chip?.warning).toBe(true);
  });

  it("treats 'not yet checked' as bad news too", () => {
    // The absence of verification is what a human about to rule needs told.
    const card = fullCard({ grounding: { state: "pending", label: "Not yet checked" } });
    expect(metaChips(card).find((c) => c.text === "Not yet checked")?.warning).toBe(true);
  });

  it("hides the confidence chip when no scan scored the card", () => {
    // Measured on S-2: all 148 read "Not scored by a scan". The absence of
    // information does not earn a line, and scan scoring is already a filter facet.
    const card = fullCard({ confidence: { band: "unscored", label: "Not scored by a scan" } });
    expect(metaChips(card).some((c) => c.text.includes("Not scored"))).toBe(false);
  });

  it("shows the band when a scan actually scored it", () => {
    const card = fullCard({ confidence: { band: "high", label: "High confidence" } });
    expect(metaChips(card).map((c) => c.text)).toContain("High confidence");
  });

  it("C-116, measured: five chips become two", () => {
    // Roman counted six on DEV; the fourth was one chip clipped at 24ch. The row
    // keeps the source and the statement kind and drops the three that said
    // nothing was wrong.
    const c116 = fullCard({
      speaker: { name: "Catholic Family Services", attribution: "extracted" },
      statement_kind: "evasive",
      grounding: { state: "normalized", label: "Grounded — found, wording normalized" },
      confidence: { band: "unscored", label: "Not scored by a scan" },
    });
    expect(metaChips(c116).map((c) => c.text)).toEqual([
      "Catholic Family Services",
      "extracted",
      "evasive",
    ]);
  });

  it("a documentary, unscored, grounded item yields no chips at all", () => {
    // Which is why the component returns null rather than an empty flex row that
    // would still take its parent's gap.
    const bare = fullCard({
      speaker: { name: null, attribution: "extracted" },
      statement_kind: null,
      grounding: { state: "exact", label: "Grounded — found on the page" },
      confidence: { band: "unscored", label: "Not scored by a scan" },
    });
    expect(metaChips(bare)).toEqual([]);
  });
});

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
    // The prompt remembers the CARD it was opened on (1.7G) as well as the draft.
    expect(state.mode).toEqual({ kind: "deferring", draft: "", graphNodeId: "ev-1" });
    expect(effect).toEqual({ kind: "none" });
  });

  it("a digit picks a quick reason without leaving the keyboard", () => {
    let s = press(stateOf([fullCard()]), "d").state;
    s = press(s, "2").state;
    expect(s.mode).toEqual({
      kind: "deferring",
      draft: DEFER_QUICK_REASONS[1],
      graphNodeId: "ev-1",
    });
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

  it("a filter that hides the selected card lands on the nearest one after it", () => {
    let s = queueReducer(stateOf(pool()), { type: "select", graphNodeId: "ev-2" }).state;
    s = visible(s, ["ev-3"]);
    expect(s.cards[s.index].graph_node_id).toBe("ev-3");
  });

  it("RULING R2: it does NOT jump to the top of the list", () => {
    // THE beta.369 DEFECT, as a test. Selection on ev-3, filter leaves ev-1 and
    // ev-3's neighbour ev-4 — the old rescue sent the selection to order[0] (ev-1,
    // the top), which is what threw Roman back to card 1 after every mouse ruling.
    const cards = [
      fullCard({ graph_node_id: "ev-1" }),
      fullCard({ graph_node_id: "ev-2" }),
      fullCard({ graph_node_id: "ev-3" }),
      fullCard({ graph_node_id: "ev-4" }),
    ];
    let s = queueReducer(stateOf(cards), { type: "select", graphNodeId: "ev-3" }).state;
    s = visible(s, ["ev-1", "ev-4"]);
    expect(s.cards[s.index].graph_node_id).toBe("ev-4");
    expect(s.cards[s.index].graph_node_id).not.toBe("ev-1");
  });

  it("falls back to the nearest card BEFORE it when nothing survives after", () => {
    // Ruling the last card in the view: being left at the bottom, where the work
    // was, beats being thrown to the top of a list already dealt with.
    let s = queueReducer(stateOf(pool()), { type: "select", graphNodeId: "ev-3" }).state;
    s = visible(s, ["ev-1", "ev-2"]);
    expect(s.cards[s.index].graph_node_id).toBe("ev-2");
  });

  it("leaves the selection where it is when the view is empty", () => {
    // The only case with nowhere to go. Moving to a card nobody can see would be
    // worse than standing still.
    let s = queueReducer(stateOf(pool()), { type: "select", graphNodeId: "ev-2" }).state;
    s = visible(s, []);
    expect(s.cards[s.index].graph_node_id).toBe("ev-2");
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

// ─── Every card rules itself (task 1.7G, ruling R1) ─────────────────────────

describe("a card's own ruling buttons", () => {
  /** Ten cards, so "the last one" is nowhere near "the selected one". */
  const pool = () =>
    Array.from({ length: 10 }, (_, i) => fullCard({ graph_node_id: `ev-${i + 1}`, code: `C-${i}` }));

  /** Click a ruling button ON a named card — the `{type: "rule"}` path. */
  function click(state: QueueState, key: "i" | "e" | "d" | "u", graphNodeId: string) {
    return queueReducer(state, { type: "rule", key, graphNodeId });
  }

  it("THE ACCEPTANCE TEST: Include on the LAST card, with the FIRST selected", () => {
    // Roman's complaint, verbatim, as a test: "open Rulable now, scroll to the LAST
    // card in the list, click its Include button first. That card — and no other —
    // is included." Nothing is selected but card 1; the ruling must not care.
    const s = stateOf(pool());
    expect(s.index).toBe(0);

    const { state, effect } = click(s, "i", "ev-10");

    expect(effect).toEqual({
      kind: "rule",
      graphNodeId: "ev-10",
      action: "include",
      reason: undefined,
    });
    expect(state.cards.find((c) => c.graph_node_id === "ev-10")?.status).toBe("included");
  });

  it("and NO OTHER card is touched", () => {
    // The other half of the sentence, and the half a passing effect assertion can
    // still hide: cards 1–9 must be exactly as they were.
    const before = stateOf(pool());
    const after = click(before, "i", "ev-10").state;

    for (const card of after.cards) {
      if (card.graph_node_id === "ev-10") continue;
      expect(card.status, `${card.graph_node_id} was changed`).toBe("undecided");
      expect(card.defer_reason).toBeNull();
    }
    expect(after.ruled).toEqual(["ev-10"]);
  });

  it("rules the card the button is on, not the one the keyboard is aimed at", () => {
    // The defect in one assertion. Select ev-2, then click ev-7's Exclude: the
    // shared-index reducer would have dropped ev-2.
    const selected = queueReducer(stateOf(pool()), {
      type: "select",
      graphNodeId: "ev-2",
    }).state;
    const { effect } = click(selected, "e", "ev-7");
    expect(effect).toMatchObject({ graphNodeId: "ev-7", action: "drop" });
  });

  it("RULING R2: the highlight lands after the card that was RULED", () => {
    // Not after the previous selection. Selection sits on ev-2; ruling ev-7 leaves
    // the human at ev-8, which is where their hand is — not at ev-3.
    const selected = queueReducer(stateOf(pool()), {
      type: "select",
      graphNodeId: "ev-2",
    }).state;
    const after = click(selected, "i", "ev-7").state;
    expect(after.cards[after.index].graph_node_id).toBe("ev-8");
  });

  it("the keyboard still rules the SELECTED card", () => {
    // The other input device is untouched by R1: I aims where the keyboard is
    // aimed, which is the selection.
    const selected = queueReducer(stateOf(pool()), {
      type: "select",
      graphNodeId: "ev-4",
    }).state;
    expect(press(selected, "i").effect).toMatchObject({ graphNodeId: "ev-4" });
  });

  it("undo takes back the most recent ruling whichever card's U is pressed", () => {
    // Single-step undo is not card-scoped, and must not become so: every card's U
    // is the same one step back. Rule ev-9, then press ev-1's U.
    const ruled = click(stateOf(pool()), "i", "ev-9").state;
    const { state, effect } = click(ruled, "u", "ev-1");

    expect(effect).toEqual({ kind: "rule", graphNodeId: "ev-9", action: "reopen" });
    expect(state.cards.find((c) => c.graph_node_id === "ev-9")?.status).toBe("undecided");
    expect(state.lastRuling).toBeNull();
  });

  it("a defer button opens the prompt ON ITS OWN card", () => {
    const { state } = click(stateOf(pool()), "d", "ev-6");
    expect(state.mode).toEqual({ kind: "deferring", draft: "", graphNodeId: "ev-6" });
  });

  it("and the reason commits to that card even if the selection moved meanwhile", () => {
    // The prompt is open on ev-6 while the human clicks another card to read it.
    // Enter must still defer ev-6 — the card they opened the prompt for.
    let s = click(stateOf(pool()), "d", "ev-6").state;
    s = queueReducer(s, { type: "select", graphNodeId: "ev-1" }).state;
    s = queueReducer(s, { type: "defer_draft", draft: "need the full page" }).state;
    const { effect } = press(s, "Enter");

    expect(effect).toEqual({
      kind: "rule",
      graphNodeId: "ev-6",
      action: "defer",
      reason: "need the full page",
    });
  });

  it("a click on another card's button abandons an open prompt rather than eating it", () => {
    // The prompt owns the KEYBOARD, not the mouse. Swallowing a click on a named
    // card's own button would be the dead-keyboard defect in mouse form.
    const open = click(stateOf(pool()), "d", "ev-6").state;
    const { state, effect } = click(open, "i", "ev-3");

    expect(effect).toMatchObject({ graphNodeId: "ev-3", action: "include" });
    expect(state.mode).toEqual({ kind: "triage" });
  });

  it("but Defer again on the SAME card keeps the half-typed reason", () => {
    let s = click(stateOf(pool()), "d", "ev-6").state;
    s = queueReducer(s, { type: "defer_draft", draft: "half a thou" }).state;
    const { state, effect } = click(s, "d", "ev-6");

    expect(effect).toEqual({ kind: "none" });
    expect(state.mode).toEqual({ kind: "deferring", draft: "half a thou", graphNodeId: "ev-6" });
  });

  it("refuses I on a defer-only card and shows the reason ON that card", () => {
    // The refusal selects the card it is about, so the sentence appears where the
    // human just acted rather than beside whatever was selected.
    const cards = [fullCard({ graph_node_id: "ev-1" }), unrulableCard("ev-222")];
    const { state, effect } = click(stateOf(cards), "i", "ev-222");

    expect(effect).toEqual({ kind: "none" });
    expect(state.notice).toBe(cards[1].defer_required_reason);
    expect(state.cards[state.index].graph_node_id).toBe("ev-222");
  });

  it("ignores a click on a card the pool no longer holds", () => {
    // A reload landing mid-click. Ruling something the human cannot see is worse
    // than dropping one click.
    const { state, effect } = click(stateOf(pool()), "i", "ev-gone");
    expect(effect).toEqual({ kind: "none" });
    expect(state.ruled).toEqual([]);
  });

  it("the Defer BUTTON takes the server's reason in one press, like the key does", () => {
    // The two callers of `rulingOn` are only equivalent if both reach the same
    // branches. The keyboard's short-circuit on a defer-only card has been tested
    // since 1.7C; this is the same branch reached through the button, and without
    // it a click could open an empty prompt where the key writes the ruling.
    const cards = [fullCard({ graph_node_id: "ev-1" }), unrulableCard("ev-222")];
    const { state, effect } = click(stateOf(cards), "d", "ev-222");

    expect(effect).toEqual({
      kind: "rule",
      graphNodeId: "ev-222",
      action: "defer",
      reason: cards[1].defer_required_reason,
    });
    expect(state.mode).toEqual({ kind: "triage" });
  });

  it("closes the prompt without ruling when its card leaves the pool mid-typing", () => {
    // Reachable, not theoretical: `cards_loaded` does not close an open prompt, so
    // a reload that drops the card while the human is typing their reason leaves
    // the prompt pointing at a card that is gone. Enter must then write NOTHING —
    // the alternative is a defer against whatever else is in scope, which is the
    // class of bug the target-carrying prompt exists to prevent.
    let s = click(stateOf(pool()), "d", "ev-6").state;
    s = queueReducer(s, { type: "defer_draft", draft: "waiting on a clean copy" }).state;
    s = queueReducer(s, {
      type: "cards_loaded",
      cards: pool().filter((c) => c.graph_node_id !== "ev-6"),
    }).state;
    const { state, effect } = press(s, "Enter");

    expect(effect).toEqual({ kind: "none" });
    expect(state.mode).toEqual({ kind: "triage" });
    expect(state.ruled).toEqual([]);
  });

  it("RULING R2: ruling the LAST card moves the highlight off where it was", () => {
    // THE DEFECT THE DEV CLICK-THROUGH CAUGHT, and the reason it got past the
    // suite: every earlier R2 test either ruled a card with something after it
    // (so `advance` had somewhere to go) or exercised the filter rescue in
    // ISOLATION with the selection already sitting on the ruled card. Neither
    // covers the real sequence — selection on card 1, click Include on card 42.
    // `advance` returned its input untouched, so the selection stayed on card 1;
    // the rescue then saw a selection that was still visible and left it there.
    // Ruling the bottom of the list parked the highlight at the top.
    const s = stateOf(pool()); // selection is on ev-1, the first card
    expect(s.index).toBe(0);
    const after = click(s, "i", "ev-10").state;
    expect(after.cards[after.index].graph_node_id).toBe("ev-10");
  });

  it("…and the filter then lands it on the nearest survivor, not the top", () => {
    // The other half of the same sequence, end to end: the ruled card leaves the
    // "Rulable now" view, the queue reports the new visible set, and the highlight
    // has to come to rest beside the work — ev-9 — rather than back at ev-1.
    const ids = pool().map((c) => c.graph_node_id);
    let s = queueReducer(stateOf(pool()), { type: "visible", ids }).state;
    s = click(s, "i", "ev-10").state;
    s = queueReducer(s, { type: "visible", ids: ids.filter((id) => id !== "ev-10") }).state;

    expect(s.cards[s.index].graph_node_id).toBe("ev-9");
    expect(s.cards[s.index].graph_node_id).not.toBe("ev-1");
  });

  it("advances from the top of the view when the ruled card is not in it", () => {
    // The recovery path in `advance`: a filter changed under the ruling, so the
    // anchor is nowhere in the visible order. Scanning forward from the start is
    // the honest landing — it puts the human on the first thing still to do
    // rather than leaving the selection on a card the filter is hiding.
    let s = queueReducer(stateOf(pool()), { type: "visible", ids: ["ev-3", "ev-4"] }).state;
    s = click(s, "i", "ev-1").state;
    expect(s.cards[s.index].graph_node_id).toBe("ev-3");
  });
});

// ─── Task 2.10: a card's own LINK control ───────────────────────────────────
//
// The same law as the ruling buttons above, and the same acceptance test, because
// the same defect would be the same disaster: a link decides whether a card can be
// ruled at all, so a link that lands on the wrong card silently unlocks the wrong
// evidence.

describe("a card's own link control", () => {
  /** Ten stuck cards, so "the last one" is nowhere near "the selected one". */
  const stuck = () =>
    Array.from({ length: 10 }, (_, i) =>
      fullCard({
        graph_node_id: `ev-${i + 1}`,
        code: `C-${i}`,
        // The stuck class: the extraction linked this to nothing.
        stance: null,
        defer_required: true,
        defer_required_reason: "This item is not linked to any accusation yet.",
      }),
    );

  /** Save the link control ON a named card — the `{type: "link"}` path. */
  function link(state: QueueState, graphNodeId: string) {
    return queueReducer(state, {
      type: "link",
      graphNodeId,
      allegationIds: ["a-41"],
      cut: "against",
    });
  }

  it("THE ACCEPTANCE TEST: link the LAST card, with the FIRST selected", () => {
    // Roman's DEV verify line, verbatim, as a test: "Filter S-2 to the stuck
    // cards. Link the LAST one in the view first, without selecting it — it links,
    // and no other card changes."
    const s = stateOf(stuck());
    expect(s.index).toBe(0);

    const { effect } = link(s, "ev-10");

    expect(effect).toEqual({
      kind: "link",
      graphNodeId: "ev-10",
      allegationIds: ["a-41"],
      cut: "against",
    });
  });

  it("and NO OTHER card is touched", () => {
    // The half a passing effect assertion can still hide.
    const before = stateOf(stuck());
    const after = link(before, "ev-10").state;

    expect(after.cards).toEqual(before.cards);
    expect(after.ruled).toEqual([]);
  });

  it("links the card the control is on, not the one the keyboard is aimed at", () => {
    const selected = queueReducer(stateOf(stuck()), {
      type: "select",
      graphNodeId: "ev-2",
    }).state;
    expect(selected.cards[selected.index].graph_node_id).toBe("ev-2");

    const { effect } = link(selected, "ev-7");
    expect(effect).toMatchObject({ kind: "link", graphNodeId: "ev-7" });
  });

  it("any card, any order — three links, three distinct targets", () => {
    // "Save and next is a convenience, never the only path" (ruling R1), as a
    // test: nothing here selects anything, and the order is deliberately random.
    let s = stateOf(stuck());
    const targets: string[] = [];
    for (const id of ["ev-9", "ev-3", "ev-6"]) {
      const result = link(s, id);
      s = result.state;
      if (result.effect.kind === "link") targets.push(result.effect.graphNodeId);
    }
    expect(targets).toEqual(["ev-9", "ev-3", "ev-6"]);
  });

  it("ITEM A: saving NEVER moves the selection", () => {
    // Roman's words: "I need to scroll up to the card I was working and then
    // press Include." Saving used to advance, so the human did the thinking on a
    // card, was moved away, and had to scroll backwards to rule it. The selection
    // now stays put and the ordinary post-ruling advance does the moving.
    const s = queueReducer(stateOf(stuck()), { type: "select", graphNodeId: "ev-4" }).state;
    const after = link(s, "ev-8").state;
    expect(after.cards[after.index].graph_node_id).toBe("ev-4");
  });

  it("…including when the human links the card they are sitting on", () => {
    // The ordinary case after item A: link the selected card, then press I on it
    // without moving. If the save advanced, I would land on the wrong card.
    const s = queueReducer(stateOf(stuck()), { type: "select", graphNodeId: "ev-4" }).state;
    const after = link(s, "ev-4").state;
    expect(after.cards[after.index].graph_node_id).toBe("ev-4");
  });

  it("does NOT patch the card optimistically", () => {
    // Ruling R2 forbids the browser composing what a linked card says — the
    // sentence, the chips and the unlocked buttons are all server-composed, so the
    // card must stay exactly as it was until the pool is re-read.
    const before = stateOf(stuck());
    const after = link(before, "ev-3").state;
    const card = after.cards.find((c) => c.graph_node_id === "ev-3");

    expect(card?.human_links).toEqual([]);
    expect(card?.human_link_summary).toBeNull();
    expect(card?.defer_required).toBe(true);
  });

  it("ignores a link on a card the pool no longer holds", () => {
    const { state, effect } = link(stateOf(stuck()), "ev-gone");
    expect(effect).toEqual({ kind: "none" });
    expect(state.cards).toHaveLength(10);
  });

  it("unlink names its own card and its own accusation", () => {
    const { effect } = queueReducer(stateOf(stuck()), {
      type: "unlink",
      graphNodeId: "ev-6",
      allegationId: "a-92",
    });
    expect(effect).toEqual({ kind: "unlink", graphNodeId: "ev-6", allegationId: "a-92" });
  });

  it("ignores an unlink on a card the pool no longer holds", () => {
    const { effect } = queueReducer(stateOf(stuck()), {
      type: "unlink",
      graphNodeId: "ev-gone",
      allegationId: "a-92",
    });
    expect(effect).toEqual({ kind: "none" });
  });

  it("a link abandons an open defer prompt rather than committing it", () => {
    // Same reasoning as a ruling on a named card: a click on this card's own Save
    // is an unambiguous statement about this card, and the half-typed reason for a
    // DIFFERENT card must not ride along with it.
    // A defer-only card short-circuits D (it already has a server-composed
    // reason), so the prompt is opened on a rulable card.
    const s = queueReducer(stateOf(fullCards()), {
      type: "rule",
      key: "d",
      graphNodeId: "ev-2",
    }).state;
    expect(s.mode.kind).toBe("deferring");

    const after = queueReducer(s, {
      type: "link",
      graphNodeId: "ev-5",
      allegationIds: ["a-41"],
      cut: "against",
    });
    expect(after.state.mode.kind).toBe("triage");
    expect(after.effect).toMatchObject({ kind: "link", graphNodeId: "ev-5" });
  });

  /** Ten RULABLE cards, for the defer-prompt case above. */
  function fullCards() {
    return Array.from({ length: 10 }, (_, i) =>
      fullCard({ graph_node_id: `ev-${i + 1}`, code: `C-${i}` }),
    );
  }
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
  it("a reload DROPS a card the server says is no longer ruled (task 2.12)", () => {
    // Measured on DEV (beta.375): after removing an included card from its fact
    // row, the filters read "Included (46)" and "Not ruled (92)" while the
    // progress line still read "57 of 148 ruled" against an authoritative 56.
    //
    // `progress` unions the session's ruled list with the payload's own states,
    // which was safe while `undo` was the only way to un-rule a card — it removes
    // the id itself. Removing from a fact row un-rules a card from OUTSIDE the
    // queue and cannot reach that list, so the count kept the card forever.
    const cards = () => [fullCard({ graph_node_id: "ev-3", code: "C-3" })];
    let s = stateOf(cards());
    s = queueReducer(s, { type: "rule", key: "i", graphNodeId: "ev-3" }).state;
    expect(progress(s).ruled).toBe(1);
    expect(s.ruled).toContain("ev-3");

    // The server's word: ev-3 is not ruled any more (it was removed elsewhere).
    const reloaded = queueReducer(s, { type: "cards_loaded", cards: cards() }).state;
    expect(reloaded.ruled).not.toContain("ev-3");
    expect(progress(reloaded).ruled).toBe(0);
  });

  it("…but a reload KEEPS one the server still says is ruled", () => {
    // The other half: reconciliation must not throw away a live entry just
    // because a reload happened.
    let s = stateOf([fullCard({ graph_node_id: "ev-3", code: "C-3" })]);
    s = queueReducer(s, { type: "rule", key: "i", graphNodeId: "ev-3" }).state;

    const server = [fullCard({ graph_node_id: "ev-3", code: "C-3" })].map((c) =>
      c.graph_node_id === "ev-3" ? { ...c, status: "included" as const } : c,
    );
    const reloaded = queueReducer(s, { type: "cards_loaded", cards: server }).state;
    expect(reloaded.ruled).toContain("ev-3");
    expect(progress(reloaded).ruled).toBe(1);
  });

  it("…and KEEPS one for a card the reloaded pool no longer holds", () => {
    // A pool that SHRANK is not a ruling that was undone. Dropping the entry
    // would silently lower the count for work the human really did.
    let s = stateOf([
      fullCard({ graph_node_id: "ev-3", code: "C-3" }),
      fullCard({ graph_node_id: "ev-4", code: "C-4" }),
    ]);
    s = queueReducer(s, { type: "rule", key: "i", graphNodeId: "ev-3" }).state;

    const shorter = [fullCard({ graph_node_id: "ev-4", code: "C-4" })];
    const reloaded = queueReducer(s, { type: "cards_loaded", cards: shorter }).state;
    expect(reloaded.ruled).toContain("ev-3");
  });

  it("…and U after that reload undoes NOTHING, because there is nothing to undo", () => {
    // The gap the reconciliation opened. `rule` sets `lastRuling` and appends to
    // `ruled` together, so the two had never been able to disagree — until a
    // reload started dropping ids from one of them.
    //
    // Left alone, U would emit a `reopen` for a card the server has already
    // un-ruled: a write that creates a fact-ref row and a ledger entry for an act
    // that undoes nothing, and snaps the human's focus to where that card was.
    let s = stateOf([fullCard({ graph_node_id: "ev-3", code: "C-3" })]);
    s = queueReducer(s, { type: "rule", key: "i", graphNodeId: "ev-3" }).state;
    expect(s.lastRuling?.graphNodeId).toBe("ev-3");

    // The server's word: ev-3 is not ruled (it was removed from its fact row).
    s = queueReducer(s, {
      type: "cards_loaded",
      cards: [fullCard({ graph_node_id: "ev-3", code: "C-3" })],
    }).state;
    expect(s.lastRuling).toBeNull();

    const pressed = queueReducer(s, { type: "key", key: "u", typing: false });
    expect(pressed.effect).toEqual({ kind: "none" });
  });

  it("…but a reload KEEPS the undo target when the ruling still stands", () => {
    // The other half: an ordinary reload must not cost the human their single
    // step back.
    let s = stateOf([fullCard({ graph_node_id: "ev-3", code: "C-3" })]);
    s = queueReducer(s, { type: "rule", key: "i", graphNodeId: "ev-3" }).state;

    const server = [fullCard({ graph_node_id: "ev-3", code: "C-3", status: "included" })];
    s = queueReducer(s, { type: "cards_loaded", cards: server }).state;

    expect(s.lastRuling?.graphNodeId).toBe("ev-3");
    const pressed = queueReducer(s, { type: "key", key: "u", typing: false });
    expect(pressed.effect).toMatchObject({ kind: "rule", action: "reopen", graphNodeId: "ev-3" });
  });

});
