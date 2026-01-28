import { API_BASE_URL } from "./api";

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
};

export type ContradictionsResponse = {
  contradictions: ContradictionDto[];
  total: number;
};

export async function getContradictions(): Promise<ContradictionsResponse> {
  const response = await fetch(`${API_BASE_URL}/contradictions`);

  if (!response.ok) {
    throw new Error(`Failed to fetch contradictions: ${response.status}`);
  }

  return response.json();
}
