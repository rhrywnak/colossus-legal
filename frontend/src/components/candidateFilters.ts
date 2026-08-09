// =============================================================================
// candidateFilters.ts — the candidate list's pure filter model (task 1.7E-a)
// =============================================================================
//
// The one-card-at-a-time queue was unusable for working a 148-candidate pool: no
// skip forward, no skip back, no way to see where the rulable candidates sit.
// 1.7E makes the queue a scrollable, filterable LIST, and this module is the part
// of that which is data rather than DOM — which state a card is in, how many are
// in each state, which cards a filter leaves visible, and which filter the list
// opens on.
//
// ## Everything here is pure, and that is the point
//
// CLAUDE.md rule 30 records that component-test infrastructure (RTL, jsdom) is
// deliberately not set up. So the §9 obligation — every displayed count
// reconciles — is only testable if the counting is a function over payload
// structs rather than something the JSX does while rendering. It is, here.
//
// ## The counts are derived ONCE (ruling R1)
//
// `candidateCounts` walks the pool a single time and returns every facet count
// together, following the `DocumentsPage` counts-derivation shape. The reason is
// not performance: two independently-derived counts are two things that can
// disagree, and a filter bar whose "Not ruled (100)" contradicts its "All (148)"
// is precisely the §9 defect this task exists to remove.
//
// ## The frontend composes no CASE vocabulary
//
// The words here name the LIST'S OWN CONTROLS — "Not ruled", "Rulable now",
// "Deferred" — the same class of word as the Include/Exclude/Defer buttons, which
// the frontend has always owned. Nothing here names a party, a document, a claim,
// or any other case vocabulary: every such string still arrives composed from the
// backend and is rendered verbatim (see `cardTriage.cardRows`).

import type { ScenarioCard } from "../services/scenarioCards";
import type { CardGrammarWording } from "../services/evidenceLinks";

// ─── What state a candidate is in ───────────────────────────────────────────

/**
 * The four mutually exclusive states a candidate can be in.
 *
 * ## Domain note: why "deferred" is a state and the payload has no such status
 *
 * A defer lands the row in `undecided` — the SAME status as never having been
 * looked at — with a `defer_reason` recording the act (see `FactAction::Defer` in
 * the backend). That is correct storage and a terrible filter: "not ruled" and
 * "parked with a stated reason" are different work, and a human clearing a pool
 * needs to tell them apart. So the state is derived from the PAIR, and the four
 * states below partition the pool exactly — which is what makes the counts
 * reconcile.
 */
export type CandidateState = "not_ruled" | "included" | "excluded" | "deferred";

/**
 * Which state one card is in.
 *
 * Reads `status` (the machine token) and `defer_reason` (present iff a human
 * parked it), never `status_label` — filtering on the prose would break the day
 * the wording changes, which is exactly why the payload ships both.
 */
export function candidateState(card: ScenarioCard): CandidateState {
  if (card.status === "included") return "included";
  if (card.status === "dropped") return "excluded";
  return card.defer_reason != null ? "deferred" : "not_ruled";
}

/**
 * Whether a card can be ruled on right now — the "Rulable now" predicate.
 *
 * Two conditions, both necessary: nobody has ruled it yet, and it is not
 * defer-only. `defer_required` is the backend's own flag (it accompanies the
 * composed reason on the wire), so this is a read, not a re-derivation.
 *
 * ## Domain note: why this is a SUBSET of "not ruled" rather than a fifth state
 *
 * A defer-only card is genuinely not ruled — it is in the pool, awaiting a
 * decision that cannot be made until linking arrives (task 2.10). Counting it as
 * its own state would make "Not ruled" understate the work left. So the options
 * show a partition (all four states sum to All) plus this one deliberate subset,
 * which the bar labels as such so no reader adds it into the total.
 */
export function isRulableNow(card: ScenarioCard): boolean {
  return candidateState(card) === "not_ruled" && !card.defer_required;
}

