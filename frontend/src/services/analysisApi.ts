// =============================================================================
// analysisApi.ts — client for GET /api/analysis
// -----------------------------------------------------------------------------
// The per-Allegation evidence tally behind the Case Evidence & Analysis browser
// (`/explorer`). Mirrors the Rust DTO (`dto::analysis`).
//
// ## What this no longer carries
//
// Until the 2026-07-27 honesty batch the payload also carried a `strength_percent`
// and `strength_category` per allegation, plus `strong_evidence` / `moderate_evidence`
// / `weak_evidence` / `gaps` bucket counts. The percentage came from a five-row
// lookup table on the evidence count (0 → 25%, 1 → 60%, 2 → 80%, 3 → 90%,
// 4+ → 95%), so it told the reader nothing the count does not while looking like a
// measurement. It is retired, not replaced — a real coverage verdict is Case State
// work, and inventing a substitute here would repeat the mistake.
//
// `contradictions_summary` and `evidence_coverage` went with the `/analysis` page
// that was the only thing rendering them: the first duplicated
// `GET /api/contradictions`, and the second was a third per-document
// "evidence produced vs. linked" table whose definition of *linked* matched
// neither of the other two in the product. Case Health Pane 1 is the surviving
// per-document connection surface.
// =============================================================================

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

/**
 * One Allegation's evidence tally.
 *
 * The type name is kept from before the strength retirement so the change stayed
 * confined to what was false; `supporting_evidence_count` is a measured count of
 * distinct evidence items reaching the allegation, not a score.
 */
export type AllegationStrength = {
  id: string;
  allegation?: string;
  paragraph?: string;
  supporting_evidence_count: number;
  supporting_evidence?: string[];
};

export type GapAnalysis = {
  total_allegations: number;
  allegations: AllegationStrength[];
};

export type AnalysisResponse = {
  gap_analysis: GapAnalysis;
};

export async function getAnalysis(): Promise<AnalysisResponse> {
  const response = await authFetch(`${API_BASE_URL}/api/analysis`);

  if (!response.ok) {
    throw new Error(`Failed to fetch analysis: ${response.status}`);
  }

  return response.json();
}
