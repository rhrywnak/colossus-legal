// =============================================================================
// proofReviewHelpers.ts — pure view-shaping for the Proof Review page
// -----------------------------------------------------------------------------
// All shaping the page needs (filter options, sub-tab badge counts, the summary
// view-model, empty-state flags, client-side edge filtering) lives here as PURE
// functions: same input → same output, no DOM, no fetch, no React. That keeps
// ProofReviewPage.tsx a thin renderer and lets vitest exercise the logic without
// jsdom/RTL — matching the established frontend test pattern (CLAUDE.md §30; see
// countCardHelpers / elementAllegationListHelpers).
//
// NONE of this is business logic: every count/grouping was already computed by
// the backend. These helpers only re-key, filter, and read lengths of data the
// payload already contains (Standing Rule: no business logic in the frontend).
// =============================================================================

import type {
  ProofEdge,
  ProofReviewResponse,
  ProofReviewSummary,
  StatementTypeCount,
  CategoryCount,
} from "../services/proofReview";

/**
 * Distinct, sorted `source_document` values across every row in the payload
 * (proof edges + excluded + borderline). Populates the document-filter
 * `<select>` options — never a hardcoded document list. `null` source documents
 * are skipped (they cannot be a filter value).
 *
 * Borderline is a subset of proof_edges, so its documents are already covered;
 * it is included only for defensiveness (a future shape where it diverges).
 */
export function distinctSourceDocuments(
  payload: ProofReviewResponse,
): string[] {
  const seen = new Set<string>();
  const collect = (rows: { source_document: string | null }[]) => {
    for (const r of rows) {
      if (r.source_document) seen.add(r.source_document);
    }
  };
  collect(payload.proof_edges);
  collect(payload.excluded);
  collect(payload.borderline);
  return Array.from(seen).sort((a, b) => a.localeCompare(b));
}

/**
 * Distinct, sorted `statement_type` values present on a set of proof edges —
 * populates the proof-edges tab's statement_type filter. `statement_type` is
 * always present (required), so no null-guard is needed.
 */
export function distinctStatementTypes(edges: ProofEdge[]): string[] {
  const seen = new Set<string>();
  for (const e of edges) seen.add(e.statement_type);
  return Array.from(seen).sort((a, b) => a.localeCompare(b));
}

/** Per-list-tab row counts, for the sub-tab badges. */
export type SubTabBadgeCounts = {
  proofEdges: number;
  excluded: number;
  borderline: number;
};

/**
 * The three list-tab badge counts (Proof edges / Excluded / Borderline), read
 * verbatim from the payload array lengths — no recomputation. On current data
 * these are 43 / 79 / 19.
 */
export function subTabBadgeCounts(
  payload: ProofReviewResponse,
): SubTabBadgeCounts {
  return {
    proofEdges: payload.proof_edges.length,
    excluded: payload.excluded.length,
    borderline: payload.borderline.length,
  };
}

/** The Summary tab's flattened view-model (passthrough of backend counts). */
export type SummaryView = {
  corroboratingTotal: number;
  excludedTotal: number;
  corroboratingByStatementType: StatementTypeCount[];
  corroboratingByCategory: CategoryCount[];
  excludedByStatementType: StatementTypeCount[];
};

/**
 * Flatten the nested `summary` into the rows the Summary tab renders. This is a
 * verbatim read of the backend's counts (the two corroborating/excluded totals
 * and the per-category breakdowns) — no summing, no client-side math. Pulling it
 * into a helper makes the "the page shows the four excluded categories and the
 * two corroborating totals" contract unit-testable.
 */
export function buildSummaryView(summary: ProofReviewSummary): SummaryView {
  return {
    corroboratingTotal: summary.corroborating.total,
    excludedTotal: summary.excluded.total,
    corroboratingByStatementType: summary.corroborating.by_statement_type,
    corroboratingByCategory: summary.corroborating.by_category,
    excludedByStatementType: summary.excluded.by_statement_type,
  };
}

/** Whether each region has nothing to render (drives explicit empty states). */
export type SectionEmptyStates = {
  summaryEmpty: boolean;
  proofEdgesEmpty: boolean;
  excludedEmpty: boolean;
  borderlineEmpty: boolean;
};

/**
 * Empty-state flags per sub-view, so each tab renders an explicit "No …"
 * message rather than a blank panel (Charter §8 honesty rule). The Summary is
 * "empty" only when both corroboration and exclusion totals are zero.
 */
export function sectionEmptyStates(
  payload: ProofReviewResponse,
): SectionEmptyStates {
  return {
    summaryEmpty:
      payload.summary.corroborating.total === 0 &&
      payload.summary.excluded.total === 0,
    proofEdgesEmpty: payload.proof_edges.length === 0,
    excludedEmpty: payload.excluded.length === 0,
    borderlineEmpty: payload.borderline.length === 0,
  };
}

/** Client-side filter selections for the proof-edges tab. `"all"` = no filter. */
export type EdgeFilter = {
  statementType: string;
  sourceDocument: string;
};

/** The sentinel a filter `<select>` uses for "no filter on this dimension". */
export const EDGE_FILTER_ALL = "all";

/**
 * Filter proof edges client-side by `statement_type` and/or `source_document`.
 * Either dimension set to {@link EDGE_FILTER_ALL} (the default) is a no-op for
 * that dimension. Pure: returns a new array, never mutates the input, and
 * preserves the backend's row order (the rows arrive pre-sorted by
 * source_document / page / paragraph).
 *
 * This is the only "logic" the proof-edges tab runs over the fetched payload —
 * narrowing which of the N rows are shown. It changes nothing about the counts
 * (the Summary tab and badges always reflect the full, unfiltered payload).
 */
export function filterEdges(
  edges: ProofEdge[],
  filter: EdgeFilter,
): ProofEdge[] {
  return edges.filter((e) => {
    const typeOk =
      filter.statementType === EDGE_FILTER_ALL ||
      e.statement_type === filter.statementType;
    const docOk =
      filter.sourceDocument === EDGE_FILTER_ALL ||
      e.source_document === filter.sourceDocument;
    return typeOk && docOk;
  });
}
