import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

export type MotionClaimDto = {
  id: string;
  title: string;
  claim_text?: string;
  category?: string;
  significance?: string;
  proves_allegations: string[];
  relies_on_evidence: string[];
  source_document_id?: string;
  source_document_title?: string;
};

export type MotionClaimsResponse = {
  motion_claims: MotionClaimDto[];
  total: number;
  by_category: Record<string, number>;
};

export async function getMotionClaims(): Promise<MotionClaimsResponse> {
  const response = await authFetch(`${API_BASE_URL}/api/motion-claims`);

  if (!response.ok) {
    throw new Error(`Failed to fetch motion claims: ${response.status}`);
  }

  return response.json();
}
