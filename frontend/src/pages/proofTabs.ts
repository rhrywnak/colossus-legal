// =============================================================================
// proofTabs.ts — the two halves of the proof surface
// =============================================================================
//
// In a module of its own so BOTH pages can name them without a circular import:
// `ProofMatrixPage` imports `ProofReviewPage` (it renders it for the review
// tab), so the review page cannot import the matrix back to get the list.
//
// The order is the design's, and it carries meaning: the FIRST tab is the
// default, and `PageTabs` leaves its id out of the URL — the canonical address
// of the matrix is `/cases/:slug/proof-matrix` with no query at all.

import type { PageTab } from "../components/PageTabs";

/** Matrix · Proof Review — the header tabs on the proof surface. */
export const PROOF_TABS: PageTab[] = [
  { id: "matrix", label: "Matrix" },
  { id: "review", label: "Proof Review" },
];
