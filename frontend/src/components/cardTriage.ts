// =============================================================================
// cardTriage.ts — the triage queue's pure core (task 1.3)
// =============================================================================
//
// Two pure things live here, and the component next door renders them:
//
//   1. `cardRows` — the §7 CONTRACT as a descriptor. Every element the card must
//      show, as a list of labelled rows built from the payload. §7 makes a card
//      missing any element a defect, so the contract is testable by asserting
//      against this list rather than against the DOM.
//   2. `queueReducer` — the keyboard state machine. Advance, undo, the defer
//      prompt, and the typing guard are all state transitions over plain data,
//      so every one of them is testable without a browser.
//
// ## Why a descriptor instead of just writing JSX
//
// CLAUDE.md rule 30 records that component-test infrastructure (RTL, jsdom) is
// deliberately not set up, and 1.3 is not the place to reverse that. Pulling the
// contract into a pure function means the assertion "every §7 element is
// present" is a real test rather than a hope — the part the tests cannot reach
// is only that the JSX faithfully walks the list, which is what Roman's DEV
// verify line covers.
//
// ## The frontend computes NOTHING
//
// Every string in a row's `value` comes verbatim from the payload. This module
// chooses which rows exist and in what order; it never composes prose, maps a
// vocabulary, formats a number, or builds a URL (v2 §7 item 2).

import type { ScenarioCard } from "../services/scenarioCards";
import type { FactAction } from "../services/scenarioGather";

// ─── The §7 descriptor ──────────────────────────────────────────────────────

/** Which §7 element a row implements. The contract test asserts on these. */
export type CardElement =
  | "quote"
  | "pinpoint"
  | "speaker"
  | "statement_kind"
  | "stance"
  | "bears_on"
  | "grounding"
  | "confidence"
  | "status"
  | "code";

/** One rendered line of a card. */
export type CardRow = {
  element: CardElement;
  /** The human-readable text, verbatim from the payload. */
  value: string;
  /** Chip-style values the card renders as separate pills rather than prose. */
  chips?: string[];
  /** Present only on the pinpoint row — the full-viewer link. */
  href?: string;
};

/**
 * Build the descriptor for one card: every §7 element, in display order.
 *
 * ## Why some rows are conditional and what that means for the contract
 *
 * A row is omitted only when the payload genuinely has nothing to say — a
 * documentary item with no speaker, an extraction with no statement kind. The
 * one structural case is §7.5: a card either has a `stance` (verb + object) or
 * it has a `defer_required_reason` explaining why it cannot be ruled on. It is
 * never missing both, and it never shows a bare verb — that was the July defect.
 * The contract test asserts exactly one of those two is present.
 */
export function cardRows(card: ScenarioCard): CardRow[] {
  const rows: CardRow[] = [];

  if (card.code) rows.push({ element: "code", value: card.code });

  // §7.1 — the quote is always present, context and question when they exist.
  rows.push({ element: "quote", value: card.quote.text });

  // §7.2 — the pinpoint, pre-composed by the backend, with its own jump target.
  // The title and page are NOT joined here: composing them would be the browser
  // making a presentation decision about case vocabulary, which is the exact thing
  // `CardStance.summary` ships pre-composed to prevent one field over.
  rows.push({
    element: "pinpoint",
    value: card.pinpoint.label,
    href: card.pinpoint.viewer_href,
  });

  // §7.3 — speaker with its attribution, then the kind of statement.
  if (card.speaker.name) {
    rows.push({
      element: "speaker",
      value: card.speaker.name,
      chips: [card.speaker.attribution],
    });
  }
  if (card.statement_kind) {
    rows.push({ element: "statement_kind", value: card.statement_kind });
  }

  // §7.5 — the stance WITH its object, or the reason there is none.
  if (card.stance) {
    rows.push({ element: "stance", value: card.stance.summary });
  } else if (card.defer_required_reason) {
    rows.push({ element: "stance", value: card.defer_required_reason });
  }

  // §7.6 — every accusation, with its elements and count as chips.
  for (const bears of card.bears_on) {
    rows.push({
      element: "bears_on",
      value: bears.accusation,
      chips: bears.count ? [...bears.elements, bears.count] : bears.elements,
    });
  }

  // §7.7 / §7.8 — grounding and the confidence band.
  if (card.grounding) {
    rows.push({ element: "grounding", value: card.grounding.label });
  }
  rows.push({ element: "confidence", value: card.confidence.label });

  rows.push({ element: "status", value: card.status_label });

  return rows;
}

