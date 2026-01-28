import { API_BASE_URL } from "./api";

export type ChainDocument = {
  id: string;
  title: string;
  page_number?: number;
};

export type EvidenceWithDocument = {
  id: string;
  title: string;
  question?: string;
  answer?: string;
  document?: ChainDocument;
};

export type MotionClaimWithEvidence = {
  id: string;
  title: string;
  evidence: EvidenceWithDocument[];
};

export type ChainAllegation = {
  id: string;
  title: string;
  paragraph?: string;
  evidence_status?: string;
  legal_counts?: string[];
};

export type ChainSummary = {
  motion_claim_count: number;
  evidence_count: number;
  document_count: number;
};

export type EvidenceChainResponse = {
  allegation: ChainAllegation;
  motion_claims: MotionClaimWithEvidence[];
  summary: ChainSummary;
};

export async function getEvidenceChain(
  allegationId: string
): Promise<EvidenceChainResponse> {
  const response = await fetch(
    `${API_BASE_URL}/allegations/${encodeURIComponent(allegationId)}/evidence-chain`
  );

  if (!response.ok) {
    throw new Error(`Failed to fetch evidence chain: ${response.status}`);
  }

  return response.json();
}
