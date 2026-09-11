// =============================================================================
// themeScanExpansion.ts — whether the scan card's body is showing
// =============================================================================
//
// One rule, extracted from `ThemeScanPanel` so it can be tested without a DOM
// (RTL/jsdom are not set up in this repo — CLAUDE.md Rule 30), and because it is
// the rule that failed on DEV.
//
// ## The defect this exists to prevent (S-13, 2026-09-10)
//
// The rule used to be, in full:
//
//     const expanded = expandOverride ?? collapsedSummary === null;
//
// which says: fold the card once there is a settled run to describe. That is
// right for a finished scan — the work has moved to the queue below — and it is
// wrong for a RUNNING one, because the running view is inside the folded body.
//
// Measured on DEV: S-13 had a scan going (40 of 273 judged) and four earlier
// FAILED runs. The failed runs make `collapsedSummary` non-null, so `expanded`
// was false, so the running view did not render — while the header's own tooltip
// said "A scan is running — its progress is below." Nothing was below it.
//
// And it could not be opened. The collapse control only renders when `header` is
// undefined; the v2.1 layout passes a header, so on the scenario page there is no
// expand affordance at all. A human meeting this has no way out of it except to
// wait for the scan to finish.
//
// So two overrides, and both are about a control that does not exist rather than
// about taste:
//
//   1. A RUNNING scan always shows. Progress the page promises must be on the
//      page (Standing Rule 1: the screen may not claim something it is not
//      showing).
//   2. With a caller-supplied header, always show. There is no control to undo a
//      fold in that layout, so folding is a one-way door.

/** What decides whether the scan card's body renders. */
export interface ExpansionInputs {
  /** The human's own click, or `null` while the computed default stands. */
  expandOverride: boolean | null;
  /** The folded card's one line, or `null` when there is nothing to fold. */
  collapsedSummary: string | null;
  /** A scan is in flight — its progress lives in the body. */
  running: boolean;
  /**
   * The caller is drawing the header itself (the v2.1 scenario-page layout).
   *
   * When true there is NO collapse control on screen, so a folded card cannot be
   * unfolded. See rule 2 above.
   */
  headerLent: boolean;
}

/**
 * Is the scan card's body showing?
 *
 * ## Why the two overrides beat the human's own click
 *
 * `expandOverride` is a real preference and it is honoured in the ordinary case.
 * It is overridden here because in both of these situations obeying it would
 * leave the page unable to say something it has already claimed:
 *
 * * **running** — the header says progress is below; the body is where progress
 *   is. A card folded by a click made ten minutes ago would hide a scan the
 *   person is now waiting on, and they would have to remember they had folded it.
 * * **headerLent** — there is no chevron in that layout. A `false` here is not a
 *   preference, it is a dead end.
 *
 * The collapse is deliberately not persisted (ruling R7), so neither override can
 * outlive the page it was made on.
 */
export function isExpanded({
  expandOverride,
  collapsedSummary,
  running,
  headerLent,
}: ExpansionInputs): boolean {
  if (running) return true;
  if (headerLent) return true;
  return expandOverride ?? collapsedSummary === null;
}
