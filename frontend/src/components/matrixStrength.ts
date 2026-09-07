// =============================================================================
// matrixStrength.ts — the Proof Matrix ROW's pure display helpers (task 396, P1)
// =============================================================================
//
// Two template fillers. Nothing here computes a number and nothing here writes a
// sentence: the counts arrive from the backend already collapsed and tiered
// (`services::matrix_strength`), and every word comes from the served
// `MatrixWording`.
//
// ## What used to be here: `tierChipLabel`
//
// PROOF_MATRIX_v2 §3 removed the Strong/Hedged/Other chip from the drill-down —
// the tier was a claim about how hard an item is to dispute, and the reader-
// facing page now leads with what the linking pass and a human said instead. The
// lookup went with its only caller rather than staying as a function nothing
// reaches, which is the state a component in this repo has already been found in
// once. The three chip WORDS are still stored and still served: they label the
// matrix row's headline column, which is unchanged.
//
// ## Why these are pure functions in their own module
//
// CLAUDE.md §30: there is no component-test infrastructure in this repo, so
// anything worth asserting has to be reachable without React. The "×N" rule in
// particular is worth asserting — a marker that appeared on every row would be
// noise, and one that never appeared would hide that a statement was recorded
// more than once; neither shows up anywhere but on screen.
//
// ## The frontend composes nothing
//
// `fillCount` puts a served number into a served template, which is substitution,
// not composition — the same move `scenarioAugmentation.fillCap` makes and for
// the same reason. A template edited to drop its `{count}` is refused by the
// settings write path, so an unfilled placeholder on screen means the store was
// edited around the API, and it then shows verbatim and visibly wrong.

import type { MatrixWording } from "../services/causesOfAction";

/**
 * Put a served count into a served template.
 *
 * @param template a stored string carrying `{count}`
 * @param count the number the backend computed
 * @returns the template with `{count}` replaced
 */
export function fillCount(template: string, count: number): string {
  return template.replace("{count}", String(count));
}

/**
 * The "×N" marker for a collapsed row, or `null` when there is nothing to mark.
 *
 * Domain note: a row that collapsed nothing has `occurrences === 1`, and "×1" on
 * every line would be noise — so the marker appears only above one. This is the
 * one display decision this module makes, and it is stated here rather than
 * inline in JSX so the rule is testable.
 */
export function duplicateMarker(
  occurrences: number,
  wording: MatrixWording,
): string | null {
  if (occurrences <= 1) return null;
  return fillCount(wording.duplicate_template, occurrences);
}
