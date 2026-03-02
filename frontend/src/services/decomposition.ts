import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

// =============================================================================
// Endpoint 1: GET /decomposition — Overview of all allegations
// =============================================================================

export type AllegationOverview = {
  id: string;
  title: string;
  description?: string;
  status: string;
  characterizations: string[];
  characterized_by?: string;
  proof_count: number;
  rebuttal_count: number;
};

export type DecompositionSummary = {
  total_allegations: number;
  proven_count: number;
  all_proven: boolean;
  total_characterizations: number;
  total_rebuttals: number;
};

export type DecompositionResponse = {
  allegations: AllegationOverview[];
  summary: DecompositionSummary;
};

// =============================================================================
// Endpoint 2: GET /allegations/:id/detail — Deep dive into one allegation
// =============================================================================

export type AllegationInfo = {
  id: string;
  title: string;
  description?: string;
  status: string;
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
// Endpoint 3: GET /rebuttals — George's claims grouped with rebuttals
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

export async function getDecomposition(): Promise<DecompositionResponse> {
  const response = await authFetch(`${API_BASE_URL}/decomposition`);

  if (!response.ok) {
    throw new Error(`Failed to fetch decomposition: ${response.status}`);
  }

  return response.json();
}

export async function getAllegationDetail(
  allegationId: string
): Promise<AllegationDetailResponse> {
  const response = await authFetch(
    `${API_BASE_URL}/allegations/${encodeURIComponent(allegationId)}/detail`
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
  const response = await authFetch(`${API_BASE_URL}/rebuttals`);

  if (!response.ok) {
    throw new Error(`Failed to fetch rebuttals: ${response.status}`);
  }

  return response.json();
}
