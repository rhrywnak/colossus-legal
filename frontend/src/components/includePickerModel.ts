// =============================================================================
// includePickerModel — Include must ask before it files
// =============================================================================
//
// Clicking Include used to POST `{"action":"include"}` and get a 400 back, every
// time, on every card. The backend has required the accusation and the stance
// since FACT_CARD_v2 §2 — including a fact writes the Proof Matrix link, and a
// link needs an object and a direction (ruling R32) — and the browser sent
// neither. Roman met it on PROD S-13.
//
// This module is the two questions, as a PURE state machine rather than a
// `useState` inside the row, for the reason `scanConfirmModel` gives: the
// property worth proving here is a NEGATIVE — "nothing but Save sends anything"
// — and a negative over a component is only as good as the list of interactions
// somebody remembered to simulate. Over a closed union it is exhaustive by
// construction, and there is no RTL or jsdom in this tree to simulate with
// (CLAUDE.md rule 30).
//
// ## Domain note: two vocabularies, mapped in ONE place
//
// A curator answers "does this help us or help them?". The graph stores
// `supports` / `rebuts` — 802 and 142 edges measured, and the word Job B wrote
// into the 59 drafted cards on disk. The human half of that mapping is two
// settings rows; the wire half is the two literals in `STANCES` below, which are
// the graph's own tokens and are never renamed.

import type { AllegationOption, CardGrammarWording } from "../services/evidenceLinks";
import type { CardBearsOn, CardFactStance } from "../services/scenarioCards";

/**
 * One row of the accusation select.
 *
 * `label` is what a human reads; `allegationId` is what goes on the wire. Both
 * are carried so the browser never has to find its way from one to the other —
 * matching a rendered label back to an id by string equality is a join that
 * breaks silently the day somebody re-words a label, and re-wording is what a
 * settings store is FOR.
 */
export interface IncludeOption {
  allegationId: string;
  label: string;
}

/**
 * An OPEN row: the card it is filing, and the two answers so far.
 *
 * ## Why the picker carries its TARGET (the 1.7G law, again)
 *
 * The defer prompt learned this the hard way: a prompt that commits to
 * `state.cards[state.index]` files against whatever is selected when Save is
 * pressed, not against the card the human opened it on. Anything that moves the
 * selection mid-answer — a filter, a J press, a reload — would include the wrong
 * card. The id is captured when the row opens and is the only card this state
 * can ever commit to.
 *
 * Exported as its own type so `QueueMode.including` can BE these fields rather
 * than hold a copy of them. A mode carrying a whole `IncludePickerState` could
 * represent "the row is open and also closed", and a state that cannot be
 * written is better than one that is merely never written.
 */
export interface IncludePicking {
  graphNodeId: string;
  /** `null` on a card the extraction linked to nothing (item 4). */
  allegationId: string | null;
  stance: CardFactStance;
}

/** Where the picker is. */
export type IncludePickerState = { phase: "closed" } | ({ phase: "picking" } & IncludePicking);

/**
 * Everything a human can do to the row.
 *
 * `cancel` and `close` are both here and they are not duplicates: `cancel` is the
 * human pressing Cancel or Escape, and `close` is the queue taking the row away
 * (the card was ruled by another device, the pool reloaded). They do the same
 * thing today. They are named separately so the exhaustive test below has to
 * name both, and so a future change that makes one of them different has a place
 * to put that difference.
 */
export type IncludePickerAction =
  | { type: "open"; graphNodeId: string; options: IncludeOption[] }
  | { type: "chooseAllegation"; allegationId: string }
  | { type: "chooseStance"; stance: CardFactStance }
  | { type: "save" }
  | { type: "cancel" }
  | { type: "close" };

/** What the caller must send — `null` for "send nothing". */
export interface IncludeCommit {
  graphNodeId: string;
  allegationId: string;
  stance: CardFactStance;
}

/** The new state, and the include to file. */
export interface IncludePickerStep {
  state: IncludePickerState;
  commit: IncludeCommit | null;
}

/** The wire tokens, and the order the control renders them in. */
export const STANCES: readonly CardFactStance[] = ["supports", "rebuts"] as const;

/** Nothing open, nothing sent. */
export const closedPicker: IncludePickerState = { phase: "closed" };

/**
 * The stance a card opens on.
 *
 * `supports` because a curator including a fact is, overwhelmingly, saying it
 * helps — the measured split is 802 to 142. The other state is one click away and
 * the control says which one is chosen, so the default costs nothing to correct
 * and saves the common case a decision.
 *
 * ## ⚑ RULED 2026-09-17: this stays compiled in FOR NOW, deliberately
 *
 * The architecture gate called it a Standing-Rule-2 value that belongs in the
 * settings store, and it is right: 802:142 is a property of the evidence
 * gathered so far, not a logical invariant, and another case could reasonably
 * open on "Helps them".
 *
 * It is not a settings row yet because of where it is APPLIED. `cardTriage`'s
 * reducer opens the picker (`openPicker`), and that reducer has no access to the
 * settings snapshot — so reading a stored default means threading it through
 * `QueueState` or onto the queue's events, which is the same seam the
 * `cardTriage` split (`cardPrompts.ts`) is about. Roman ruled that the row
 * `card_include_picker_default_stance` ships WITH that split, where the
 * threading belongs, rather than being bolted on here first.
 *
 * So: no `// STRUCTURAL:` marker, because the structural argument is the weak
 * one. This is a deferral with a date and an owner, and this block is the record
 * of it — so the next reader, and the next gate, do not re-litigate it.
 */
