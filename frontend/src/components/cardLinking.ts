// =============================================================================
// cardLinking.ts — the link panel's rules, as pure functions (task 2.10)
// =============================================================================
//
// What a human ticks, what may be saved, which accusations the filter leaves, and
// where the highlight lands after "Save and next". None of it touches React or
// the network, which is what makes the acceptance test — link the LAST stuck card
// with a different one selected — a real test rather than a hope (CLAUDE.md rule
// 30: there is no component-test infrastructure here).
//
// ## Why this is its own file and not part of `cardTriage`
//
// `cardTriage.ts` is already at 323 non-comment lines against a 300-line limit
// (Rule 17) — a pre-existing overage this task must not make worse. The seam is
// real anyway: that file is WHAT A KEY DOES, this one is WHAT THE LINK CONTROL
// DOES, and the reducer's link branch is four lines that delegate here.
//
// ## The frontend composes NOTHING
//
// Every string a human reads comes from `LinkPanelWording`, which the backend
// reads out of the settings store. There is not one user-facing literal in this
// file, and the refusals below return the STORED sentence rather than a message
// written here — so the browser's pre-check and the server's 400 cannot drift
// into telling a person two different things about one mistake (Roman's R4).

import type { AllegationOption, LinkCut, LinkPanelWording } from "../services/evidenceLinks";
// Type-only import: erased at compile time, so this does NOT create a runtime
// cycle with `cardTriage`, which imports the two handlers below as values. The
// vocabulary (what a queue event and effect ARE) belongs to the reducer; what a
// link DOES belongs here.
import type { QueueEffect, QueueResult, QueueState } from "./cardTriage";
import type { ScenarioCard } from "../services/scenarioCards";

/** What a human has ticked and chosen, before they save it. */
export type LinkDraft = {
  /** The accusations ticked, in the order they were ticked. That order reaches
   *  the card's sentence, so it is not a set. */
  allegationIds: string[];
  /** `null` until they say which way it cuts — which is required. */
  cut: LinkCut | null;
  /** Whether the full complaint list is open. */
  showAll: boolean;
  /** What they have typed into the filter box. */
  filter: string;
};

export const EMPTY_DRAFT: LinkDraft = {
  allegationIds: [],
  cut: null,
  showAll: false,
  filter: "",
};

/**
 * Tick or untick one accusation.
 *
 * Appends rather than inserting, so the list keeps the order the human worked in
 * — which is the order the card's sentence lists them in.
 */
export function toggleAllegation(draft: LinkDraft, allegationId: string): LinkDraft {
  const ticked = draft.allegationIds.includes(allegationId);
  return {
    ...draft,
    allegationIds: ticked
      ? draft.allegationIds.filter((id) => id !== allegationId)
      : [...draft.allegationIds, allegationId],
  };
}

/**
 * Why this draft cannot be saved yet, in the STORED words, or `null` if it can.
 *
 * ## Why the browser checks at all, when the backend also refuses
 *
 * The same reason 1.7E stopped I and E making a doomed round trip on a defer-only
 * card: a human who clicks Save and waits for a 400 has been made to wait to be
 * told something the screen already knew. Both checks exist, both use the same
 * stored sentence, and the backend's is the one that actually guards the table.
 *
 * The cut is checked SECOND on purpose. A human who has ticked nothing and chosen
 * nothing is told to tick something first, because that is the step they are on.
 */
export function refusalFor(draft: LinkDraft, wording: LinkPanelWording): string | null {
  if (draft.allegationIds.length === 0) return wording.missing_allegation_refusal;
  if (draft.cut === null) return wording.missing_cut_refusal;
  return null;
}

/** Whether this draft is ready to send. */
export function canSave(draft: LinkDraft): boolean {
  return draft.allegationIds.length > 0 && draft.cut !== null;
}

/**
 * The accusations to show, given the draft's state.
 *
 * Short list until "Show all" is pressed; then the whole complaint, narrowed by
 * whatever has been typed. The filter matches the backend-supplied `filter_text`
 * and nothing else — deciding what is searchable is a statement about what a
 * human means when they type "fiduciary", and that decision is already made
 * server-side.
 *
 * ## Why a ticked accusation is always shown
 *
 * Filtering it out of view while it stays in the draft would let a human save a
 * link to something they can no longer see — and then wonder where the third chip
 * came from. Anything ticked survives the filter.
 */
export function visibleOptions(
  draft: LinkDraft,
  serving: AllegationOption[],
  others: AllegationOption[],
): AllegationOption[] {
  const pool = draft.showAll ? [...serving, ...others] : serving;
  const needle = draft.filter.trim().toLowerCase();
  if (!needle) return pool;
  return pool.filter(
    (option) =>
      option.filter_text.includes(needle) || draft.allegationIds.includes(option.allegation_id),
  );
}

/**
 * Whether a card needs the link control at all.
 *
 * Exactly the cards the extraction never linked AND nobody has linked since:
 * `defer_required` is the backend's own flag, and `defer_required_reason` names
 * which of the two unrulable classes it is. A card with no QUOTE is defer-only
 * too and linking cannot help it — no accusation gives a statement words — so the
 * panel would be a control that cannot do anything, offered beside a refusal
 * saying so.
 *
 * ## Why this asks about `stance` rather than parsing the reason
 *
 * Reading which class a card is in out of its English sentence would break the
 * day the wording changed — and the wording is now editable from the Settings
 * page, so that day is any day. `stance === null` is the machine-readable form of
 * "the extraction linked this to nothing", which is the condition linking cures.
 */