/**
 * Whether the latest completed scan is PROPOSING this card (2026-08-08).
 *
 * ## Why this replaced the "scan-scored" facet
 *
 * That facet read `confidence.band !== "unscored"`, and a card only gained a
 * confidence when a scan verdict was MERGED onto it — so "scan-scored" actually
 * meant "somebody merged this", and a scenario scanned three times with nothing
 * merged reported every card as never scanned. Worse, ruling a card IN nulls its
 * confidence, so an included card silently LEFT the facet it had been in.
 *
 * Both defects are gone by construction rather than by repair. `proposed` is
 * present exactly when the projection put this card forward, and precedence R-a
 * keeps any human-touched card out of it whatever its confidence says.
 */
export function isProposed(card: ScenarioCard): boolean {
  return card.proposed != null;
}

// ─── The filter model ───────────────────────────────────────────────────────

/**
 * The FIVE facets the chips offer (ONE_CARD_GRAMMAR Piece 1a, ruling R5).
 *
 * ## "Rulable now" retired, and "All" became "Full pool"
 *
 * Both changes are the same ruling. The bar carried six options, two of which
 * were subsets of a third, and it made the human read a taxonomy before they
 * could read a count — "is Rulable now part of Not ruled, or beside it?" was a
 * question the hint text existed to answer. The five below are what a curator
 * actually asks for, and a locked card now rides INSIDE Proposed stating its own
 * condition (Piece 4b) rather than being sorted out of it by a facet.
 *
 * `full_pool` replaces `all` in name because the old word read as a to-do list.
 * What it names is the denominator, not the work — which is exactly what its ⓘ
 * says.
 */
export type StateFacet = "proposed" | "deferred" | "included" | "excluded" | "full_pool";

export type CandidateFilters = { state: StateFacet };

/** No filter at all — the honest denominator's view. */
export const UNFILTERED: CandidateFilters = { state: "full_pool" };

/**
 * Whether the human has narrowed anything.
 *
 * Drives the counter line's wording (see `filteredCounterLine`) — intent, never
 * the arithmetic accident that the filtered count equals the total.
 */
export function hasAnyFilter(filters: CandidateFilters): boolean {
  return filters.state !== "full_pool";
}

/** Every facet count, derived in one pass so no two of them can disagree. */
export type CandidateCounts = {
  all: number;
  not_ruled: number;
  rulable: number;
  deferred: number;
  included: number;
  excluded: number;
  /** Cards the latest completed scan is proposing — a SUBSET of `not_ruled`,
   *  exactly as `rulable` is, because precedence keeps every human-touched card
   *  out of it. Never added into the total. */
  proposed: number;
};

/**
 * Count every facet in a single walk of the pool (ruling R1).
 *
 * The four state counts partition `all` exactly; `rulable` and `proposed` are
 * each a subset of `not_ruled`. Those identities are asserted by the tests,
 * because §9's promise is not that the numbers look plausible but that they
 * reconcile.
 */
export function candidateCounts(cards: ScenarioCard[]): CandidateCounts {
  const counts: CandidateCounts = {
    all: cards.length,
    not_ruled: 0,
    rulable: 0,
    deferred: 0,
    included: 0,
    excluded: 0,
    proposed: 0,
  };

  for (const card of cards) {
    // The state facets. One increment per card — the four are exclusive by
    // construction, which is what makes them sum to `all`.
    switch (candidateState(card)) {
      case "not_ruled":
        counts.not_ruled += 1;
        break;
      case "included":
        counts.included += 1;
        break;
      case "excluded":
        counts.excluded += 1;
        break;
      case "deferred":
        counts.deferred += 1;
        break;
    }
    if (isRulableNow(card)) counts.rulable += 1;
    if (isProposed(card)) counts.proposed += 1;
  }

  return counts;
}

