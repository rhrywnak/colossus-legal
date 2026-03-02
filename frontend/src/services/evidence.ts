import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

export type EvidenceDto = {
  id: string;
  exhibit_number?: string;
  title?: string;
  question?: string;
  answer?: string;
  kind?: string;
  weight?: number;
  page_number?: number;
  significance?: string;
  verbatim_quote?: string;
  stated_by?: string;
  document_id?: string;
  document_title?: string;
};

export type EvidenceResponse = {
  evidence: EvidenceDto[];
  total: number;
  by_kind: Record<string, number>;
};

export async function getEvidence(): Promise<EvidenceResponse> {
  const response = await authFetch(`${API_BASE_URL}/evidence`);

  if (!response.ok) {
    throw new Error(`Failed to fetch evidence: ${response.status}`);
  }

  return response.json();
}
