// =============================================================================
// proofReview.ts — client for GET /api/cases/:slug/proof-review
// -----------------------------------------------------------------------------
// Endpoint: GET /api/cases/:slug/proof-review (optional ?document_id=) → backend
// handler `api::proof_review`. Neo4j-backed, read-only. Returns ONE payload with
// four sections (summary, proof_edges, excluded, borderline) over the
// Evidence-[:CORROBORATES]->Allegation proof edges.
//
// These types mirror the Rust DTO (`dto::proof_review`) byte-for-byte: snake_case
// keys, nullable fields typed `T | null` (the backend always emits the key, null
// when absent — never omitted), and required categorization fields as plain
// `string`. The frontend renders these labeled rows as-is and holds NO business
// logic — all counts/grouping were computed by the backend (Standing Rule:
// no business logic in the frontend).
// =============================================================================

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { DEFAULT_CASE_SLUG } from "./caseHeader";

/** One per-`statement_type` tally (e.g. admission → 24). */
export type StatementTypeCount = {
  statement_type: string;
  count: number;
};

/** One per-(`statement_type`, `evidence_strength`) tally. */
export type CategoryCount = {
  statement_type: string;
  evidence_strength: string;
  count: number;
};

/** Corroboration counts at two granularities, both over the same edge set. */
export type CorroboratingSummary = {
  total: number;
  by_statement_type: StatementTypeCount[];
  by_category: CategoryCount[];
};

/** Exclusion counts: preserved non-answers kept out of the corroboration set. */
export type ExcludedSummary = {
  total: number;
  by_statement_type: StatementTypeCount[];
};

/** Sub-view 1: the two summary breakdowns, derived by the backend. */
export type ProofReviewSummary = {
  corroborating: CorroboratingSummary;
  excluded: ExcludedSummary;
};

/**
 * One `CORROBORATES` proof edge — the discovery answer and the complaint
 * allegation it corroborates. Used by both `proof_edges` and `borderline`
 * (borderline is the `partial_admission` subset). `statement_type` /
 * `evidence_strength` are always present; every display/locator field is
 * nullable (the backend emits `null`, never omits the key).
 */
export type ProofEdge = {
  answer: string | null;
  question: string | null;
  evidence_verbatim_quote: string | null;
  statement_type: string;
  evidence_strength: string;
  paragraph: string | null;
  page_number: number | null;
  source_document: string | null;
  allegation_summary: string | null;
  allegation_title: string | null;
  allegation_paragraph_number: string | null;
  allegation_id: string | null;
};

/** Sub-view 3: a preserved non-answer with no `CORROBORATES` edge. */
export type ExcludedEvidence = {
  answer: string | null;
  question: string | null;
  evidence_verbatim_quote: string | null;
  statement_type: string;
  paragraph: string | null;
  page_number: number | null;
  source_document: string | null;
};

/** Top-level payload: echoed scope + the four sub-view sections. */
export type ProofReviewResponse = {
  case_slug: string;
  document_id: string | null;
  summary: ProofReviewSummary;
  proof_edges: ProofEdge[];
  excluded: ExcludedEvidence[];
  borderline: ProofEdge[];
};

/**
 * Fetch the Proof-Review payload for `slug`, optionally scoped to one source
 * document.
 *
 * Mirrors `getProofMatrixRollup`: validates the load-bearing fields and throws a
 * contextual error at the boundary rather than letting a malformed body crash a
 * component later (Standing Rule 1 — no silent failures). Every failure path
 * (non-2xx, unparseable body, missing section) produces a distinct message.
 *
 * @param slug case slug; defaults to {@link DEFAULT_CASE_SLUG}
 * @param documentId when set, scopes every section to that one source document
 *   (the backend's `?document_id=` filter); when omitted, all documents
 * @returns the typed Proof-Review payload
 * @throws Error on non-2xx, unparseable body, or a body missing a section
 */
export async function getProofReview(
  slug: string = DEFAULT_CASE_SLUG,
  documentId?: string,
): Promise<ProofReviewResponse> {
  // Build the URL; only append the filter when a document is selected so the
  // "all documents" call is a bare path (matching the backend's None branch).
  const base = `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/proof-review`;
  const url = documentId
    ? `${base}?document_id=${encodeURIComponent(documentId)}`
    : base;

  // authFetch adds credentials + a 30s timeout (AbortController) — Rule 13.
  const response = await authFetch(url);

  if (!response.ok) {
    // 404 here means no proof-review data exists for this scope (no corroborating
    // edges and no preserved non-answers) — same 404 contract as the rollup.
    const reason =
      response.status === 404
        ? " — no proof-review data loaded for this case/document"
        : "";
    throw new Error(
      `Failed to load proof review for "${slug}" (HTTP ${response.status}${reason}). Try reloading the page.`,
    );
  }

  let data: unknown;
  try {
    data = await response.json();
  } catch {
    throw new Error(
      `Proof-review response for "${slug}" was not valid JSON (the backend may be down). Try reloading the page.`,
    );
  }

  const parsed = data as Partial<ProofReviewResponse>;
  // Validate each load-bearing section so a contract mismatch fails here with a
  // clear message, not as an "undefined.map" deep inside a sub-view later.
  if (
    !parsed.summary ||
    !Array.isArray(parsed.proof_edges) ||
    !Array.isArray(parsed.excluded) ||
    !Array.isArray(parsed.borderline)
  ) {
    throw new Error(
      `Proof-review response for "${slug}" is missing a required section ` +
        `(summary / proof_edges / excluded / borderline) — backend/frontend contract ` +
        `mismatch. If reloading does not help, report this to the site administrator.`,
    );
  }

  return parsed as ProofReviewResponse;
}