/**
 * Whether one card survives the active filter.
 *
 * Exported since 2026-08-08 because the QUEUE has to ask it about a card it has
 * just ruled: a ruled card usually stops matching (Proposed and Rulable-now both
 * exclude it), so it leaves the list — and a card that disappears with nothing
 * said reads exactly like a click that did nothing. That was the measured
 * beta.385 defect. The queue asks this, and says so.
 */
export function matchesFilter(card: ScenarioCard, filters: CandidateFilters): boolean {
  return matches(card, filters);
}

function matches(card: ScenarioCard, filters: CandidateFilters): boolean {
  if (filters.state === "full_pool") return true;
  if (filters.state === "proposed") return isProposed(card);
  return filters.state === candidateState(card);
}

/**
 * The cards a filter leaves visible, in the order they arrived.
 *
 * ## Why the order is never touched here
 *
 * The backend sorts the pool by C-ordinal and says why (`sort_by_code`: "C-10" <
 * "C-9" lexicographically, so the ordinal is read back out of the code rather
 * than the display string being compared). Re-sorting in the browser would be the
 * client re-deriving an order the backend owns; filtering preserves it instead.
 */
export function filterCandidates(
  cards: ScenarioCard[],
  filters: CandidateFilters,
): ScenarioCard[] {
  return cards.filter((card) => matches(card, filters));
}

/**
 * Which filter the list opens on.
 *
 * ## Proposals lead when there are any (2026-08-08, unchanged)
 *
 * A completed scan's candidates are what the human came to the page to rule, and
 * before this they arrived buried in a pool of 148 with nothing marking them. So
 * when the projection is proposing anything, the queue opens on it.
 *
 * ## And when there are none, the FULL POOL (ruling R5)
 *
 * The old fallback was "Rulable now while any exist, else Not ruled", and both
 * facets are gone. The honest replacement is the denominator: a scenario with
 * nothing proposed has no shortlist to lead with, and opening on a slice would
 * mean the first thing a human sees is a filtered view they did not choose and
 * cannot see the edges of. The chips are one click each; the pool says how much
 * there is.
 */
export function defaultFilters(counts: CandidateCounts): CandidateFilters {
  return counts.proposed > 0 ? { state: "proposed" } : { state: "full_pool" };
}

// ─── The filter CHIPS (Piece 1a) ────────────────────────────────────────────

/**
 * One filter chip, as data: its facet, its stored name, and its count.
 *
 * ## Why the LABEL is now a stored word and the four state-chip words are not
 *
 * They answer different questions. A state chip says what happened to one card
 * ("Included ✓") and is the list's own control vocabulary, in the same class as
 * the Include button — the frontend has always owned those. A FILTER name is
 * case-facing: "Proposed" is a claim about what a scan did, Marie and Chuck read
 * it before they read anything else on the page, and Roman renames that kind of
 * word without a rebuild (§2b, as extended to text).
 */
export type FilterChip = {
  facet: StateFacet;
  label: string;
  count: number;
  /** The ⓘ text, on the one chip whose meaning a reader cannot infer. */
  explainer?: string;
};

/**
 * The five chips, in display order, with their counts.
 *
 * Proposed LEADS because it is the work the page exists to get through and what
 * the queue opens on when a scan has run. Full pool is LAST because it is the
 * denominator rather than a slice — and it carries the ⓘ, which is Roman's
 * addition: Marie and Chuck will not know the term, and a filter whose meaning a
 * reader has to infer from its count is a filter they will not press.
 *
 * Counts come from the ONE derivation (`candidateCounts`), so no two numbers on
 * this bar can disagree — the half of ruling R1 that survives every redesign of
 * the control.
 */
