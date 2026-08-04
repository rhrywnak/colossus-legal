// =============================================================================
// cardTriage.ts — the triage queue's state machine (task 1.3, extended in 1.7E)
// =============================================================================
//
// What a KEY DOES. Selection, navigation, advance, undo, the defer prompt and
// the typing guard are all state transitions over plain data, so every one of
// them is testable without a browser (CLAUDE.md rule 30: no RTL, no jsdom, by
// deliberate convention).
//
// What a card SHOWS moved to `cardRows.ts` in 1.7E — see that file's header for
// the seam.
//
// ## What 1.7E added, and why the reducer had to learn about it
//
// The queue became a scrollable, filterable LIST. Three of those consequences are
// state-machine business rather than chrome:
//
//   * `visibleIds` — navigation and auto-advance must walk the FILTERED order, or
//     the keyboard would select cards the filter is hiding.
//   * `notice` — I or E on a defer-only card refused SILENTLY before. A key that
//     does nothing and says nothing is indistinguishable from a dead keyboard.
//   * the optimistic card patch — every card now wears its state as a chip, so a
//     ruling has to be visible on the card the instant it is made.
//
// The §7 ruling semantics are unchanged and extended, never weakened: one-key
// I/E/D on the selected card, U single-step undo, auto-advance after a ruling,
// the typing guard, the defer quick-pick, the defer-required refusal (now
// visible).
//
// ## The reducer still computes no case vocabulary
//
// The one string it ever holds is `notice`, and that is the backend's own
// `defer_required_reason` passed through untouched.

import type { ScenarioCard } from "../services/scenarioCards";
import type { FactAction } from "../services/scenarioGather";
import { candidateState } from "./candidateFilters";

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
  /** Index of the SELECTED card — the one the keyboard is aimed at. */
  index: number;
  mode: QueueMode;
  /** `null` once undone — undo is single-step, never a stack. */
  lastRuling: LastRuling | null;
  /** Node ids ruled in this session, for the running count. */
  ruled: string[];
  /**
   * The cards the active filter leaves visible, in display order (task 1.7E).
   *
   * `null` means no filter has spoken — every card is visible, which is the state
   * a queue mounted without a filter bar stays in forever. Navigation and
   * auto-advance walk THIS order, so a filtered list moves within itself and
   * never selects a card the human cannot see.
   *
   * Ids rather than indices: the filter is recomputed against a pool that
   * reloads, and an index into yesterday's array is a wrong card rather than a
   * missing one.
   */
  visibleIds: string[] | null;
  /**
   * A refusal the human must be told about, or `null`.
   *
   * Today it carries one thing: the reason I or E did nothing on a defer-only
   * card. 1.7C refused those keys silently — correct in that it made no doomed
   * round trip, wrong in that the human pressed a key and the screen did not
   * react at all (Standing Rule 1: a distinct state needs a distinct observable).
   *
   * The STRING is always the backend's own `defer_required_reason`, never a
   * sentence composed here — which also means it stays true for the second
   * unrulable class (an item with no verbatim quote), whose reason is a different
   * sentence entirely.
   */
  notice: string | null;
};

/** A ruling the reducer wants the caller to send to the backend. */
export type QueueEffect =
  | { kind: "rule"; graphNodeId: string; action: FactAction; reason?: string }
  | { kind: "none" };

export type QueueEvent =
  | { type: "key"; key: string; typing: boolean }
  | { type: "focus"; index: number }
  /** A click on a card in the list — selection by identity, never by position. */
  | { type: "select"; graphNodeId: string }
  /** The filter bar reporting which cards it leaves visible, in display order. */
  | { type: "visible"; ids: string[] }
  | { type: "defer_draft"; draft: string }
  | { type: "cards_loaded"; cards: ScenarioCard[] };

export type QueueResult = { state: QueueState; effect: QueueEffect };

export function initialQueueState(cards: ScenarioCard[]): QueueState {
  return {
    cards,
    index: 0,
    mode: { kind: "triage" },
    lastRuling: null,
    ruled: [],
    visibleIds: null,
    notice: null,
  };
}

const NONE: QueueEffect = { kind: "none" };

/**
 * The positions of the visible cards, in the order the list renders them.
 *
 * A visible id with no card behind it is dropped rather than reported: it means
 * the filter was computed against a pool that has since reloaded shorter, which
 * lasts exactly until the component recomputes and dispatches `visible` again. It
 * is a one-render ordering artefact, not an operational failure — there is
 * nothing for an operator to do about it and nothing lost by it.
 */
