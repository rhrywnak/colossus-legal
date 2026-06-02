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
 * Element | Mapped Allegations | Supporting | Opposing | Status.
 *
 * `minmax(0, …)` on the flexible columns lets them shrink below their content
 * width instead of overflowing the row; the two fixed columns (the badge and the
 * status pill) hold a stable width so the numbers/pills line up down the table.
 */
export const PROOF_MATRIX_GRID_TEMPLATE =
  "minmax(0, 2fr) 130px minmax(0, 1fr) minmax(0, 1fr) 110px";

/** Column header labels, in the same order as {@link PROOF_MATRIX_GRID_TEMPLATE}. */
export const PROOF_MATRIX_COLUMN_LABELS = [
  "Element",
  "Mapped Allegations",
  "Supporting",
  "Opposing",
  "Status",
] as const;