export function filterChips(
  counts: CandidateCounts,
  wording: CardGrammarWording,
): FilterChip[] {
  return [
    { facet: "proposed", label: wording.filter_proposed_label, count: counts.proposed },
    { facet: "deferred", label: wording.filter_deferred_label, count: counts.deferred },
    { facet: "included", label: wording.filter_included_label, count: counts.included },
    { facet: "excluded", label: wording.filter_excluded_label, count: counts.excluded },
    {
      facet: "full_pool",
      label: wording.filter_full_pool_label,
      count: counts.all,
      explainer: wording.full_pool_explainer,
    },
  ];
}

/**
 * The active facet's own label, for a sentence that has to name the list.
 *
 * Reads the same chip table the bar renders, so the sentence and the control can
 * never call one list two names — the counts are irrelevant here, so a zeroed set
 * is passed rather than threading the real ones through.
 */
export function facetLabel(facet: StateFacet, wording: CardGrammarWording): string {
  const zero: CandidateCounts = {
    all: 0,
    not_ruled: 0,
    rulable: 0,
    deferred: 0,
    included: 0,
    excluded: 0,
    proposed: 0,
  };
  return filterChips(zero, wording).find((c) => c.facet === facet)?.label ?? facet;
}

/**
 * How many of the ACTIVE filter's cards have been ruled (Piece 1c).
 *
 * ## Domain note: the denominator is the filter, never the pool
 *
 * The line this replaces read "23 of 148 ruled" over a progress bar of 125
 * nobody owes. Rule-the-promising is the ratified triage model — a curator works
 * the proposals and leaves the rest — so a bar measuring the whole gathered pool
 * was reporting a debt the method says does not exist.
 *
 * "Ruled" here means a human has decided: included or excluded. A DEFERRED card
 * is deliberately not counted as ruled — it is parked, and the work of deciding
 * it is still outstanding, which is the whole reason defer is its own verb.
 */
export function filterProgress(
  visible: ScenarioCard[],
): { ruled: number; total: number } {
  let ruled = 0;
  for (const card of visible) {
    const state = candidateState(card);
    if (state === "included" || state === "excluded") ruled += 1;
  }
  return { ruled, total: visible.length };
}

// ─── The state chip ─────────────────────────────────────────────────────────

/**
 * The colour family a state chip wears. NOT a colour — a tone.
 *
 * The component maps a tone to v3 tokens; this module stays free of hex values
 * and of `var(--…)` names, so the chip's vocabulary can be tested without a DOM
 * and without the tests knowing anything about the palette.
 */
export type ChipTone = "neutral" | "success" | "danger" | "warning";

/** A state chip as data: an icon, a word, and a tone. */
export type StateChip = { icon: string; label: string; tone: ChipTone };

/**
 * How a candidate's state renders as a chip.
 *
 * ## Why colour is never alone
 *
 * Every chip carries an ICON and a WORD as well as its tone. A reader with a
 * colour-vision difference, a greyscale print of a screenshot, and a lawyer
 * skimming at a distance all need the state to survive the loss of hue — and the
 * §2c visual language requires it (v3, item 2).
 *
 * ## Why the label is the list's own word rather than `status_label`
 *
 * Three of these states have a payload label (`status_label`), and the fourth —
 * deferred — does not: a deferred row's status IS `undecided`, so its label reads
 * "Not yet decided", which is the one thing the chip must not say about a card a
 * human deliberately parked. Taking three words from the payload and inventing
 * the fourth would make the chip's vocabulary half one thing and half another.
 * These four are control words, like the Defer button beside them, so the list
 * owns all four and the payload's `status_label` still renders verbatim in the
 * card's own status row.
 */
export function stateChip(state: CandidateState): StateChip {
  switch (state) {
    case "included":
      return { icon: "✓", label: "Included", tone: "success" };
    case "excluded":
      return { icon: "✕", label: "Excluded", tone: "danger" };
    case "deferred":
      return { icon: "⏸", label: "Deferred", tone: "warning" };
    case "not_ruled":
      return { icon: "○", label: "Not ruled", tone: "neutral" };
  }
}
