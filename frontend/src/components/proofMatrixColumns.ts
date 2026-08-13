// =============================================================================
// proofMatrixColumns.ts — the ONE shared column template for the Proof Matrix
// -----------------------------------------------------------------------------
// The PM4 header row and every matrix-variant ElementRow import these so their
// columns cannot drift apart: change a column width or label here and both the
// header and the data rows move together (the lockstep the Part-3 instruction
// requires). Kept in its own module — neutral to both the page and the row — so
// neither owns the contract.
// =============================================================================

/**
 * CSS `grid-template-columns` for the five Proof Matrix columns, in order:
 * Element | Mapped Allegations | Supporting | Disputes | Status.
 *
 * `minmax(0, …)` on the flexible columns lets them shrink below their content
 * width instead of overflowing the row; the two fixed columns (the badge and the
 * status pill) hold a stable width so the numbers/pills line up down the table.
 */
export const PROOF_MATRIX_GRID_TEMPLATE =
  "minmax(0, 2fr) 130px minmax(0, 1fr) minmax(0, 1fr) 110px";

/**
 * Column header labels, in the same order as {@link PROOF_MATRIX_GRID_TEMPLATE}.
 *
 * "Disputes" (formerly the never-populated "Opposing") counts the Evidence that
 * REBUTS an Allegation bearing on the Element. The word is chosen deliberately:
 * not "Contradicts", which is reserved for the future evidence-vs-evidence
 * impeachment layer and would make two different relationships read as one; and
 * not "Opposing", which describes a party's posture rather than what the record
 * actually disputes.
 */
/**
 * Column header labels, in the same order as {@link PROOF_MATRIX_GRID_TEMPLATE}.
 *
 * The third label is NOT here. Since task 396 that column leads with the STRONG
 * count and its heading is a stored row (`matrix_strong_column_label`) — a
 * different claim from "how many items corroborate", and one Roman can retune
 * from the Settings page. It is passed in by the page, which has the served
 * wording; the four that remain are structural words this surface owns.
 */
export function proofMatrixColumnLabels(strongColumnLabel: string): string[] {
  return [
    "Element",
    "Mapped Allegations",
    strongColumnLabel,
    "Disputes",
    "Status",
  ];
}
