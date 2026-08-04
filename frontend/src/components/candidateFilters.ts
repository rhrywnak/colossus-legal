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
import { filteredCounterLine } from "./shared/filteredCounter";

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
 * Whether any scan has ever scored this item.
 *
 * Roman's finding, 2026-08-03: the pool carries fossils from old processing runs
 * that no current scan has looked at, and they deserve visible skepticism. The
 * band vocabulary already preserves the distinction — `unscored` is NOT `low`
 * (see `ConfidenceBand::Unscored`) — so the facet reads the band rather than
 * inventing a second notion of "scanned".
 */
export function isScanScored(card: ScenarioCard): boolean {
  return card.confidence.band !== "unscored";
}

// ─── The filter model ───────────────────────────────────────────────────────

/** The state facet: the four states, the rulable subset, or everything. */
export type StateFacet = "all" | CandidateState | "rulable";

/** The scanned facet (Roman's skepticism filter). */
export type ScannedFacet = "any" | "scored" | "never";

export type CandidateFilters = { state: StateFacet; scanned: ScannedFacet };

/** No filter at all — the honest denominator's view. */
export const UNFILTERED: CandidateFilters = { state: "all", scanned: "any" };

/**
 * Whether the human has narrowed anything.
 *
 * Drives the counter line's wording (see `filteredCounterLine`) — intent, never
 * the arithmetic accident that the filtered count equals the total.
 */
export function hasAnyFilter(filters: CandidateFilters): boolean {
  return filters.state !== "all" || filters.scanned !== "any";
}

/** Every facet count, derived in one pass so no two of them can disagree. */
export type CandidateCounts = {
  all: number;
  not_ruled: number;
  rulable: number;
  deferred: number;
  included: number;
  excluded: number;
  scored: number;
  never_scanned: number;
};

/**
 * Count every facet in a single walk of the pool (ruling R1).
 *
 * The four state counts partition `all` exactly; `rulable` is a subset of
 * `not_ruled`; `scored` + `never_scanned` is `all` again. Those three identities
 * are asserted by the tests, because §9's promise is not that the numbers look
 * plausible but that they reconcile.
 */
export function candidateCounts(cards: ScenarioCard[]): CandidateCounts {
  const counts: CandidateCounts = {
    all: cards.length,
    not_ruled: 0,
    rulable: 0,
    deferred: 0,
    included: 0,
    excluded: 0,
    scored: 0,
    never_scanned: 0,
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
    if (isScanScored(card)) counts.scored += 1;
    else counts.never_scanned += 1;
  }

  return counts;
}

/** Whether one card survives the active filters. */
function matches(card: ScenarioCard, filters: CandidateFilters): boolean {
  const state = candidateState(card);
  const stateOk =
    filters.state === "all" ||
    (filters.state === "rulable" ? isRulableNow(card) : filters.state === state);
  if (!stateOk) return false;

  if (filters.scanned === "any") return true;
  return filters.scanned === "scored" ? isScanScored(card) : !isScanScored(card);
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
 * "Rulable now" while any exist, else "Not ruled". The reason is the task's own
 * premise: the human is looking for the handful of candidates they can actually
 * decide, and opening on All means finding them by scrolling — which is the
 * defect. When none are rulable, defaulting to Rulable now would open on an empty
 * list, so it falls back to the honest next-widest view.
 */
export function defaultFilters(counts: CandidateCounts): CandidateFilters {
  return { state: counts.rulable > 0 ? "rulable" : "not_ruled", scanned: "any" };
}

// ─── The dropdown options ───────────────────────────────────────────────────

/**
 * One option in a filter dropdown, as data.
 *
 * ## From pills to selects (task 1.7G, Roman's ruling)
 *
 * 1.7E's ruling R1 declined `<select>` dropdowns on the grounds that a count
 * inside a closed dropdown is a count nobody can see. Roman overruled it: he had
 * named the Bias Analysis page's filter controls as the pattern twice, and the
 * pill rows were a pattern he never approved. His ruling R3 answers R1's objection
 * on its own terms — the options CARRY their counts, exactly as the Bias Analysis
 * page's do ("Rulable now (42)"), so nothing is lost but the two rows of chips.
 *
 * `hint` is what the facet MEANS, said in a sentence, because a two-word label
 * with a number on it invites a reader to guess (is "Rulable now" part of "Not
 * ruled", or beside it?) and §9 does not permit guessing.
 */
export type FilterOption<F> = { facet: F; label: string; count: number; hint: string };

/** The Status options, in display order. Counts come from the one derivation. */
export function stateOptions(counts: CandidateCounts): FilterOption<StateFacet>[] {
  return [
    {
      facet: "all",
      label: "All",
      count: counts.all,
      hint: "Every candidate in this scenario, whatever has been decided about it.",
    },
    {
      facet: "not_ruled",
      label: "Not ruled",
      count: counts.not_ruled,
      hint: "Nobody has included, excluded or deferred these yet.",
    },
    {
      facet: "rulable",
      label: "Rulable now",
      count: counts.rulable,
      hint:
        "The not-ruled candidates that can be decided as they stand — the rest " +
        "are not linked to an accusation yet, so only Defer is available on them. " +
        "Part of Not ruled, not a separate group.",
    },
    {
      facet: "deferred",
      label: "Deferred",
      count: counts.deferred,
      hint: "Parked with a stated reason. They stay in the queue.",
    },
    {
      facet: "included",
      label: "Included",
      count: counts.included,
      hint: "Confirmed as facts of this scenario.",
    },
    {
      facet: "excluded",
      label: "Excluded",
      count: counts.excluded,
      hint: "Set aside for this scenario. The evidence itself is untouched elsewhere.",
    },
  ];
}

/** The Scan options. Roman's skepticism filter, said out loud. */
export function scannedOptions(counts: CandidateCounts): FilterOption<ScannedFacet>[] {
  return [
    {
      facet: "any",
      label: "Any",
      count: counts.all,
      hint: "Scanned or not — no filter on scan history.",
    },
    {
      facet: "scored",
      label: "Scan-scored",
      count: counts.scored,
      hint: "A scan has looked at these and reported a confidence.",
    },
    {
      facet: "never",
      label: "Never scanned",
      count: counts.never_scanned,
      hint:
        "No scan has ever scored these — they are pool fossils from earlier " +
        "processing. Unscored is not low confidence; nobody has looked.",
    },
  ];
}

/**
 * The counter line under the filter row, in the list's own noun.
 *
 * The §9 honesty rule itself lives in the shared helper (ruling R1) and is shared
 * with the Bias Explorer's bar; what this function contributes is the word
 * "candidate".
 */
export function candidateCounterLine(
  shown: number,
  total: number,
  filters: CandidateFilters,
): string {
  return filteredCounterLine(shown, total, hasAnyFilter(filters), {
    singular: "candidate",
    plural: "candidates",
  });
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
