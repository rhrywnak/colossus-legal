// =============================================================================
// factsOrderReset.ts — "forget where every fact was placed" (Piece 5b)
// =============================================================================
//
// ⚑ NAMED `factsOrderReset` and not `factsResetOrder`, which is what it does:
// macOS is case-insensitive, so `factsResetOrder.ts` and `FactsResetOrder.tsx`
// (the confirmation dialog beside it) are the SAME PATH to the filesystem and
// TypeScript refuses the pair outright (TS1149). Same trap `headerStripRules`
// documents; the COMPONENT keeps the name a reader greps for.
//
// Split out of `ScenarioFactsSection` when change D pushed that file past the
// 300-line limit (CLAUDE.md rule 17). The seam is a good one independently: this
// is the section's one DESTRUCTIVE act, it has more reasoning attached than any
// other write there, and until now it could not be tested without mounting a
// component (Rule 30 — there is no component-test tier here).
//
// ## Why one control in the header replaced forty-six on the cards
//
// "Clear my order" sat on every placed card to do a thing a human does once, and
// it competed for the footer with Remove — two controls a click apart, one of
// which discards a position and the other of which takes the fact out of the
// scenario. The footer now keeps only Remove.
//
// ## Why it is a loop over the existing per-fact route
//
// There is no bulk endpoint and this deliberately adds none: the ruling and
// order write paths are audited surfaces, and a second writer for "clear them
// all" would be a new path to guard for a control that runs a handful of times.
// The loop is bounded by the number of PLACED facts — a fact nobody dragged has
// nothing to clear — which is a few, not the whole list.
//
// ## Partial failure is REPORTED, never swallowed
//
// Each write can fail independently, so the outcome is counted and the sentence
// names how many landed. A loop that stopped at the first refusal and said
// "reset" would leave the order half-cleared with the screen claiming otherwise
// (Standing Rule 1).

import { clearFactOrder } from "../services/scenarioFactCuration";
import { fillSlots, type CardGrammarWording } from "../services/evidenceLinks";
import type { ScenarioCard } from "../services/scenarioCards";

/**
 * Everything the reset needs from its caller.
 *
 * ## Rust Learning: this is a "context struct", and why it beats eight arguments
 *
 * The same move a Rust API makes when a function's arguments outgrow readability
 * — `fn run(ctx: &Context)` rather than eight positional parameters. Two things
 * come with it that matter here: a caller cannot silently swap two same-typed
 * arguments (`slug` and `scenarioId` are both `String`, and transposing them
 * would write to the wrong scenario and fail with a 404 nobody could explain),
 * and adding a field later does not touch every call site.
 */
export type ResetOrderContext = {
  slug: string;
  scenarioId: string;
  /** The whole pool; only the PLACED cards are written. */
  cards: ScenarioCard[];
  /**
   * The stored words, or `null` if they have not loaded.
   *
   * `null` is a REFUSAL, not a fallback — see the guard below.
   */
  grammar: CardGrammarWording | null;
  /** Show a failure. Called with `null` to clear a previous one. */
  setError: (message: string | null) => void;
  /** Show what the reset actually did. Every action acknowledges itself. */
  setNotice: (message: string) => void;
  /** Re-read the cards, so the screen matches the store. */
  onDone: () => void;
};

/**
 * Clear every stored placement in this scenario.
 *
 * Resolves when every write has settled — successfully or not — and the caller's
 * notice and error have been set. It never rejects: a partial failure is a
 * REPORTED outcome here, not an exception, because the human is the only one who
 * can act on it and they need the count either way.
 *
 * @param ctx see {@link ResetOrderContext}
 */
export async function resetFactOrder(ctx: ResetOrderContext): Promise<void> {
  const { grammar } = ctx;

  // The control is withheld until the words load, so this branch is unreachable
  // through the UI. It is not a silent `return` all the same: a second caller
  // wired up without the wording would otherwise get a confirm dialog, a click,
  // and nothing at all — the exact shape Standing Rule 1 exists to forbid. The
  // refusal SAYS so, in the one place this module can speak without a stored
  // sentence to speak it with.
  if (!grammar) {
    // eslint-disable-next-line no-console -- there is no stored sentence for a
    // state that cannot happen through the UI, and inventing one would be the
    // literal the language law deletes.
    console.warn(
      "Reset order was invoked before the card wording loaded; nothing was written.",
    );
    ctx.setError("The order could not be reset — please reload and try again.");
    return;
  }

  const placed = ctx.cards.filter((card) => card.sort_ordinal != null);
  const results = await Promise.allSettled(
    placed.map((card) => clearFactOrder(ctx.slug, ctx.scenarioId, card.graph_node_id)),
  );

  const cleared = results.filter((r) => r.status === "fulfilled").length;
  const failed = results.length - cleared;

  if (failed > 0) {
    ctx.setError(
      fillSlots(grammar.reset_order_failed_template, { reason: firstReason(results) }),
    );
  } else {
    ctx.setError(null);
  }

  // The count is what ACTUALLY cleared, not what was attempted: a sentence
  // reporting the intention would be the screen agreeing with the click rather
  // than with the database.
  ctx.setNotice(
    fillSlots(grammar.reset_order_done_template, { count: String(cleared) }),
  );
  ctx.onDone();
}

/**
 * The first refusal's own words, or an empty string.
 *
 * One reason and not all of them: forty-six identical "connection lost" lines
 * are less readable than one, and the COUNT above already says how many failed.
 * Empty rather than a stand-in phrase when the rejection carries no message —
 * the template's slot then closes over nothing, which reads as "it failed" and
 * is exactly what is known.
 */
function firstReason(results: PromiseSettledResult<unknown>[]): string {
  const first = results.find((r) => r.status === "rejected");
  if (!first || first.status !== "rejected") return "";
  return first.reason instanceof Error ? first.reason.message : String(first.reason);
}
