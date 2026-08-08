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
// ## What 1.7G changed, and why it had to be here (Roman's ruling R1)
//
// Until 1.7G a ruling had exactly one possible target: `state.cards[state.index]`.
// That single "current position" was the defect — every ruling button in a
// 148-card list aimed at whatever the selection happened to be, so which card a
// button ruled depended on click order rather than on the card it was printed on.
// The signed design says it plainly: "There is no hidden 'current position'
// deciding the target."
//
// So a ruling now names its card. Two events reach the same semantics:
//
//   * `{ type: "key" }`   — the KEYBOARD, which acts on the selected card because
//                           the selection is what a keyboard is aimed at.
//   * `{ type: "rule" }`  — a BUTTON, which carries the id of the card it is
//                           printed on, resolved in that card's own render scope.
//
// Both funnel into `rulingOn`, so there is still one set of §7 semantics and one
// place they can change — the thing 1.7D's "one state machine, two input devices"
// was protecting. What was wrong was not the shared machine; it was the shared
// INDEX standing in for a target the caller already knew.
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
import type { LinkCut } from "../services/evidenceLinks";
import type { FactAction } from "../services/scenarioGather";
import { candidateState } from "./candidateFilters";
import { linkOnCard, unlinkOnCard } from "./cardLinking";

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

/**
 * Which ruling a key or a button performs.
 *
 * ## Why the reducer owns this type (moved here in 1.7G)
 *
 * It lived in `RulingButtons.tsx` while the buttons were the only thing that named
 * a ruling. Now the reducer's own event carries one, and a state machine that
 * imports its vocabulary from a component is a state machine that cannot be tested
 * without one. The component imports it from here instead.
 */
export type RulingKey = "i" | "e" | "d" | "u";

/** What the queue is doing right now. */
export type QueueMode =
  | { kind: "triage" }
  /**
   * The defer prompt is open; keys type instead of ruling.
   *
   * ## Why the prompt carries its TARGET (1.7G)
   *
   * It used to commit to `state.cards[state.index]` when Enter was pressed — the
   * same shared-index defect one step later in time. The gap is real: D on the
   * 40th card's button opens the prompt, and anything that moves the selection
   * while the human is typing their reason would silently defer a different card.
   * The prompt now remembers the card it was opened on, so Enter can only ever
   * commit to that one.
   */
  | { kind: "deferring"; draft: string; graphNodeId: string };

