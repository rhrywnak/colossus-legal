// =============================================================================
// cardPrompts.ts — the triage queue's two PROMPTS: defer, and the Include picker
// =============================================================================
//
// CC_TASK_CARDTRIAGE_SPLIT_v1. Lifted out of `cardTriage.ts` unchanged, except
// for the one thing the split was for: the Include picker's opening stance
// arrives as an argument now instead of being a constant.
//
// ## What a PROMPT is, and why these two are one module
//
// Everything else in the queue acts immediately: a key rules a card, moves the
// selection, or does nothing. These two open something that stays open and takes
// further input before anything is recorded — a free-text reason, or an
// accusation and a stance. That is the seam: both hold a partial answer in
// `QueueMode`, both can be abandoned with Escape, and both end by handing one
// finished ruling back to the machine that records it.
//
// ## Why the ruling function is passed IN
//
// `stepPicker` and `handleDeferKey` finish by recording a ruling, which
// `cardTriage` owns (`rule`). Importing it here would make the two modules
// depend on each other at RUNTIME — a cycle that works until the day module
// evaluation order changes and one of them is half-built when the other reads
// it. Passed as [`RuleFn`], the dependency runs one way (`cardTriage` →
// `cardPrompts`), the types flow back the other way as `import type`, which
// TypeScript erases, and a test can hand in a fake and watch what the prompt
// asks for.
//
// ## Nothing here composes a sentence
//
// Same law as the module this came from: the only string these machines hold is
// a quick-pick reason the human could have typed themselves, and the stance
// token, which is a vocabulary the backend owns.

import type { ScenarioCard } from "../services/scenarioCards";
import type { CardFactStance } from "../services/scenarioCards";
import type { FactAction } from "../services/scenarioGather";
import type { QueueEffect, QueueMode, QueueResult, QueueState } from "./cardTriage";
import { includePickerStep, type IncludePickerAction } from "./includePickerModel";

/**
 * How a prompt records the ruling it finished with.
 *
 * `cardTriage.rule` in production. A parameter rather than an import for the
 * reason this module's header gives — and it means these two machines can be
 * tested without the reducer that owns rulings.
 */
export type RuleFn = (
  state: QueueState,
  graphNodeId: string,
  action: FactAction,
  reason?: string,
) => QueueResult;

/**
 * The effect that says nothing happened.
 *
 * It lives here rather than in `cardTriage` so the import between the two
 * modules runs ONE WAY. Same value, same meaning; `cardTriage` imports it.
 */
export const NONE: QueueEffect = { kind: "none" };

// STRUCTURAL: the quick-pick defer reasons. These are UI affordances, not
// configuration: each is a shortcut for typing a sentence the human could type
// anyway, and the free-text field beside them accepts anything. Making them
// configurable would add a settings surface whose only effect is which three
// suggestions appear above an unrestricted input — the reason a defer records is
// whatever the human wrote, and that is never constrained to this list.
//
// ⚑ The marker was `// CONST:` until this list moved here on 2026-09-19.
// `// CONST:` does not exempt anything — it only says "this is a constant",
// which is what the rule is about. The argument above is the structural one and
// always was; the marker is now the one that carries it.
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
 * The mode an Include click opens, defaulted from the card's OWN bears-on and
 * from the STORED opening stance.
 *
 * The scenario's other accusations are in the select too (the component builds
 * that list), but the DEFAULT is the card's first bears-on, because that is what
 * the human is reading when they press Include. A card the extraction linked to
 * nothing opens with nothing chosen and Save refused — never defaulted into a
 * scenario-wide list, which would file a fact under an accusation nobody picked.
 *
 * ## ⚑ `defaultStance` is a SETTINGS ROW, and used to be a constant
 *
 * `includePickerModel` held `const DEFAULT_STANCE = "supports"` until
 * 2026-09-19, with a doc block recording that it belonged in the store and was
 * waiting for this split — because the reducer that opens the picker is pure and
 * cannot read a snapshot. It arrives on the cards payload now
 * (`card_include_picker_default_stance`), rides the `cards_loaded` event onto
 * `QueueState`, and is handed to the picker here. No module holds a default of
 * its own any more, which is why the constant was deleted rather than moved.
 */
export function openPicker(card: ScenarioCard, defaultStance: CardFactStance): QueueMode {
  const opened = includePickerStep(
    { phase: "closed" },
    {
      type: "open",
      graphNodeId: card.graph_node_id,
      options: card.bears_on.map((b) => ({ allegationId: b.allegation_id, label: b.accusation })),
      defaultStance,
    },
  ).state;
  // `open` always yields `picking`; the check is the type system's, not a doubt.
  return opened.phase === "picking"
    ? { kind: "including", graphNodeId: opened.graphNodeId, allegationId: opened.allegationId, stance: opened.stance }
    : { kind: "triage" };
}

/**
 * Hand one picker action to the picker's own machine, and turn its commit into
 * this reducer's ruling.
 *
 * ## Why the include goes through `rule()` like every other ruling
 *
 * A saved include IS a ruling — it patches the card, advances the queue, and is
 * undoable — and it must be all three in exactly the way E and D are. The only
 * difference is that its effect carries two more fields. Routing it anywhere
 * else would give the queue a second way to rule a card, which is how the
 * keyboard and the buttons drifted apart the first time (1.7D).
 */
export function stepPicker(
  state: QueueState,
  action: IncludePickerAction,
  rule: RuleFn,
): QueueResult {
  if (state.mode.kind !== "including") return { state, effect: NONE };

  const { state: next, commit } = includePickerStep({ phase: "picking", ...state.mode }, action);
  const mode: QueueMode =
    next.phase === "picking"
      ? { kind: "including", graphNodeId: next.graphNodeId, allegationId: next.allegationId, stance: next.stance }
      : { kind: "triage" };

  if (!commit) return { state: { ...state, mode }, effect: NONE };

  const ruled = rule(state, commit.graphNodeId, "include");
  return {
    state: { ...ruled.state, mode: { kind: "triage" } },
    effect:
      ruled.effect.kind === "rule"
        ? { ...ruled.effect, allegationId: commit.allegationId, stance: commit.stance }
        : ruled.effect,
  };
}

/**
 * The keyboard while the defer prompt is open.
 *
 * `graphNodeId` is the card the prompt was opened on — carried through from the
 * mode rather than re-read from the selection, so a reason typed about one card
 * can never be committed against another (see `QueueMode`).
 */
export function handleDeferKey(
  state: QueueState,
  key: string,
  draft: string,
  graphNodeId: string,
  rule: RuleFn,
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
