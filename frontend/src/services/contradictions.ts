import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

export type ContradictionEvidence = {
  id: string;
  title?: string;
  answer?: string;
  document_title?: string;
};

export type ContradictionDto = {
  evidence_a: ContradictionEvidence;
  evidence_b: ContradictionEvidence;
  description?: string;
  topic?: string;
  impeachment_value?: string;
  earlier_claim?: string;
  later_admission?: string;
};

export type ContradictionsResponse = {
  contradictions: ContradictionDto[];
  total: number;
};

export async function getContradictions(): Promise<ContradictionsResponse> {
  const response = await authFetch(`${API_BASE_URL}/api/contradictions`);

  if (!response.ok) {
    throw new Error(`Failed to fetch contradictions: ${response.status}`);
  }

  return response.json();
}
