// =============================================================================
// rulingAcknowledgment.ts — every ruling says what it did (2026-08-08)
// =============================================================================
//
// ## The measured defect this module exists for
//
// On beta.385 the architect pressed Defer on a locked card in S-4 and reported
// the feature dead: no dialog, no state change, no error, nothing at all. The
// database said otherwise. The ruling landed at 20:51:04 — a ruling anchor, a
// reference row carrying `status = undecided` and the server-composed
// `defer_reason`, and `source_run_id` populated. It was the most recent ruling
// of any kind on DEV, four minutes after the last include.
//
// Defer was never broken. It was SILENT, and worse than silent: ruling the card
// made it human-touched, so precedence stopped the projection proposing it, and
// the card correctly left the Proposed filter and vanished from the list. A
// correct write plus a correct filter, with nothing said in between, is
// indistinguishable from a dead button — and that is how it was reported.
//
// So the rule here is not "report errors". It is: EVERY ruling acknowledges
// itself, in success as well as failure, and a card that leaves the list because
// of a ruling says so as it goes.
//
// ## Why this is a pure module and not three `.replace()` calls in the queue
//
// CLAUDE.md rule 30 records that component-test infrastructure is deliberately
// not set up, so a sentence composed inside a React tree is a sentence nothing
// can assert — which is exactly how a silent ruling path survived a suite of 102
// passing reducer tests. The words are the behaviour here, so the words are
// testable.
//
// Every string comes from the settings store. Nothing here invents a sentence.

import type { LinkPanelWording } from "../services/evidenceLinks";
import type { ScenarioCard } from "../services/scenarioCards";
import type { CandidateState } from "./candidateFilters";
import type { RulingOutcome } from "./useQueueReducer";

/** What the queue shows about the ruling that just happened. */
export type RulingReceipt = {
  /** The card it is about, so it can be rendered ON that card when it is still
   *  on screen — and reported in the queue's own strip when it is not. */
  graphNodeId: string;
  /** The composed sentence(s), already filled. Rendered verbatim. */
  text: string;
  /** `true` when the ruling was refused — the one case that is an alert. */
  failed: boolean;
};

/** Everything the composition needs, gathered by the caller. */
export type AcknowledgmentInput = {
  outcome: RulingOutcome;
  /** The card as the queue now holds it, or `undefined` if the pool no longer
   *  has it (a reload landing mid-ruling). */
  card: ScenarioCard | undefined;
  state: CandidateState | null;
  /** The list's own word for that state — "Included", "Deferred". */
  stateLabel: string | null;
  /** Whether the ruling took the card out of the list the human is looking at. */
  leftTheList: boolean;
  /**
   * The active filter's own label, so the sentence names the list correctly.
   *
   * `null` until the stored words load — the five filter names became rows in
   * ONE_CARD_GRAMMAR (ruling R6), so this side can no longer be sure of one. The
   * whole acknowledgment is then withheld rather than composed around a gap: a
   * sentence reading "C-73 has left the  list" is worse than the silence, and
   * the receipt has always been all-or-nothing on its wording (see below).
   */
  filterLabel: string | null;
  /** `null` until the panel wording loads. No sentence is invented without it. */
  wording: LinkPanelWording | null;
};

/**
 * The sentence a ruling leaves behind, or `null` when there is nothing to say.
 *
 * `null` happens in exactly one situation worth naming: the wording has not
 * loaded. There is deliberately no compiled-in fallback (the configuration law,
 * ruling R4) — a receipt that invented its own words would be the one sentence on
 * this screen the store cannot reach.
 */
export function rulingAcknowledgment(input: AcknowledgmentInput): RulingReceipt | null {
  const { outcome, wording } = input;
  if (!wording) return null;

  const code = input.card?.code ?? UNNUMBERED;

  if (outcome.failure !== null) {
    return {
      graphNodeId: outcome.graphNodeId,
      text: fill(wording.card_ruling_failed_template, {
        code,
        detail: outcome.failure,
      }),
      failed: true,
    };
  }

  return {
    graphNodeId: outcome.graphNodeId,
    text: [saidAboutTheRuling(input, code), saidAboutTheList(input, code)]
      .filter((part) => part.length > 0)
      .join(" "),
    failed: false,
  };
}

/**
 * What the ruling itself did.
 *
 * A DEFER on a locked card is the one that says something extra, and it is the
 * whole reason the architect could not tell a working feature from a dead one: a
 * locked card carries the system's own reason, so Defer commits in ONE press with
 * no prompt (prompting would ask the human to retype a sentence the server
 * wrote). They should still be able to read the sentence they just signed.
 */
function saidAboutTheRuling(input: AcknowledgmentInput, code: string): string {
  const { outcome, wording, stateLabel } = input;
  if (!wording) return "";

  if (outcome.action === "defer" && outcome.reason) {
    return fill(wording.card_defer_recorded_template, { reason: outcome.reason });
  }
  // Every other verb: name the card and the state it is now in. `stateLabel` is
  // absent only when the pool no longer holds the card, in which case there is no
  // state to report and the list sentence carries the acknowledgment alone.
  if (!stateLabel) return "";
  return fill(wording.card_ruling_saved_template, { code, state: stateLabel });
}

/**
 * …and what it did to the list.
 *
 * THE VANISH. Empty when the card stayed put, because a sentence explaining that
 * nothing moved is noise on a surface whose whole job is to be read at speed.
 */
function saidAboutTheList(input: AcknowledgmentInput, code: string): string {
  if (!input.leftTheList || !input.wording || input.filterLabel === null) return "";
  return fill(input.wording.card_ruling_left_filter_template, {
    code,
    filter: input.filterLabel,
  });
}

// CONST: a control word, not configuration. A card gather has not numbered yet
// has no handle a human can say out loud (§2a), and "undefined has left the
// Proposed list" is worse than a plain noun. Same class as the Include button's
// label — it names no party, document or claim — and it is the fallback for a
// transient edge case (a reload landing mid-ruling), not a tunable. Every string
// in this module that a human READS as a sentence comes from the settings store;
// this is the one word that stands in for a missing identifier.
const UNNUMBERED = "That candidate";

/** Fill `{name}` placeholders from a map. Unknown tokens are left verbatim, as
 *  the backend's own `render` does — a placeholder on screen is a visible fault,
 *  and a silently blanked one is not. */
function fill(template: string, values: Record<string, string>): string {
  return template.replace(/\{(\w+)\}/g, (token, name: string) =>
    name in values ? values[name] : token,
  );
}
