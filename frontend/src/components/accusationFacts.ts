// =============================================================================
// accusationFacts.ts — which facts the accusation section may offer (task 2.11)
// =============================================================================
//
// One pure function, split out of the page so it can be tested: the frontend test
// pattern here is pure helpers plus service tests, and a rule this load-bearing
// should not live inside a component nothing can call.
//
// ## The rule, and why it is a rule rather than a filter
//
// Only INCLUDED facts may be marked as accusation instances or paired as our
// answers. The rehearsal page is built on the evidence a human ruled in, and
// nothing on it may come from anywhere else — marking an undecided candidate
// would put an unreviewed statement into the accusation's count, and pairing one
// would put it in Marie's mouth as something we said.
//
// The backend does not enforce this today, and that is recorded rather than
// hidden: the write routes accept any graph node id, because a scenario's
// membership can change between a page load and a click and refusing there would
// turn a stale page into an error instead of a gap. What stops an unruled
// statement reaching the section is (a) this filter, which decides what can be
// clicked, and (b) the derivation, which drops any judgment whose anchor is not
// included and names it as a gap. A judgment written around the UI therefore
// shows up as a NAMED gap rather than as silent content — which is the honest
// behaviour, and the reason this is a UI rule.

import type { ScenarioCard } from "../services/scenarioCards";
import type { PickableFact } from "./AccusationFactPicker";

/**
 * The scenario's included facts, reduced to what the picker shows.
 *
 * Order is the cards payload's own — the order the facts list already uses — so a
 * human meets them in the picker in the same sequence they read them above.
 */
export function includedPickableFacts(cards: ScenarioCard[]): PickableFact[] {
  return cards
    .filter((card) => card.status === "included")
    .map((card) => ({
      graphNodeId: card.graph_node_id,
      code: card.code,
      // The statement's own words. Not the context flanks and not the summary:
      // what a human is identifying is the sentence they will read out in court.
      text: card.quote.text,
    }));
}