const DEFAULT_STANCE: CardFactStance = "supports";

/**
 * Advance the picker.
 *
 * ## Rust Learning: this is the reducer shape, in TypeScript
 *
 * (state, action) → new state + a DESCRIBED effect, where the effect is returned
 * as data and performed by the caller. `cardTriage` does the sending; this
 * decides whether anything may be sent, and a test can enumerate every action
 * without a browser, a DOM or a click.
 */
export function includePickerStep(
  state: IncludePickerState,
  action: IncludePickerAction,
): IncludePickerStep {
  switch (action.type) {
    case "open":
      // The first option is the default — the card's own first bears-on, which is
      // what the human is looking at. A card with NO options opens with nothing
      // chosen rather than nothing at all: the row still appears, says what it
      // wants, and refuses to save until it has it (instruction item 4).
      return {
        state: {
          phase: "picking",
          graphNodeId: action.graphNodeId,
          allegationId: action.options[0]?.allegationId ?? null,
          stance: DEFAULT_STANCE,
        },
        commit: null,
      };

    case "chooseAllegation":
      // Re-aims an OPEN row. It never opens one — a select cannot be changed
      // while it is not on screen, but "cannot" is the kind of claim that
      // survives exactly until the next refactor moves the control.
      return state.phase === "picking"
        ? { state: { ...state, allegationId: action.allegationId }, commit: null }
        : { state, commit: null };

    case "chooseStance":
      return state.phase === "picking"
        ? { state: { ...state, stance: action.stance }, commit: null }
        : { state, commit: null };

    case "save":
      // The ONE arm that sends. Guarded on BOTH the phase and the accusation:
      // a save with nothing chosen is a doomed round trip that would come back
      // as the very 400 this whole task exists to stop, so the row keeps itself
      // open and the control stays disabled instead.
      return state.phase === "picking" && state.allegationId !== null
        ? {
            state: { phase: "closed" },
            commit: {
              graphNodeId: state.graphNodeId,
              allegationId: state.allegationId,
              stance: state.stance,
            },
          }
        : { state, commit: null };

    case "cancel":
    case "close":
      // Cancel sends NOTHING. That is a chosen absence, not a failure: the card
      // keeps whatever it already carried, and nothing on screen claims an
      // include happened.
      return { state: { phase: "closed" }, commit: null };
  }
}

/** Can Save do anything from here? The control's own disabled state. */
export function canSave(state: IncludePickerState): boolean {
  return state.phase === "picking" && state.allegationId !== null;
}

/**
 * The accusations this card may be filed under: its OWN first, then the
 * scenario's, deduped by id.
 *
 * ## Domain note: why `others` is not in this list (ruled, §12 Q6)
 *
 * `AllegationOptions` serves two lists. `serving` is what this scenario already
 * argues; `others` is the rest of the complaint — 120 rows on this case. A
 * 120-row dropdown is not a picker, it is a search problem wearing a select, and
 * a card that genuinely belongs under an unrelated paragraph still has the link
 * type-ahead that exists for exactly that. So: the card's own bears-on first,
 * because that is what the human is reading, then the scenario's own.
 *
 * ## Why dedupe on the id and not the label
 *
 * The same reason the id is on `CardBearsOn` at all. Two accusations can render
 * the same sentence, and two rows a human cannot tell apart are worse than one.
 */
export function includeOptions(
  bearsOn: readonly CardBearsOn[],
  serving: readonly AllegationOption[],
): IncludeOption[] {
  const out: IncludeOption[] = [];
  const seen = new Set<string>();

  for (const entry of bearsOn) {
    if (seen.has(entry.allegation_id)) continue;
    seen.add(entry.allegation_id);
    out.push({ allegationId: entry.allegation_id, label: entry.accusation });
  }
  for (const option of serving) {
    if (seen.has(option.allegation_id)) continue;
    seen.add(option.allegation_id);
    out.push({ allegationId: option.allegation_id, label: option.label });
  }
  return out;
}

/** The two stance buttons, labelled from the store. */
export function stanceLabels(
  wording: CardGrammarWording,
): { stance: CardFactStance; label: string }[] {
  return [
    { stance: "supports", label: wording.include_picker_helps_us_label },
    { stance: "rebuts", label: wording.include_picker_helps_them_label },
  ];
}
