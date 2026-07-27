import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

// =============================================================================
// Endpoint 1: GET /allegations/:id/detail — deep dive into one allegation
//
// The former `GET /decomposition` overview client was removed on 2026-07-27 with
// the unreachable page that was its only caller: its summary reported a
// `proven_count` / `all_proven` pair the backend derived from an allegation
// `status` property the v5.1 migration had dropped, so both were permanently
// zero/false by construction.
// =============================================================================

export type AllegationInfo = {
  id: string;
  title: string;
  description?: string;
  status?: string;
  legal_counts: string[];
};

export type RebuttalDetail = {
  evidence_id: string;
  topic?: string;
  verbatim_quote?: string;
  page_number?: string;
  document?: string;
  stated_by?: string;
};

export type CharacterizationDetail = {
  label: string;
  evidence_id: string;
  verbatim_quote?: string;
  page_number?: string;
  document?: string;
  stated_by?: string;
  rebuttals: RebuttalDetail[];
};

export type ProofClaimSummary = {
  id: string;
  title: string;
  category?: string;
  evidence_count: number;
};

export type AllegationDetailResponse = {
  allegation: AllegationInfo;
  characterizations: CharacterizationDetail[];
  proof_claims: ProofClaimSummary[];
};

// =============================================================================
// Endpoint 2: GET /rebuttals — George's claims grouped with rebuttals
// =============================================================================

export type GeorgeClaimWithRebuttals = {
  claim_id: string;
  claim_title: string;
  george_quote?: string;
  document?: string;
  rebuttals: RebuttalDetail[];
  rebuttal_count: number;
};

export type UnrebuttedReason = {
  claim: string;
  reason: string;
};

export type RebuttalsSummary = {
  total_george_claims_rebutted: number;
  total_george_claims_unrebutted: number;
  total_rebuttals: number;
  unrebutted_reasons: UnrebuttedReason[];
};

export type RebuttalsResponse = {
  george_claims: GeorgeClaimWithRebuttals[];
  summary: RebuttalsSummary;
};

// =============================================================================
// Fetch functions
// =============================================================================

export async function getAllegationDetail(
  allegationId: string
): Promise<AllegationDetailResponse> {
  const response = await authFetch(
    `${API_BASE_URL}/api/allegations/${encodeURIComponent(allegationId)}/detail`
  );

  if (!response.ok) {
    if (response.status === 404) {
      throw new Error("Allegation not found");
    }
    throw new Error(`Failed to fetch allegation detail: ${response.status}`);
  }

  return response.json();
}

export async function getRebuttals(): Promise<RebuttalsResponse> {
  const response = await authFetch(`${API_BASE_URL}/api/rebuttals`);

  if (!response.ok) {
    throw new Error(`Failed to fetch rebuttals: ${response.status}`);
  }

  return response.json();
}