function visibleOrder(state: QueueState): number[] {
  if (state.visibleIds === null) return state.cards.map((_, i) => i);
  const positions = new Map(state.cards.map((card, i) => [card.graph_node_id, i]));
  const order: number[] = [];
  for (const id of state.visibleIds) {
    const at = positions.get(id);
    if (at !== undefined) order.push(at);
  }
  return order;
}

/**
 * Whether a card has been dealt with — by anyone, in any session.
 *
 * Two sources, deliberately: the payload's own state (someone ruled it before
 * today) and this session's `ruled` list (the human just did, and the optimistic
 * patch below has already updated the card, so this second half is belt and
 * braces for a card that arrived ruled and was ruled again).
 */
function isDealtWith(state: QueueState, card: ScenarioCard): boolean {
  return candidateState(card) !== "not_ruled" || state.ruled.includes(card.graph_node_id);
}

/**
 * Advance to the next UNRULED visible card, stopping rather than wrapping.
 *
 * ## Why it skips what is already ruled (task 1.7E re-anchor)
 *
 * Until 1.7E the queue showed one unruled card at a time, so "the next card" and
 * "the next card needing a decision" were the same thing. The list shows ruled
 * cards too — that is the point of the state chips — so a plain +1 would now park
 * the human on a card they have already decided after every ruling.
 *
 * ## Domain note: why it does not wrap
 *
 * Wrapping would silently put the human back at the top of a list they thought
 * they had finished, and the running count is what tells them they are done.
 * Stopping at the last card makes "the queue is exhausted" a visible state.
 */
function advance(state: QueueState): QueueState {
  const order = visibleOrder(state);
  const here = order.indexOf(state.index);
  // A selection outside the visible set can only mean a filter changed under the
  // ruling; scanning the whole visible order forward from the start is the honest
  // recovery, and it lands on the first thing left to do.
  const from = here === -1 ? -1 : here;
  for (let at = from + 1; at < order.length; at += 1) {
    const card = state.cards[order[at]];
    if (card && !isDealtWith(state, card)) return { ...state, index: order[at] };
  }
  return state;
}

/**
 * Move the selection one step through the visible list WITHOUT ruling anything.
 *
 * The whole point of item 3: browsing is free. Nothing is written until I, E or D
 * is pressed, which is why this returns `effect: none` and why that is the
 * assertion the tests lean on hardest.
 */
function moveSelection(state: QueueState, step: 1 | -1): QueueResult {
  const order = visibleOrder(state);
  if (order.length === 0) return { state, effect: NONE };
  const here = order.indexOf(state.index);
  // Not on a visible card (a filter just changed): the first step lands on the
  // first visible one rather than nowhere.
  if (here === -1) return { state: { ...state, index: order[0] }, effect: NONE };
  const next = Math.min(Math.max(here + step, 0), order.length - 1);
  return { state: { ...state, index: order[next] }, effect: NONE };
}

/**
 * The card as it will read once the server accepts this ruling.
 *
 * ## Why the reducer patches its own copy of the card
 *
 * The list wears a state chip on every card (item 2). Without this patch, a human
 * who presses I watches the card stay "Not ruled" until something else reloads
 * the pool — the screen telling them their ruling did not happen. The optimistic
 * advance has always worked this way; this extends it to the one field that is
 * now visible.
 *
 * It stays honest because the failure path already exists: a refused ruling
 * surfaces an alert AND re-reads the pool (`useQueueReducer`), so what survives
 * on screen is what the database holds.
 */
function applyRulingToCard(
  card: ScenarioCard,
  action: FactAction,
  reason: string | undefined,
): ScenarioCard {
  switch (action) {
    case "include":
      return { ...card, status: "included", defer_reason: null };
    case "drop":
      return { ...card, status: "dropped", defer_reason: null };
    case "defer":
      // A defer lands in `undecided` WITH a reason — that pair is what
      // distinguishes "parked" from "never looked at" (backend `FactAction`).
      return { ...card, status: "undecided", defer_reason: reason ?? card.defer_reason };
    case "undrop":
    case "reopen":
      return { ...card, status: "undecided", defer_reason: null };
  }
}