/** The last ruling, kept for single-step undo. */
export type LastRuling = {
  graphNodeId: string;
  action: FactAction;
  /** Where the focus was, so undo returns the human to the card they ruled. */
  index: number;
  /**
   * What the scan was proposing about this card BEFORE the ruling, so undo can
   * put it back (architect ruling R2, 2026-08-08).
   *
   * ## Domain note: a card un-ruled is proposed again
   *
   * Ruling a card clears its proposal, because precedence R-a says a reference
   * row always wins and the server will stop projecting it. Undo removes that
   * row — so the projection resumes, and the card the human is looking at should
   * say so without waiting for a refetch. Stashing the ORIGINAL value rather
   * than reconstructing one is what keeps this honest: the browser never invents
   * a proposal, it only restores the one it was served.
   *
   * `undefined` when nothing was proposing the card.
   */
  proposed: ScenarioCard["proposed"];
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
  /**
   * A human's link from one statement to one or more accusations (task 2.10).
   *
   * A sibling of `rule`, not a variant of it: a link is not a ruling — it changes
   * what MAY be ruled. It travels through the same reducer for one reason, and it
   * is the reason that matters: the id comes from the card's own render scope, so
   * the acceptance test (link the LAST stuck card with a different one selected)
   * is a reducer test of exactly the shape 1.7G's is.
   */
  | { kind: "link"; graphNodeId: string; allegationIds: string[]; cut: LinkCut }
  | { kind: "unlink"; graphNodeId: string; allegationId: string }
  | { kind: "none" };

export type QueueEvent =
  | { type: "key"; key: string; typing: boolean }
  /**
   * A ruling BUTTON on one card (1.7G, ruling R1).
   *
   * The id comes from the card's own render scope, so the target is decided by
   * which card the button is printed on and by nothing else. This is the event
   * that retires the shared current index from the button path.
   */
  | { type: "rule"; key: RulingKey; graphNodeId: string }
  | { type: "focus"; index: number }
  /** A click on a card in the list — selection by identity, never by position. */
  | { type: "select"; graphNodeId: string }
  /** The filter bar reporting which cards it leaves visible, in display order. */
  | { type: "visible"; ids: string[] }
  | { type: "defer_draft"; draft: string }
  /**
   * The link control on ONE card, saved (task 2.10, ruling R1).
   *
   * Same law as `rule`: the id comes up from the card's own render scope, so no
   * shared position can decide the target. It never moves the selection — task
   * 2.12 item A retired the advance, so the human stays on the card they were
   * working and rules it in place.
   */
  | { type: "link"; graphNodeId: string; allegationIds: string[]; cut: LinkCut }
  /** One link taken back, on the card it is printed on. */
  | { type: "unlink"; graphNodeId: string; allegationId: string }
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
 * Advance to the next UNRULED visible card after `anchor`, stopping rather than
 * wrapping.
 *
 * ## Why the anchor is a PARAMETER (1.7G, ruling R2)
 *
 * It used to advance from `state.index` — the selection. That was only ever
 * correct because the selection was also the only thing that could be ruled. Now
 * that a button rules its own card, the two come apart: rule the 40th card while
 * the 3rd is selected and advancing from the selection would land on the 4th,
 * which is nowhere near the work the human is doing. Roman's ruling R2: the
 * highlight lands relative to the card that was RULED.
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
function advance(state: QueueState, anchor: number): QueueState {
  const order = visibleOrder(state);
  const here = order.indexOf(anchor);
  // An anchor outside the visible set can only mean a filter changed under the
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
 * Where the highlight goes when the card it was on is no longer visible.
 *
 * ## Ruling R2: never the top of the list
 *
 * The old rescue sent the selection to `order[0]`. That is what threw Roman back
 * to card 1 after every mouse ruling: rule a card, the card leaves the "Rulable
 * now" filter, the filter reports a new visible set, and the rescue "helpfully"
 * re-aimed at the top of the list — 40 cards from where he was working.
 *
 * So it lands on the nearest surviving card AT OR AFTER the position it lost,
 * which is the next piece of work in reading order. When nothing survives after
 * it (the human just ruled the last card), it falls back to the nearest survivor
 * BEFORE it — the new end of the list — because being left at the bottom, where
 * the work was, beats being thrown to the top. An empty view is the only case with
 * nowhere to go, and there the selection simply stays put.
 */
function nearestVisible(order: number[], index: number): number | undefined {
  for (const at of order) {
    if (at >= index) return at;
  }
  return order.length > 0 ? order[order.length - 1] : undefined;
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
 * advance has always worked this way; this extends it to every field that is
 * VISIBLE, which is the property that has to be maintained rather than the list.
 *
 * It stays honest because the failure path already exists: a refused ruling
 * surfaces its sentence AND re-reads the pool (`useQueueReducer`), so what
 * survives on screen is what the database holds.
 *
 * ## `proposed` is cleared, and that was a measured defect (2026-08-08)
 *
 * A ruling makes the card human-touched, so precedence R-a stops the server
 * projecting it — the next payload has no `proposed` at all. Until this cleared
 * it, the queue's own Proposed facet went on counting a card it had just ruled
 * while every server-sourced number moved: measured on DEV beta.385 as a heading
 * reading 25 beside a facet reading 27, reconciling only on reload. The rule this
 * function has to obey is not "patch status" but "patch everything the screen
 * reads", and `proposed` became one of those the day the facet did.
 */
function applyRulingToCard(
  card: ScenarioCard,
  action: FactAction,
  reason: string | undefined,
  /**
   * The proposal to restore — undo's stash (R2). Omitted on a forward ruling,
   * which always clears.
   */
  restoreProposed?: ScenarioCard["proposed"],
): ScenarioCard {
  switch (action) {
    case "include":
      return { ...card, status: "included", defer_reason: null, proposed: undefined };
    case "drop":
      return { ...card, status: "dropped", defer_reason: null, proposed: undefined };
    case "defer":
      // A defer lands in `undecided` WITH a reason — that pair is what
      // distinguishes "parked" from "never looked at" (backend `FactAction`).
      // It is still a human touch, so the proposal goes with the other two.
      return {
        ...card,
        status: "undecided",
        defer_reason: reason ?? card.defer_reason,
        proposed: undefined,
      };
    case "undrop":
    case "reopen":
      // The two un-ruling verbs: the reference row goes, so the projection
      // resumes and the card carries whatever it was proposing before (R2).
      return {
        ...card,
        status: "undecided",
        defer_reason: null,
        proposed: restoreProposed,
      };
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

      // …and it RECONCILES the session's ruled list against the payload.
      //
      // ## Why this is not redundant with the card states (task 2.12)
      //
      // `progress` unions this list with the payload's own states, so a card can
      // be counted as dealt-with by EITHER. That was safe while the queue was the
      // only thing that could un-rule a card: `undo` removes the id itself. Task
      // 2.12 added a second path — Remove on a fact row — which un-rules a card
      // from OUTSIDE the queue and cannot reach this list.
      //
      // Measured on DEV (beta.375): after removing one included card the filters
      // correctly read "Included (46)" and "Not ruled (92)", while the progress
      // line still read "57 of 148 ruled" against an authoritative 56. Two
      // surfaces of one screen disagreeing about the same fact — the defect §9
      // and the 1.7E-a pool-truth ruling exist to prevent.
      //
      // A reload is the moment the server's word arrives, so session memory it
      // contradicts is simply stale. An entry for a card the payload no longer
      // holds is KEPT: that is a pool that shrank, not a ruling that was undone,
      // and dropping it would silently lower the count.
      const ruled = state.ruled.filter((id) => {
        const card = event.cards.find((c) => c.graph_node_id === id);
        return !card || candidateState(card) !== "not_ruled";
      });

      // `lastRuling` travels WITH that list, because `rule` sets the two together
      // and they have never been able to disagree until now. Dropping an id
      // without clearing the undo target would leave U pointing at a ruling the
      // server has already released: pressing it would emit a `reopen` for a card
      // that is not ruled — a write that creates a fact-ref row and a ledger entry
      // for an act that undoes nothing, and snaps the human's focus to wherever
      // that card used to be.
      //
      // An entry KEPT for a card the pool no longer holds keeps its undo target
      // too, for the same reason it keeps its count: the ruling still happened.
      const lastRuling =
        state.lastRuling && !ruled.includes(state.lastRuling.graphNodeId)
          ? null
          : state.lastRuling;

      return {
        state: { ...state, cards: event.cards, index, ruled, lastRuling, notice: null },
        effect: NONE,
      };
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
      // A filter that hides the selected card has to move the selection: leaving
      // it where it was would aim the keyboard at a card nobody can see — the
      // exact defect ruling R7 guards against elsewhere.
      if (selected && event.ids.includes(selected.graph_node_id)) {
        return { state: next, effect: NONE };
      }
      const order = visibleOrder(next);
      const landing = nearestVisible(order, state.index);
      return {
        state: { ...next, index: landing ?? next.index },
        effect: NONE,
      };
    }

    case "rule":
      return handleCardRuling(state, event.key, event.graphNodeId);

    case "link":
      return linkOnCard(state, event);

    case "unlink":
      return unlinkOnCard(state, event.graphNodeId, event.allegationId);

    case "defer_draft":
      // Typing in the prompt changes the draft and NOTHING else — the target it
      // was opened on is carried through untouched.
      return state.mode.kind === "deferring"
        ? {
            state: { ...state, mode: { ...state.mode, draft: event.draft } },
            effect: NONE,
          }
        : { state, effect: NONE };

    case "key":
      return handleKey(state, event.key, event.typing);
  }
}

/**
 * A ruling BUTTON on one named card (task 1.7G, ruling R1).
 *
 * The target is resolved from the EVENT, never from the selection — that is the
 * whole fix. A click on a card the pool no longer holds is ignored for the same
 * reason `select` ignores one: the click came from a rendered row, so it can only
 * be a reload landing mid-click, and ruling a card the human can no longer see is
 * worse than dropping one click.
 */
function handleCardRuling(state: QueueState, key: RulingKey, graphNodeId: string): QueueResult {
  const card = state.cards.find((c) => c.graph_node_id === graphNodeId);
  if (!card) return { state, effect: NONE };

  // An open defer prompt owns the KEYBOARD, but not the mouse: a click on a named
  // card's own button is an unambiguous statement about that card, and swallowing
  // it would be the dead-keyboard defect in mouse form. So the prompt is abandoned
  // and the new ruling applies — except when the click is D on the very card the
  // prompt is already open for, which would throw away a half-typed reason only to
  // reopen the same prompt.
  if (state.mode.kind === "deferring") {
    if (key === "d" && state.mode.graphNodeId === card.graph_node_id) {
      return { state, effect: NONE };
    }
    return rulingOn({ ...state, mode: { kind: "triage" }, notice: null }, card, key);
  }

  // A ruling is also the end of any refusal message left on screen.
  const cleared = state.notice === null ? state : { ...state, notice: null };
  return rulingOn(cleared, card, key);
}

function handleKey(state: QueueState, key: string, typing: boolean): QueueResult {
  const card = state.cards[state.index];

  // The defer prompt owns the keyboard while it is open.
  if (state.mode.kind === "deferring") {
    return handleDeferKey(state, key, state.mode.draft, state.mode.graphNodeId);
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
    case "e":
    case "d":
    case "u":
      // The keyboard's target is the SELECTED card, because the selection is what
      // a keyboard is aimed at. It hands that card to the same semantics a button
      // uses, so the two input devices cannot drift apart.
      return rulingOn(cleared, card, key.toLowerCase() as RulingKey);

    default:
      return { state: cleared, effect: NONE };
  }
}

/**
 * The §7 ruling semantics, for ONE named card.
 *
 * ## Rust/TS learning: one function, two callers, no shared mutable "current"
 *
 * The keyboard resolves its card from the selection and a button resolves its card
 * from its own render scope, but from here down they are identical — so I/E/D/U
 * mean exactly one thing, whichever device produced them. That is 1.7D's "one
 * state machine, two input devices" kept intact while the shared INDEX that used
 * to stand in for the target is gone (1.7G, ruling R1).
 */
function rulingOn(state: QueueState, card: ScenarioCard, key: RulingKey): QueueResult {
  switch (key) {
    case "i":
    case "e": {
      // An unrulable card refuses in the UI rather than making a round trip that
      // returns 400 — and SAYS why, in the backend's own words. Refusing silently
      // was 1.7C's behaviour and Roman's defect: the key did nothing and the
      // screen did not react, which is indistinguishable from a dead keyboard.
      //
      // The refusal also SELECTS the card it is about, so the sentence appears on
      // the card the human just acted on. Via the keyboard that card is already
      // selected and nothing moves; the buttons for this class are rendered
      // disabled, so this is the belt to that braces.
      if (card.defer_required) {
        return {
          state: { ...state, index: indexOf(state, card), notice: card.defer_required_reason },
          effect: NONE,
        };
      }
      const action: FactAction = key === "i" ? "include" : "drop";
      return rule(state, card.graph_node_id, action);
    }

    case "d": {
      // The short-circuit: a defer-required card already carries a server-composed
      // reason, so one press accepts it. Prompting would ask the human to
      // re-type a sentence the system already wrote.
      if (card.defer_required && card.defer_required_reason) {
        return rule(state, card.graph_node_id, "defer", card.defer_required_reason);
      }
      return {
        state: {
          ...state,
          mode: { kind: "deferring", draft: "", graphNodeId: card.graph_node_id },
        },
        effect: NONE,
      };
    }

    case "u":
      // Undo is deliberately NOT card-scoped: it takes back the most recent
      // ruling, whichever card that was. Every card's U button is the same one
      // step back, which is what "single-step undo" has always meant here — a
      // per-card undo stack would let a human unwind a session by leaning on one
      // key, and would also mean two cards' U buttons doing different things.
      return undo(state);
  }
}

/**
 * Where a card sits in the pool.
 *
 * Returns the current index unchanged for a card the pool no longer holds, which
 * cannot happen on the paths that call it (both resolve the card FROM the pool
 * first) but keeps this total rather than returning a `-1` that would silently
 * become a selection of nothing.
 */
function indexOf(state: QueueState, card: ScenarioCard): number {
  const at = state.cards.findIndex((c) => c.graph_node_id === card.graph_node_id);
  return at === -1 ? state.index : at;
}

/**
 * The keyboard while the defer prompt is open.
 *
 * `graphNodeId` is the card the prompt was opened on — carried through from the
 * mode rather than re-read from the selection, so a reason typed about one card
 * can never be committed against another (see `QueueMode`).
 */
function handleDeferKey(
  state: QueueState,
  key: string,
  draft: string,
  graphNodeId: string,
): QueueResult {
  if (key === "Escape") {
    return { state: { ...state, mode: { kind: "triage" } }, effect: NONE };
  }
  if (key === "Enter") {
    const reason = draft.trim();
    // A blank reason is not a defer — the backend refuses it, and refusing here
    // keeps the prompt open with the human's cursor in it rather than bouncing
    // an error back at them.
    if (!reason) return { state, effect: NONE };
    // The card the prompt was OPENED on, never whatever is selected now (1.7G).
    const card = state.cards.find((c) => c.graph_node_id === graphNodeId);
    if (!card) return { state: { ...state, mode: { kind: "triage" } }, effect: NONE };
    const ruled = rule(state, card.graph_node_id, "defer", reason);
    return { state: { ...ruled.state, mode: { kind: "triage" } }, effect: ruled.effect };
  }
  // Digits pick a quick reason without leaving the keyboard.
  const pick = Number.parseInt(key, 10);
  if (!Number.isNaN(pick) && pick >= 1 && pick <= DEFER_QUICK_REASONS.length) {
    return {
      state: {
        ...state,
        mode: { kind: "deferring", draft: DEFER_QUICK_REASONS[pick - 1], graphNodeId },
      },
      effect: NONE,
    };
  }
  return { state, effect: NONE };
}

/**
 * Record a ruling, patch the card, advance, and ask the caller to send it.
 *
 * ## Ruling R2: everything here is anchored on the card that was RULED
 *
 * Both the advance and the undo return-address used to be `state.index` — the
 * selection. With per-card buttons the ruled card and the selection are no longer
 * the same thing, and the selection is the wrong one of the two: it is where the
 * human's attention WAS, while the ruled card is where their hand just went.
 */
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
  const at = state.cards.findIndex((c) => c.graph_node_id === graphNodeId);
  const anchor = at === -1 ? state.index : at;

  const next = advance(
    {
      ...state,
      cards,
      // The selection MOVES TO THE RULED CARD before advancing, and that is
      // load-bearing rather than tidy. `advance` returns its input untouched when
      // there is nothing left to advance to — so without this line, ruling the
      // LAST card in the view left the highlight wherever the human happened to
      // have it, the filter rescue then found that selection still visible and
      // left it alone, and the highlight sat at the top of a list whose bottom
      // card had just been ruled. Caught on DEV by the acceptance click-through,
      // not by the unit tests, which had only ever exercised the rescue with the
      // selection already on the ruled card.
      index: anchor,
      // The proposal is stashed BEFORE the patch clears it, so undo can put back
      // exactly what the server served rather than a reconstruction (R2).
      lastRuling: { graphNodeId, action, index: anchor, proposed: target?.proposed },
      // A card ruled twice counts once — the running count is "how many of the
      // pool have been dealt with", not "how many keys were pressed".
      ruled: state.ruled.includes(graphNodeId) ? state.ruled : [...state.ruled, graphNodeId],
    },
    anchor,
  );
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
    ? withCard(
        state,
        last.graphNodeId,
        // The stash goes back with the status: a card the human un-rules is
        // proposed again if the projection was proposing it (R2), and the next
        // refetch is what confirms it rather than what discovers it.
        applyRulingToCard(target, "reopen", undefined, last.proposed),
      )
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