/**
 * The §7 elements a card MUST carry to be rulable.
 *
 * Deliberately not every element: speaker and statement kind are legitimately
 * absent on documentary evidence, and `code` is absent until gather numbers the
 * candidate. These five are the ones whose absence makes the card a defect.
 */
export const REQUIRED_CARD_ELEMENTS: CardElement[] = [
  "quote",
  "pinpoint",
  "stance",
  "confidence",
  "status",
];

/** Which required §7 elements a card is missing. Empty means the card is whole. */
export function missingElements(card: ScenarioCard): CardElement[] {
  const present = new Set(cardRows(card).map((r) => r.element));
  return REQUIRED_CARD_ELEMENTS.filter((e) => !present.has(e));
}

// ─── The keyboard state machine ─────────────────────────────────────────────

// CONST: the quick-pick defer reasons. These are UI affordances, not
// configuration: each is a shortcut for typing a sentence the human could type
// anyway, and the free-text field beside them accepts anything. Making them
// configurable would add a settings surface whose only effect is which three
// suggestions appear above an unrestricted input — the reason a defer records is
// whatever the human wrote, and that is never constrained to this list.
//
// They are also deliberately case-agnostic: nothing here names a party, a
// document or a claim, so another Colossus case renders them unchanged
// (Standing Rule 2's reusability checkpoint).
export const DEFER_QUICK_REASONS = [
  "Need to read the full page first",
  "Waiting on a clearer copy of the document",
  "Not sure this belongs in this scenario",
];

/** What the queue is doing right now. */
export type QueueMode =
  | { kind: "triage" }
  /** The defer prompt is open on the focused card; keys type instead of ruling. */
  | { kind: "deferring"; draft: string };

/** The last ruling, kept for single-step undo. */
export type LastRuling = {
  graphNodeId: string;
  action: FactAction;
  /** Where the focus was, so undo returns the human to the card they ruled. */
  index: number;
};

export type QueueState = {
  /** Every card in the working pool, in payload order. */
  cards: ScenarioCard[];
  /** Index of the focused card. */
  index: number;
  mode: QueueMode;
  /** `null` once undone — undo is single-step, never a stack. */
  lastRuling: LastRuling | null;
  /** Node ids ruled in this session, for the running count. */
  ruled: string[];
};

/** A ruling the reducer wants the caller to send to the backend. */
export type QueueEffect =
  | { kind: "rule"; graphNodeId: string; action: FactAction; reason?: string }
  | { kind: "none" };

export type QueueEvent =
  | { type: "key"; key: string; typing: boolean }
  | { type: "focus"; index: number }
  | { type: "defer_draft"; draft: string }
  | { type: "cards_loaded"; cards: ScenarioCard[] };

export type QueueResult = { state: QueueState; effect: QueueEffect };

export function initialQueueState(cards: ScenarioCard[]): QueueState {
  return { cards, index: 0, mode: { kind: "triage" }, lastRuling: null, ruled: [] };
}

const NONE: QueueEffect = { kind: "none" };

/**
 * Advance to the next card, stopping at the end rather than wrapping.
 *
 * ## Domain note: why it does not wrap
 *
 * Wrapping would silently put the human back at the top of a list they thought
 * they had finished, and the running count is what tells them they are done.
 * Stopping at the last card makes "the queue is exhausted" a visible state.
 */
function advance(state: QueueState): QueueState {
  return { ...state, index: Math.min(state.index + 1, Math.max(state.cards.length - 1, 0)) };
}

/**
 * Apply one event. Pure: returns the next state and any backend call to make.
 *
 * ## Rust/TS learning: a reducer that returns an EFFECT rather than performing it
 *
 * The reducer never calls the network. It returns a description of the call the
 * caller should make, which is what keeps the whole state machine testable
 * without mocking `fetch`. The caller performs the effect and reconciles the
 * result — the UI advances optimistically, but the server's answer is what the
 * next reload shows (v2: zero business logic in the frontend).
 */
export function queueReducer(state: QueueState, event: QueueEvent): QueueResult {
  switch (event.type) {
    case "cards_loaded": {
      // A reload keeps the human where they were, clamped to the new length —
      // re-fetching after a ruling must not throw them back to the top.
      const index = Math.min(state.index, Math.max(event.cards.length - 1, 0));
      return { state: { ...state, cards: event.cards, index }, effect: NONE };
    }

    case "focus":
      return { state: { ...state, index: event.index }, effect: NONE };

    case "defer_draft":
      return state.mode.kind === "deferring"
        ? { state: { ...state, mode: { kind: "deferring", draft: event.draft } }, effect: NONE }
        : { state, effect: NONE };

    case "key":
      return handleKey(state, event.key, event.typing);
  }
}