export function needsLinking(card: ScenarioCard): boolean {
  return card.stance === null && card.quote.text.trim().length > 0;
}

/**
 * Whether a card is still waiting for a human to link it.
 *
 * The stuck pile, minus what has been cleared. Drives which card "Save and next"
 * lands on.
 */
export function stillStuck(card: ScenarioCard): boolean {
  return needsLinking(card) && card.human_links.length === 0;
}

/**
 * The next card needing a link AFTER the one just saved, in the visible order.
 *
 * ## Ruling R1 and R2 together, in one function
 *
 * The anchor is the card that was LINKED — never the selection. Link the fortieth
 * card while the third is selected and the highlight lands on the forty-first
 * stuck card, which is where the work is. That is 1.7G's ruling R2 applied to a
 * second control, and it is why this takes an id rather than an index.
 *
 * Returns `null` when nothing after it is stuck, which leaves the selection where
 * it was: stopping is how "you have cleared the rest of this list" becomes a
 * visible state, exactly as the ruling queue's `advance` stops rather than
 * wrapping.
 *
 * A card the list no longer holds yields the first stuck card in view — the same
 * honest recovery `advance` makes when a filter moved under a ruling.
 */
export function nextStuckAfter(cards: ScenarioCard[], linkedId: string): string | null {
  const at = cards.findIndex((card) => card.graph_node_id === linkedId);
  const from = at === -1 ? -1 : at;

  for (let i = from + 1; i < cards.length; i += 1) {
    const card = cards[i];
    // The card just linked is not stuck any more, but the payload in hand still
    // says it is — the pool has not been re-read yet. Skipping by ID rather than
    // by state is what stops "Save and next" landing back on the card just saved.
    if (card.graph_node_id !== linkedId && stillStuck(card)) {
      return card.graph_node_id;
    }
  }
  return null;
}

// ─── The reducer's link branches ────────────────────────────────────────────

/**
 * A link saved on ONE named card (task 2.10, ruling R1).
 *
 * ## No optimistic state, deliberately — unlike a ruling
 *
 * A ruling patches its card here so the state chip changes under the human's
 * hand. A link cannot: what a linked card SAYS is a sentence the backend composes
 * from stored wording and the accusation labels, and building it here would be
 * the browser inventing vocabulary (the language law) — the very thing ruling R2
 * forbids most sharply on this card. So the effect fires, the caller re-reads the
 * pool, and the summary, the chips, the unlocked Include button and the progress
 * line all arrive together from one authoritative read.
 *
 * ## Where the highlight goes
 *
 * Only when the human asked for it. "Save and next" is a convenience, never the
 * only path (ruling R1), so a plain save leaves the selection exactly where they
 * put it — and the advance, when they do ask, is measured from the card that was
 * LINKED, never from the selection (ruling R2).
 */
export function linkOnCard(
  state: QueueState,
  event: { graphNodeId: string; allegationIds: string[]; cut: LinkCut; advance: boolean },
  /** The cards the active filter leaves, in display order — where "next" is
   *  measured. Passed in rather than derived here: which cards are VISIBLE is the
   *  queue's business, and this module knows only which of them are stuck. */
  visible: ScenarioCard[],
): QueueResult {
  const card = state.cards.find((c) => c.graph_node_id === event.graphNodeId);
  if (!card) return { state, effect: { kind: "none" } };

  const effect: QueueEffect = {
    kind: "link",
    graphNodeId: event.graphNodeId,
    allegationIds: event.allegationIds,
    cut: event.cut,
  };

  // An open defer prompt is abandoned, as it is for a ruling on a named card: a
  // click on this card's own Save is an unambiguous statement about this card.
  const cleared: QueueState = { ...state, mode: { kind: "triage" }, notice: null };

  if (!event.advance) return { state: cleared, effect };

  const nextId = nextStuckAfter(visible, event.graphNodeId);
  if (nextId === null) return { state: cleared, effect };

  const index = cleared.cards.findIndex((c) => c.graph_node_id === nextId);
  return { state: index === -1 ? cleared : { ...cleared, index }, effect };
}

/**
 * One link taken back, on the card it is printed on (task 2.10).
 *
 * A card the pool no longer holds is ignored, for the same reason `select` and
 * `rule` ignore one: the click came from a rendered row, so it can only be a
 * reload landing mid-click, and unlinking a card the human can no longer see is
 * worse than dropping one click.
 *
 * Nothing is patched optimistically here either — the card keeps its chips until
 * the re-read removes them, because what a card shows once a link is gone is a
 * server-composed decision (it returns to defer-only, with the sentence that
 * says so).
 */
export function unlinkOnCard(
  state: QueueState,
  graphNodeId: string,
  allegationId: string,
): QueueResult {
  const card = state.cards.find((c) => c.graph_node_id === graphNodeId);
  if (!card) return { state, effect: { kind: "none" } };
  return {
    state: { ...state, notice: null },
    effect: { kind: "unlink", graphNodeId, allegationId },
  };
}
