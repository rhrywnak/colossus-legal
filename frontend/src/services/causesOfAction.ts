// =============================================================================
// causesOfAction.ts — client for GET /api/cases/:slug/causes-of-action
// -----------------------------------------------------------------------------
// Endpoint: GET /api/cases/:slug/causes-of-action → backend handler
// `api::causes_of_action`. Neo4j-backed, read-only. Returns the case's Counts,
// each with its canonical Elements, for the Home page Causes of Action tables
// (HOME_PAGE_REDESIGN_v2.md §7).
//
// As with the case-header endpoint, these interfaces mirror the Rust DTO
// (`dto::causes_of_action`) exactly. Nullable fields are emitted as JSON `null`
// (present, not omitted), so "absent" stays distinguishable from "empty".
// `count_number` and `allegation_count` are plain numbers (Arabic) — there are
// no Roman-numeral strings on the wire.
// =============================================================================

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { DEFAULT_CASE_SLUG } from "./caseHeader";
import { ElementProofStatus } from "./proofMatrix";

/** A controlling authority (case / statute / jury instruction / court rule). */
export type Authority = {
  citation: string;
  authority_type: string;
  court: string | null;
  year: number | null;
  role: string;
};

/** A doctrinal pleading requirement (e.g. Count IV — abuse of process). */
export type DoctrinalRequirement = {
  requirement: string;
  description: string;
  satisfied_in_case: boolean;
  satisfaction_evidence: string;
};

/** One canonical Element of a Count. `element_id` is the click-through target. */
export type ElementDetail = {
  element_id: string;
  order_in_count: number | null;
  element_name: string;
  what_plaintiff_must_prove: string | null;
  controlling_authority: string | null;
  theory_variant: string | null;
  allegation_count: number;
  /**
   * Proof-Matrix fields (Part 2), mirroring the Rust `ElementDetail` DTO. All
   * three are computed by the backend; the frontend renders them as-is (Rule 19
   * — no client-side derivation).
   *
   * - `supporting_evidence_count`: DISTINCT Evidence corroborating any allegation
   *   bearing on this Element — the Supporting column magnitude.
   * - `covered_allegation_count`: allegations with >=1 corroboration (the
   *   coverage numerator; carried for completeness, not currently rendered).
   * - `proof_status`: the backend-derived coverage label.
   */
  supporting_evidence_count: number;
  covered_allegation_count: number;
  proof_status: ElementProofStatus;
};

/** One Count with its canonical metadata and Elements. */
export type CountDetail = {
  count_number: number;
  count_name: string | null;
  burden_of_proof: string | null;
  m_civ_ji_reference: string | null;
  controlling_authority_primary: string | null;
  controlling_authorities: Authority[];
  doctrinal_requirements: DoctrinalRequirement[] | null;
  chuck_review_required: boolean;
  chuck_review_note: string | null;
  special_note: string | null;
  elements: ElementDetail[];
};

/** Top-level payload: the echoed slug and the case's Counts. */
export type CausesOfActionResponse = {
  case_slug: string;
  counts: CountDetail[];
};

/**
 * Fetch the Counts + Elements for `slug` (defaults to the single seeded case).
 *
 * Mirrors `getCaseHeader`: validates the load-bearing field (`counts` is an
 * array) and throws a contextual error at the boundary rather than letting a
 * malformed body crash a component later — Standing Rule 1 (no silent failures).
 *
 * @param slug case slug; defaults to {@link DEFAULT_CASE_SLUG}
 * @returns the typed causes-of-action payload
 * @throws Error on non-2xx, unparseable body, or a body missing `counts`
 */
export async function getCausesOfAction(
  slug: string = DEFAULT_CASE_SLUG,
): Promise<CausesOfActionResponse> {
  // authFetch adds credentials + a 30s timeout (AbortController) — Rule 13.
  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/causes-of-action`,
  );

  if (!response.ok) {
    // 404 here means the canonical case structure hasn't been loaded into Neo4j.
    const reason =
      response.status === 404
        ? " — case structure not loaded (run the canonical Element loader)"
        : "";
    throw new Error(
      `Failed to load causes of action for "${slug}" (HTTP ${response.status}${reason}). Try reloading the page.`,
    );
  }

  let data: unknown;
  try {
    data = await response.json();
  } catch {
    throw new Error(
      `Causes-of-action response for "${slug}" was not valid JSON (the backend may be down). Try reloading the page.`,
    );
  }

  const parsed = data as Partial<CausesOfActionResponse>;
  if (!Array.isArray(parsed.counts)) {
    throw new Error(
      `Causes-of-action response for "${slug}" is missing the "counts" array — ` +
        `backend/frontend contract mismatch. If reloading does not help, report this to the site administrator.`,
    );
  }

  return parsed as CausesOfActionResponse;
}
