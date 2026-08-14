// =============================================================================
// matrixStrength.ts — the Proof Matrix's pure display helpers (task 396, P1)
// =============================================================================
//
// Two template fillers and one chip lookup. Nothing here computes a number and
// nothing here writes a sentence: the counts arrive from the backend already
// collapsed and tiered (`services::matrix_strength`), and every word comes from
// the served `MatrixWording`.
//
// ## Why these are pure functions in their own module
//
// CLAUDE.md §30: there is no component-test infrastructure in this repo, so
// anything worth asserting has to be reachable without React. The chip lookup in
// particular is worth asserting — it maps a backend token to a stored label, and
// a token this build does not know about must render NOTHING rather than the
// token itself, which is the sort of thing that only shows up on screen.
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
 * The strength tiers the backend can stamp on a drill-down row.
 *
 * Mirrors `domain::evidence_tier::EvidenceTier::code()`. `null` is a real value
 * on the wire, not an absence: it means the stored tier map does not name that
 * item's `(statement_type, evidence_strength)` pair. Such a row is still
 * approved, still counted and still rendered — it simply carries no chip.
 */
export type EvidenceTierToken = "strong" | "hedged" | "other";

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
 * The stored label for one tier token, or `null` when there is nothing to show.
 *
 * Returns `null` for BOTH a `null` token (an unmapped pair — the backend's own
 * "I have no claim about this one") and an unrecognized token (a tier a newer
 * backend defines and this build does not). The two are the same on screen for a
 * good reason: in neither case does this build have a word it is entitled to
 * print, and printing the raw token — `"hedged"` — would put the database's
 * vocabulary in front of Chuck.
 *
 * An unknown token IS worth an operator observable, so it is warned once at the
 * call site's expense rather than swallowed.
 */
export function tierChipLabel(
  tier: string | null,
  wording: MatrixWording,
): string | null {
  switch (tier) {
    case null:
      return null;
    case "strong":
      return wording.tier_strong_chip;
    case "hedged":
      return wording.tier_hedged_chip;
    case "other":
      return wording.tier_other_chip;
    default:
      // NOT the Rule 1 storage carve-out — that one is scoped to cosmetic
      // browser-storage preferences, and this is neither storage nor a
      // preference. It degrades for its own reason: a tier this build cannot
      // name renders as a row with NO CHIP, which is exactly how a legitimately
      // unmapped pair renders. No item is hidden and no count moves, so nothing
      // a reader acts on is affected — only a label is absent. A banner over a
      // missing label would be disproportionate; the console warning is the
      // observable, and it carries the offending token so an operator can see
      // the frontend is behind the backend's vocabulary.
      console.warn(
        `Proof Matrix: unknown evidence tier "${tier}" — the row renders without a ` +
          `strength chip. This build knows strong/hedged/other; the backend has sent ` +
          `something newer.`,
      );
      return null;
  }
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
