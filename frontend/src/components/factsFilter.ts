// =============================================================================
// factsFilter.ts — the Scenario facts list's three filters (v2.1, change D)
// =============================================================================
//
// Ruling R35 collapsed three sections into one: "Scan & candidates", "Scenario
// facts" and "The accusation, and every time they made it" were three stacked
// lists over one pool, and a reader had to hold all three in their head to know
// what the scenario actually contains. There is now ONE list with three filters.
//
// ## Domain note: what the three words mean
//
//   Included    the evidence a human RULED IN. This is what the scenario holds.
//   Candidates  what the latest completed scan is PROPOSING and nobody has ruled
//               on yet. Work waiting, not content. PRESENCE of `card.proposed`
//               is what that means — there is no "proposed" status and no third
//               writer, which is why this is `proposedCount` and not a status
//               comparison a reader would have to decode.
//   All         both, because the question "did I already rule on this?" is asked
//               across the two and answering it used to mean scrolling between
//               two sections.
//
// The distinction is load-bearing and it is the reason this is a filter and not
// a merge: a candidate on screen beside an included fact must never be mistaken
// for something the scenario holds.
//
// ## ⚑ Why hand-written facts are in the LIST but not in the COUNT
//
// The Included list renders the ruled-in evidence AND the facts a human wrote
// themselves — it always has, since 1.7D put C2 and C4 in one table. The pill
// counts only the first, so on S-7 today it reads 15 over a list of 22 rows.
// That is deliberate and it is the mockup's own arithmetic (15 + 34 = 49):
//
//   * "Included" names a RULING. A hand-written fact was never ruled in — §8
//     makes it authored, uncited and never machine-touched — so counting it
//     under that word would say something false about how it got there. The
//     header this pill replaces made the same split out loud: "15 included · 7
//     added by hand", two numbers because they are two kinds of thing.
//   * Every hand-written row says so on its face: a blue spine instead of a
//     green one, no citation, and "Added by roman" where an evidence row states
//     its ruling. So the reader is never counting rows against the pill blind.
//   * `WorkingView`'s own footer reports what the LIST holds ("N shown · M in
//     the background"), which is the number that answers "how long is this".
//
// Measured on DEV 2026-09-07: S-7 has 15 included cards, 7 human facts, 34
// proposed. Reported to Roman with this change — if he wants the pill to count
// rows, it counts rows, and All becomes 56.
//
// ## Why the counts are a value and not JSX
//
// CLAUDE.md Rule 30 — no component-test tier — so a count decided inside the
// pill row is a count no test can reach. And these are exactly the numbers that
// go wrong: the .374 defect put "No candidates gathered yet" over a pool of 148
// by collapsing "not read yet" into zero, and `null` below is what keeps those
// two states apart.

import { includedRows } from "./factsTable";
import { proposedCount } from "./queueRegion";
import type { ScenarioCard } from "../services/scenarioCards";

/** Which of the three lists the section is showing. */
export type FactsFilter = "included" | "candidates" | "all";

/** The three filters in the order the pill row draws them. */
export const FACTS_FILTERS: FactsFilter[] = ["included", "candidates", "all"];

/**
 * What each pill counts.
 *
 * `null` on every field means the pool has not been READ yet — which is a
 * different fact from a pool of zero, and the one the pills must not collapse.
 * A pill with no number renders its word alone rather than a confident `0`.
 */
export type FactsCounts = {
  included: number | null;
  candidates: number | null;
  all: number | null;
};

/**
 * Count the three buckets.
 *
 * @param cards the page's card pool, or `null` while the read is in flight
 *
 * ## Why `all` is a sum and not a third filter over the pool
 *
 * Included and Candidates are disjoint by construction — `includedRows` keeps
 * `status === "included"` and `proposedCount` counts cards carrying a live
 * proposal, which a ruled card no longer does — so the sum is exact. Counting
 * "everything the two lists would show" a third way would be a third place for
 * the arithmetic to be wrong, and the number nobody checks is the one that
 * drifts.
 */
export function factsCounts(cards: ScenarioCard[] | null): FactsCounts {
  if (cards === null) return { included: null, candidates: null, all: null };

  const included = includedRows(cards).length;
  const candidates = proposedCount(cards);
  return { included, candidates, all: included + candidates };
}

/** Whether the included list renders under this filter. */
export function showsIncluded(filter: FactsFilter): boolean {
  return filter === "included" || filter === "all";
}

/** Whether the candidate queue renders under this filter. */
export function showsCandidates(filter: FactsFilter): boolean {
  return filter === "candidates" || filter === "all";
}
