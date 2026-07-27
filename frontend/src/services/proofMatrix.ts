// =============================================================================
// proofMatrix.ts — client for GET /api/cases/:slug/proof-matrix/rollup
// -----------------------------------------------------------------------------
// Endpoint: GET /api/cases/:slug/proof-matrix/rollup → backend handler
// `api::proof_matrix`. Neo4j-backed, read-only. Returns, per LegalCount, the
// count of DISTINCT Allegations bearing on that Count's Elements — the single
// source of truth for Count-level deduped allegation totals.
//
// These interfaces mirror the Rust DTO (`dto::proof_matrix`) exactly. This is a
// SEPARATE endpoint from causes-of-action: the deduped per-Count total is NOT
// the same number as summing the per-Element `allegation_count` fields that
// causes-of-action returns (an Allegation bearing on two Elements of one Count
// is counted once here, twice there). We therefore keep this in its own service
// and its own type — never merged onto `CountDetail`, which mirrors a different
// wire DTO.
// =============================================================================

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { DEFAULT_CASE_SLUG } from "./caseHeader";

/** One Count's deduped allegation total. `count_number` is the join key. */
export type CountRollup = {
  count_number: number;
  count_id: string;
  deduped_allegations: number;
};

/** Top-level payload: the echoed slug and the per-Count rollup rows. */
export type ProofMatrixRollupResponse = {
  case_slug: string;
  counts: CountRollup[];
};

// ─── Structural-column types (PM4) ──────────────────────────────────────────
//
// `EvidenceRef` used to live here: a placeholder shape for "what the evidence
// columns will hold once discovery is processed", declared while both columns
// were fed empty arrays. It was deleted on 2026-07-27 with the last thing that
// rendered it. The anticipated shape never became the real one — the Supporting
// and Disputes columns show backend-computed COUNTS, and the items behind them
// come from the element-detail endpoint as `AllegationEvidence`
// (services/elementDetailService.ts), a different shape carrying the source
// locator the placeholder lacked.
//
// Worth remembering rather than just deleting: the placeholder outlived its
// premise. It was defined on the assumption that zero Evidence nodes existed,
// and stayed after that stopped being true — which is why the Disputes column
// sat blank over 41 real REBUTS edges. Shapes designed ahead of their data need
// a reason to be revisited, not just a comment saying Stage 2 will swap them.

/**
 * Proof status for one Element — the EXACT lowercase values the backend's
 * `derive_proof_status` emits (Part 2 aligned these to the wire). Computed by
 * the backend from coverage; the frontend only renders it (Rule 19 — no
 * client-side status derivation).
 *
 * Domain note: this is **presence-of-evidence**, NOT legal sufficiency — there
 * is intentionally no `"proven"`. `"no_allegations"` (nothing mapped to the
 * Element) is distinct from `"gap"` (allegations mapped, none corroborated).
 */
export type ElementProofStatus =
  | "no_allegations"
  | "gap"
  | "partial"
  | "supported";

/**
 * Fetch the per-Count deduped allegation rollup for `slug` (defaults to the
 * single seeded case).
 *
 * Mirrors `getCausesOfAction`: validates the load-bearing field (`counts` is an
 * array) and throws a contextual error at the boundary rather than letting a
 * malformed body crash a component later — Standing Rule 1 (no silent failures).
 *
 * @param slug case slug; defaults to {@link DEFAULT_CASE_SLUG}
 * @returns the typed proof-matrix rollup payload
 * @throws Error on non-2xx, unparseable body, or a body missing `counts`
 */
export async function getProofMatrixRollup(
  slug: string = DEFAULT_CASE_SLUG,
): Promise<ProofMatrixRollupResponse> {
  // authFetch adds credentials + a 30s timeout (AbortController) — Rule 13.
  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/proof-matrix/rollup`,
  );

  if (!response.ok) {
    // 404 here means the canonical case structure (Counts / Elements / bearing
    // Allegations) hasn't been loaded into Neo4j — same contract as the
    // causes-of-action endpoint.
    const reason =
      response.status === 404
        ? " — case structure not loaded (run the canonical Element loader)"
        : "";
    throw new Error(
      `Failed to load proof-matrix rollup for "${slug}" (HTTP ${response.status}${reason}). Try reloading the page.`,
    );
  }

  let data: unknown;
  try {
    data = await response.json();
  } catch {
    throw new Error(
      `Proof-matrix rollup response for "${slug}" was not valid JSON (the backend may be down). Try reloading the page.`,
    );
  }

  const parsed = data as Partial<ProofMatrixRollupResponse>;
  if (!Array.isArray(parsed.counts)) {
    throw new Error(
      `Proof-matrix rollup response for "${slug}" is missing the "counts" array — ` +
        `backend/frontend contract mismatch. If reloading does not help, report this to the site administrator.`,
    );
  }

  return parsed as ProofMatrixRollupResponse;
}

/**
 * Re-key the rollup rows into a `count_number → deduped_allegations` lookup so
 * Home can match each Count card to its total in O(1).
 *
 * ## React/TS Learning: a pure data-shaping helper (not business logic)
 * This is a verbatim re-keying of values the endpoint already computed — no
 * summing, no math, no per-Element reasoning (Standing Rule: no business logic
 * in the frontend; the backend's `deduped_allegations` is displayed as-is). It
 * is kept pure (same input → same output, no side effects) so it is trivially
 * unit-testable and stays out of the component, mirroring how Home consumes the
 * `count_descriptions` map.
 *
 * @param counts the rollup rows
 * @returns a record keyed by `count_number` whose values are the deduped totals
 */
export function indexAllegationTotals(
  counts: CountRollup[],
): Record<number, number> {
  const totals: Record<number, number> = {};
  for (const row of counts) {
    totals[row.count_number] = row.deduped_allegations;
  }
  return totals;
}
