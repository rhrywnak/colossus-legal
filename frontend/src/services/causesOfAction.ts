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
  /**
   * The HEADLINE number the matrix leads with (task 396): corroborating items
   * whose (statement_type, evidence_strength) pair maps to the strong tier,
   * counted AFTER near-identical statements have been collapsed.
   *
   * Domain note: strong means what the other side cannot dispute — their own
   * sworn admissions and the court's own findings. `supporting_evidence_count`
   * above is the PRE-COLLAPSE raw magnitude and is deliberately not what this
   * page renders; showing all three at once would put a disagreement on screen
   * that a reader would read as a bug.
   */
  strong_evidence_count: number;
  /**
   * Every corroborating item after collapse, whatever its tier — the "· N
   * approved" depth line beside the headline. Always >= `strong_evidence_count`,
   * which is what makes the pair readable as "this many of these".
   */
  approved_evidence_count: number;
  covered_allegation_count: number;
  /**
   * DISTINCT Evidence items REBUTTING any allegation that bears on this Element
   * — the Disputes column magnitude. Independent of `proof_status`: an Element
   * can be well corroborated AND heavily disputed, and that Element is the one
   * worth arguing about, so the two are shown side by side rather than netted.
   */
  disputing_evidence_count: number;
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
  /**
   * The Proof Matrix's own words, riding this payload because the page GATES on
   * this read — it cannot draw a row without it — and both surfaces that speak
   * them (the row's headline and its drill-down) are on that page.
   *
   * Mirrors the backend `MatrixWordingDto` field for field. There is no fallback
   * vocabulary in this file: a matrix that could not read its words renders the
   * page's error state rather than inventing a column header (Standing Rule 1,
   * and the language law).
   */
  matrix_wording: MatrixWording;
};

/**
 * The eight strings the Proof Matrix speaks, served from the settings store.
 *
 * `raw_approved_template` and `duplicate_template` carry `{count}`; the two
 * fillers live in `components/matrixStrength.ts` beside the rest of the matrix's
 * pure helpers. The frontend composes nothing else here.
 */
export type MatrixWording = {
  strong_column_label: string;
  raw_approved_template: string;
  strong_hint: string;
  tier_strong_chip: string;
  tier_hedged_chip: string;
  tier_other_chip: string;
  duplicate_template: string;
  ranked_list_note: string;
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

  // The matrix cannot draw a column header it does not have. Checked HERE, at
  // the boundary, so a missing wording block is a named error rather than
  // `undefined` rendered where a heading should be — and deliberately NOT
  // defaulted to an English literal, which would be this file inventing the
  // vocabulary the settings store owns.
  if (parsed.matrix_wording == null) {
    throw new Error(
      `Causes-of-action response for "${slug}" is missing "matrix_wording" — ` +
        `backend/frontend contract mismatch. If reloading does not help, report this to the site administrator.`,
    );
  }

  return parsed as CausesOfActionResponse;
}