function handleKey(state: QueueState, key: string, typing: boolean): QueueResult {
  const card = state.cards[state.index];

  // The defer prompt owns the keyboard while it is open.
  if (state.mode.kind === "deferring") {
    return handleDeferKey(state, key, state.mode.draft);
  }

  // The typing guard: while focus is in a field, letters type, they do not rule.
  // Without this, typing a note would silently include half a pool.
  if (typing || !card) return { state, effect: NONE };

  switch (key.toLowerCase()) {
    case "i":
    case "e": {
      // An unrulable card refuses in the UI rather than making a round trip that
      // returns 400 — the human learns why from the reason already on the card.
      if (card.defer_required) return { state, effect: NONE };
      const action: FactAction = key.toLowerCase() === "i" ? "include" : "drop";
      return rule(state, card.graph_node_id, action);
    }

    case "d": {
      // The short-circuit: a defer-required card already carries a server-composed
      // reason, so one press accepts it. Prompting would ask the human to
      // re-type a sentence the system already wrote.
      if (card.defer_required && card.defer_required_reason) {
        return rule(state, card.graph_node_id, "defer", card.defer_required_reason);
      }
      return { state: { ...state, mode: { kind: "deferring", draft: "" } }, effect: NONE };
    }

    case "u":
      return undo(state);

    default:
      return { state, effect: NONE };
  }
}

function handleDeferKey(state: QueueState, key: string, draft: string): QueueResult {
  if (key === "Escape") {
    return { state: { ...state, mode: { kind: "triage" } }, effect: NONE };
  }
  if (key === "Enter") {
    const reason = draft.trim();
    // A blank reason is not a defer — the backend refuses it, and refusing here
    // keeps the prompt open with the human's cursor in it rather than bouncing
    // an error back at them.
    if (!reason) return { state, effect: NONE };
    const card = state.cards[state.index];
    if (!card) return { state: { ...state, mode: { kind: "triage" } }, effect: NONE };
    const ruled = rule(state, card.graph_node_id, "defer", reason);
    return { state: { ...ruled.state, mode: { kind: "triage" } }, effect: ruled.effect };
  }
  // Digits pick a quick reason without leaving the keyboard.
  const pick = Number.parseInt(key, 10);
  if (!Number.isNaN(pick) && pick >= 1 && pick <= DEFER_QUICK_REASONS.length) {
    return {
      state: { ...state, mode: { kind: "deferring", draft: DEFER_QUICK_REASONS[pick - 1] } },
      effect: NONE,
    };
  }
  return { state, effect: NONE };
}

/** Record a ruling, advance, and ask the caller to send it. */
function rule(
  state: QueueState,
  graphNodeId: string,
  action: FactAction,
  reason?: string,
): QueueResult {
  const next = advance({
    ...state,
    lastRuling: { graphNodeId, action, index: state.index },
    // A card ruled twice counts once — the running count is "how many of the
    // pool have been dealt with", not "how many keys were pressed".
    ruled: state.ruled.includes(graphNodeId) ? state.ruled : [...state.ruled, graphNodeId],
  });
  return { state: next, effect: { kind: "rule", graphNodeId, action, reason } };
}

/**
 * Take back the last ruling: return that card to undecided and refocus it.
 *
 * Single-step by construction — `lastRuling` is cleared, so a second U does
 * nothing. A stack would let a human unwind a whole session by leaning on one
 * key, which is a worse failure than having to re-rule one card.
 *
 * The verb is `reopen`, never `undrop`: they reach the same state, but the
 * ledger records the word, and "undrop" on an item that was never dropped is a
 * false entry in the forensic record.
 */
function undo(state: QueueState): QueueResult {
  const last = state.lastRuling;
  if (!last) return { state, effect: NONE };
  return {
    state: {
      ...state,
      index: last.index,
      lastRuling: null,
      ruled: state.ruled.filter((id) => id !== last.graphNodeId),
    },
    effect: { kind: "rule", graphNodeId: last.graphNodeId, action: "reopen" },
  };
}

/** The running count the queue always shows. */
export function progress(state: QueueState): { ruled: number; total: number } {
  return { ruled: state.ruled.length, total: state.cards.length };
}
