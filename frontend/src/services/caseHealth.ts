// =============================================================================
// caseHealth.ts — client for GET /api/cases/:slug/case-health/inventory
// -----------------------------------------------------------------------------
// Endpoint: backend handler `api::case_health`. Neo4j-backed, read-only. Pane 1
// of the Case Health dashboard ("Graph Inventory").
//
// These types mirror the Rust DTO (`dto::case_health`) exactly. Every number the
// page shows is already computed here — the rates, the inert counts, the
// ordering and the edge-class column list all arrive from the backend. This
// module therefore contains NO arithmetic and NO sorting, and the page contains
// none either: "zero business logic in the frontend" is the pane's defining
// constraint, and the payload shape is what enforces it.
//
// Domain note on the two rates, because the distinction is the whole point:
// `probative` counts Evidence carrying at least one CORROBORATES / REBUTS /
// CHARACTERIZES edge to an Allegation — edges that bear on whether the
// allegation is TRUE. `topical` additionally counts ABOUT, which asserts only
// that the item is on the subject. The probative rate is the headline; the
// topical rate renders beside it, separately labeled. They are NEVER blended
// into one number, and no component may add, average, or otherwise combine them.
// =============================================================================

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { DEFAULT_CASE_SLUG } from "./caseHeader";

/** A populated node label and how many nodes carry it. */
export type LabelCount = {
  label: string;
  node_count: number;
};

/**
 * One (from-label, rel-type, to-label) triple and its edge count.
 * `from_label` / `to_label` are absent when the endpoint node carries no label.
 */
export type EdgeTripleCount = {
  from_label?: string;
  rel_type: string;
  to_label?: string;
  edge_count: number;
};

/**
 * One Evidence→Allegation edge class and how many of a document's Evidence
 * items carry at least one edge of it.
 *
 * These are ITEM counts, not edge counts, and they do NOT sum to
 * `topical_connected` — one item may both corroborate one allegation and rebut
 * another, so it appears under both classes while contributing 1 to the
 * connected total. Do not try to reconcile them by addition.
 */
export type EdgeClassCount = {
  rel_type: string;
  item_count: number;
};

/**
 * The corpus-wide rollup. Rates are `null` — never `0` — when there is no
 * Evidence to rate; render `null` as an em-dash, not as "0%".
 */
export type CorpusConnection = {
  evidence_total: number;
  probative_connected: number;
  topical_connected: number;
  inert: number;
  probative_rate_pct: number | null;
  topical_rate_pct: number | null;
  inert_rate_pct: number | null;
  evidence_without_document: number;
};

/** One Document's Evidence yield and how much of it is wired into the case. */
export type DocumentConnection = {
  document_id: string;
  /** The Document node's `title` property. Absent if the node carries none. */
  title?: string;
  /** The Document node's `doc_type` property. Absent if the node carries none. */
  doc_type?: string;
  evidence_total: number;
  probative_connected: number;
  topical_connected: number;
  inert: number;
  probative_rate_pct: number | null;
  topical_rate_pct: number | null;
  inert_rate_pct: number | null;
  /** Parallel to the response's `edge_classes`, same order. */
  by_edge_class: EdgeClassCount[];
};

/** One complete measurement — the unit a future snapshot would store. */
export type GraphInventory = {
  computed_at: string;
  corpus: CorpusConnection;
  node_labels: LabelCount[];
  unlabeled_node_count: number;
  edge_triples: EdgeTripleCount[];
  documents: DocumentConnection[];
};

/**
 * The full response body.
 *
 * `previous` is absent today (no history store exists yet) and is declared so
 * the delta view attaches later without re-typing this module.
 */
export type CaseHealthInventoryResponse = {
  case_slug: string;
  connection_tier_lookup_v: number;
  /** Edge-class column list, in display order. The backend owns this vocabulary. */
  edge_classes: string[];
  current: GraphInventory;
  previous?: GraphInventory;
};

