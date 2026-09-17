// =============================================================================
// countWording.ts — pick the singular or plural stored template for a count
// =============================================================================
//
// CC_GO_REVIEW_LOOP_v3: "1 answers" does not ship — lawyers read these screens.
// Every count-bearing sentence has TWO stored rows, a singular and a plural,
// and this is the ONE place that decides between them.
//
// ## Why exactly 1, and nothing cleverer
//
// English (the only language the store holds today) takes the singular for 1
// and the plural for everything else, 0 included. A locale-aware plural rule
// would be the right tool the day the store gains a second language; until then
// it would be a dependency deciding a question with one answer.

/**
 * The template to fill for `count`: `one` when the count is exactly 1, else `many`.
 *
 * @param count the number the sentence reports
 * @param one   the stored singular row
 * @param many  the stored plural row
 */
export function pickByCount(count: number, one: string, many: string): string {
  return count === 1 ? one : many;
}
