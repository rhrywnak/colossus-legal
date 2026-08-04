// =============================================================================
// filteredCounter.ts — the §9 honest counter line, owned once (task 1.7E, R1)
// =============================================================================
//
// Two surfaces now show "how much of the whole am I looking at": the Bias
// Explorer's filter bar (since v2) and the candidate list's filter pills (task
// 1.7E). Ruling R1: the honesty rule they share is LIFTED here rather than
// copied, so a change to what honesty means happens in one place.
//
// ## The rule this file owns
//
// The wording is driven by the human's INTENT (`hasAnyFilter`), never by the
// arithmetic accident that `filtered === total`. A pool small enough that every
// filter matches everything would otherwise read "Showing all …" while a filter
// is visibly in force — which tells the reader the filter did nothing, when in
// fact it selected everything. §9: a count that can mislead is a defect.
//
// ## Why the noun is a PARAMETER and not a string this file picks
//
// Bias Explorer counts "instances"; the candidate list counts "candidates". The
// helper owns the RULE (which sentence, singular or plural), and the caller owns
// the WORD, because the word is that surface's own vocabulary. Building the noun
// in here would put two surfaces' vocabularies in one module and make adding a
// third an edit to shared code.

/**
 * The singular/plural pair a surface counts in.
 *
 * ## TS learning: a two-field object rather than a `noun: string` plus an `s`
 *
 * English plurals are not "append s" ("instance"/"instances" is, "party"/
 * "parties" is not), and a helper that guesses would be composing a word it does
 * not know. Making both forms explicit means the caller states them and this
 * module never invents one.
 */
export type CounterNoun = { singular: string; plural: string };

/**
 * The counter line under a filter row.
 *
 * @param filtered How many rows the active filters leave visible.
 * @param total    How many rows exist unfiltered — the honest denominator.
 * @param hasAnyFilter Whether the human has any filter applied. Drives the
 *        wording; see the intent rule in this file's header.
 * @param noun The surface's own word for what it is counting.
 */
export function filteredCounterLine(
  filtered: number,
  total: number,
  hasAnyFilter: boolean,
  noun: CounterNoun,
): string {
  const word = total === 1 ? noun.singular : noun.plural;
  if (hasAnyFilter) {
    return `Filtered: ${filtered} of ${total} ${word}`;
  }
  return `Showing all ${total} ${word}`;
}
