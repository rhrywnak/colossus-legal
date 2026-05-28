/**
 * paragraphSort.ts — shared helper for ordering items by their complaint
 * paragraph reference.
 *
 * Both the Element detail panel (allegations mapped to an Element) and the
 * Evidence Explorer page (allegations per legal Count) need to render
 * allegations in **paragraph order**. The values come back from Neo4j as
 * strings because some allegations carry ranges (`"16-18"`) — `parseInt`
 * alone would silently accept the leading number, but we want a fully-
 * explicit helper that:
 *
 *   - Parses the leading numeric prefix of a string.
 *   - Returns `null` (not `0`, not `NaN`) when there is no leading digit,
 *     so callers can sort non-numeric values to the END instead of
 *     misordering them as 0.
 *
 * Centralising the parse here keeps the contract identical for every
 * consumer — the previous duplication risked subtle drift (one site uses
 * `parseInt`, another uses a regex, neither handles ranges).
 *
 * ## React/TS Learning: a pure helper as a leaf module
 * No React imports, no DOM, no fetch. The function is referentially
 * transparent — same input, same output, no side effects — which makes it
 * trivially unit-testable from vitest without any rendering setup. The
 * Evidence page and the Element panel both import it; each writes its own
 * one-line `Array.sort` comparator at the call site so the sort intent
 * stays readable where you read the page.
 */

/**
 * Parse the leading integer prefix of a paragraph reference string.
 *
 * Examples:
 *   parseLeadingParagraph("10")    === 10
 *   parseLeadingParagraph("16-18") === 16   // range start
 *   parseLeadingParagraph("abc")   === null // non-numeric → null
 *   parseLeadingParagraph("")      === null
 *   parseLeadingParagraph("¶7")    === null // leading non-digit
 *   parseLeadingParagraph("-3")    === null // leading sign is not a digit
 *
 * Returns `null` rather than throwing, returning `0`, or returning `NaN`:
 *
 *   - `0` would slot non-numeric values BEFORE every real paragraph in an
 *     ascending sort, which is the opposite of "sort to the end".
 *   - `NaN` propagates through comparators silently; `NaN < N` is `false`
 *     for every N, so non-numeric values would order non-deterministically.
 *   - Throwing forces every call site to wrap in try/catch.
 *
 * `null` is the unambiguous "no leading integer" signal; comparators put
 * `null` last by explicit check.
 */
export function parseLeadingParagraph(paragraphReference: string): number | null {
  let i = 0;
  while (
    i < paragraphReference.length &&
    paragraphReference[i] >= "0" &&
    paragraphReference[i] <= "9"
  ) {
    i++;
  }
  if (i === 0) return null;
  const parsed = Number.parseInt(paragraphReference.slice(0, i), 10);
  return Number.isFinite(parsed) ? parsed : null;
}
