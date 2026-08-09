// =============================================================================
// useCardsPayload — the queue's ONE read of the card payload
// =============================================================================
//
// Extracted from `CardQueue` when ONE_CARD_GRAMMAR pushed that file past the
// 300-line limit (Rule 17, and ruling R8). The seam is the one that file's own
// header already claimed: it "fetches, wires, and renders", and this is the
// FETCH — the request, the four pieces of payload state it lands in, and the
// error that must never look like an empty queue.
//
// What stays behind is the keyboard boundary, the filters and the JSX.

import { useCallback, useState } from "react";

import {
  fetchScenarioCards,
  type ProposalSource,
  type ScenarioCard,
  type ScenarioCardsResponse,
} from "../services/scenarioCards";

/** Everything one read of the payload produces. */
export type CardsPayload = {
  loading: boolean;
  /** Named, contextual, and NEVER a silently empty queue (Standing Rule 1). */
  error: string | null;
  /** The stored sentence a target-less scenario shows INSTEAD of a queue. */
  noTargetNotice: string | null;
  /** Which completed run is proposing the cards, or `null`. */
  proposalSource: ProposalSource | null;
};

/**
 * Read one scenario's cards, and hold what the read produced.
 *
 * @param onCards called with the WHOLE pool — both lists concatenated — so the
 *        caller's reducer owns the cards and this hook owns only the request.
 *
 * ## Why both lists arrive as one
 *
 * The backend partitions DROPPED candidates into `set_aside` (they are out of
 * the working queue), but the chip row has an "Excluded (n)" facet and it has to
 * count something real — and "Full pool" must not shrink every time Roman
 * excludes a card, or the denominator would move under the counts (§9).
 *
 * Concatenated rather than merged: each list arrives sorted by C-ordinal, and
 * re-sorting the union here would be the browser re-deriving an order the
 * backend owns and warns about (`sort_by_code`).
 */
export function useCardsPayload(
  slug: string,
  scenarioId: string,
  onCards: (cards: ScenarioCard[]) => void,
): CardsPayload & { load: () => Promise<void> } {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [noTargetNotice, setNoTargetNotice] = useState<string | null>(null);
  const [proposalSource, setProposalSource] = useState<ProposalSource | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const cards: ScenarioCardsResponse = await fetchScenarioCards(slug, scenarioId);
      onCards([...cards.pool, ...cards.set_aside]);
      // `cards.link_progress` is deliberately not read (Piece 1d): the stuck
      // pile's count left the frame for the cards themselves, where a locked
      // card states its own condition and offers the type-ahead. The payload
      // still carries it — other surfaces may want it — and this queue does not.
      //
      // Present only when the scenario names nobody, in which case both lists
      // above were empty. Held rather than derived: an empty queue is not by
      // itself this state.
      setNoTargetNotice(cards.no_target_notice ?? null);
      setProposalSource(cards.proposal_source ?? null);
      setError(null);
    } catch (e: unknown) {
      // Name WHAT failed, WHERE, and WHY. The scenario is in scope here and the
      // human may have several open, so a bare "failed to load" leaves them
      // guessing which one broke — and a non-`Error` throw used to lose the
      // cause entirely (Standing Rule 1: a failure a reader cannot diagnose is
      // incomplete error handling).
      const cause = e instanceof Error ? e.message : String(e);
      setError(`Could not load the candidates for scenario ${scenarioId} (${slug}): ${cause}`);
    } finally {
      setLoading(false);
    }
  }, [slug, scenarioId, onCards]);

  return { loading, error, noTargetNotice, proposalSource, load };
}