/**
 * Fetch the Graph Inventory payload for `slug` (defaults to the single seeded
 * case).
 *
 * Mirrors `getProofMatrixRollup`: validates the load-bearing fields and throws a
 * contextual error at the boundary rather than letting a malformed body crash a
 * component later — Standing Rule 1 (no silent failures). There is deliberately
 * no 404 branch: an empty graph is a legitimate reading that returns 200 with
 * zeroed counts, not an error.
 *
 * @param slug case slug; defaults to {@link DEFAULT_CASE_SLUG}
 * @returns the typed inventory payload
 * @throws Error on non-2xx, unparseable body, or a body missing `current`
 */
export async function getCaseHealthInventory(
  slug: string = DEFAULT_CASE_SLUG,
): Promise<CaseHealthInventoryResponse> {
  // authFetch adds credentials + a 30s timeout (AbortController) — Rule 13.
  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/case-health/inventory`,
  );

  if (!response.ok) {
    throw new Error(
      `Failed to load case health inventory for "${slug}" (HTTP ${response.status}). ` +
        `The graph may be unreachable. Try reloading the page; if it persists, report this to the site administrator.`,
    );
  }

  let data: unknown;
  try {
    data = await response.json();
  } catch {
    throw new Error(
      `Case health inventory response for "${slug}" was not valid JSON (the backend may be down). Try reloading the page.`,
    );
  }

  const parsed = data as Partial<CaseHealthInventoryResponse>;
  // `current` and `edge_classes` are what every section of the page reads. A
  // body missing either is a backend/frontend contract mismatch, not a
  // renderable state — so it fails loudly here, once, with a message naming the
  // problem, instead of surfacing as an undefined-property crash deep in a row.
  if (
    parsed.current === undefined ||
    parsed.current === null ||
    !Array.isArray(parsed.current.documents) ||
    !Array.isArray(parsed.edge_classes)
  ) {
    throw new Error(
      `Case health inventory response for "${slug}" is missing its "current" measurement or ` +
        `"edge_classes" list — backend/frontend contract mismatch. If reloading does not help, report this to the site administrator.`,
    );
  }

  return parsed as CaseHealthInventoryResponse;
}

/**
 * Format a nullable rate for display.
 *
 * ## React/TS Learning: a pure formatter is not business logic
 *
 * This performs no arithmetic — it renders the number the backend already
 * computed, and turns `null` into an em-dash. That distinction matters: `null`
 * means "there was nothing to measure" (a document that produced no Evidence),
 * which is a genuinely different statement from "0.0%" ("it produced Evidence
 * and none of it connected"). Showing "0.0%" for both would erase the
 * difference the backend went to the trouble of preserving.
 *
 * Kept pure and out of the component so it is trivially unit-testable, matching
 * `indexAllegationTotals` in `proofMatrix.ts`.
 *
 * @param rate the backend-computed percentage, or `null`
 * @returns e.g. `"19.6%"`, or `"—"` when there was nothing to measure
 */
export function formatRate(rate: number | null | undefined): string {
  if (rate === null || rate === undefined) {
    return "—";
  }
  return `${rate.toFixed(1)}%`;
}

/**
 * Look up one edge class's item count on a document row.
 *
 * The backend ships `edge_classes` (the column order) and each row's
 * `by_edge_class` in that same order, but a table cell is addressed by class
 * NAME, so this resolves the name to its count without the component indexing
 * by position — which would silently render the wrong column if the two ever
 * came apart.
 *
 * Returns `null` — not `0` — when the row carries no entry for the class, so a
 * contract mismatch shows as an em-dash rather than a fabricated zero.
 *
 * @param row the document row
 * @param relType the edge-class name from `edge_classes`
 * @returns the item count, or `null` if this row has no entry for that class
 */
export function edgeClassCount(
  row: DocumentConnection,
  relType: string,
): number | null {
  const entry = row.by_edge_class.find((c) => c.rel_type === relType);
  return entry ? entry.item_count : null;
}