/** Replace one card in the pool, by id, leaving the order untouched. */
function withCard(state: QueueState, graphNodeId: string, next: ScenarioCard): ScenarioCard[] {
  return state.cards.map((card) => (card.graph_node_id === graphNodeId ? next : card));
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
      return { state: { ...state, cards: event.cards, index, notice: null }, effect: NONE };
    }

    case "focus":
      return { state: { ...state, index: event.index, notice: null }, effect: NONE };

    case "select": {
      const index = state.cards.findIndex((c) => c.graph_node_id === event.graphNodeId);
      // A click on a card the pool no longer holds leaves the selection alone
      // rather than jumping to card 0: the click came from a rendered row, so
      // this can only be a reload landing mid-click, and moving the selection
      // somewhere the human did not point is worse than ignoring one click.
      return index === -1
        ? { state, effect: NONE }
        : { state: { ...state, index, notice: null }, effect: NONE };
    }

    case "visible": {
      const next = { ...state, visibleIds: event.ids, notice: null };
      const selected = state.cards[state.index];
      // A filter that hides the selected card moves the selection to the top of
      // what IS visible. Leaving it where it was would aim the keyboard at a card
      // nobody can see — the exact defect ruling R7 guards against elsewhere.
      if (selected && event.ids.includes(selected.graph_node_id)) {
        return { state: next, effect: NONE };
      }
      const order = visibleOrder(next);
      return {
        state: { ...next, index: order.length > 0 ? order[0] : next.index },
        effect: NONE,
      };
    }

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

  // Every key that is not the I/E refusal clears a notice left by a previous one:
  // a message about the LAST card, still on screen beside a different one, would
  // be a lie the human has no way to date.
  const cleared = state.notice === null ? state : { ...state, notice: null };

  switch (key.toLowerCase()) {
    // Item 3: browsing costs nothing. Arrows for the mouse-adjacent hand, j/k for
    // the touch typist — the same two conventions the study's tools ship, and
    // neither of them writes anything.
    case "arrowdown":
    case "j":
      return moveSelection(cleared, 1);

    case "arrowup":
    case "k":
      return moveSelection(cleared, -1);

    case "i":
    case "e": {
      // An unrulable card refuses in the UI rather than making a round trip that
      // returns 400 — and SAYS why, in the backend's own words. Refusing silently
      // was 1.7C's behaviour and Roman's defect: the key did nothing and the
      // screen did not react, which is indistinguishable from a dead keyboard.
      if (card.defer_required) {
        return { state: { ...cleared, notice: card.defer_required_reason }, effect: NONE };
      }
      const action: FactAction = key.toLowerCase() === "i" ? "include" : "drop";
      return rule(cleared, card.graph_node_id, action);
    }

    case "d": {
      // The short-circuit: a defer-required card already carries a server-composed
      // reason, so one press accepts it. Prompting would ask the human to
      // re-type a sentence the system already wrote.
      if (card.defer_required && card.defer_required_reason) {
        return rule(cleared, card.graph_node_id, "defer", card.defer_required_reason);
      }
      return { state: { ...cleared, mode: { kind: "deferring", draft: "" } }, effect: NONE };
    }

    case "u":
      return undo(cleared);

    default:
      return { state: cleared, effect: NONE };
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

/** Record a ruling, patch the card, advance, and ask the caller to send it. */
function rule(
  state: QueueState,
  graphNodeId: string,
  action: FactAction,
  reason?: string,
): QueueResult {
  const target = state.cards.find((c) => c.graph_node_id === graphNodeId);
  const cards = target
    ? withCard(state, graphNodeId, applyRulingToCard(target, action, reason))
    : state.cards;

  const next = advance({
    ...state,
    cards,
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
  const target = state.cards.find((c) => c.graph_node_id === last.graphNodeId);
  // The card's own state goes back with the selection — otherwise the chip would
  // still read "Included" on a card the human has just taken back.
  const cards = target
    ? withCard(state, last.graphNodeId, applyRulingToCard(target, "reopen", undefined))
    : state.cards;
  return {
    state: {
      ...state,
      cards,
      index: last.index,
      lastRuling: null,
      ruled: state.ruled.filter((id) => id !== last.graphNodeId),
    },
    effect: { kind: "rule", graphNodeId: last.graphNodeId, action: "reopen" },
  };
}

/**
 * The running count the section above shows: how much of the pool is dealt with.
 *
 * ## Why it counts the POOL's state and not just this session's keystrokes
 *
 * Until 1.7E this returned `state.ruled.length` — the rulings made since the page
 * loaded. That was invisible then, because the queue only ever showed unruled
 * cards. The list shows every card with its state chip, so a bar reading "0 of
 * 148 ruled" beside twelve green chips is two surfaces of the same screen
 * disagreeing about the same fact (§9). A card counts once however it came to be
 * ruled — the set union is what guarantees that.
 *
 * `total` stays the WHOLE pool, never the filtered view: the section summary is a
 * statement about the work, not about what is on screen right now.
 */
export function progress(state: QueueState): { ruled: number; total: number } {
  const dealt = new Set(state.ruled);
  for (const card of state.cards) {
    if (candidateState(card) !== "not_ruled") dealt.add(card.graph_node_id);
  }
  return { ruled: dealt.size, total: state.cards.length };
}
