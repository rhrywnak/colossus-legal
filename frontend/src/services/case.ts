import { API_BASE_URL } from "./api";

// NOTE: Fields match backend DTO - most are optional
export type CaseInfo = {
  id: string;
  title: string;
  case_number?: string;
  court?: string;
  court_type?: string;
  filing_date?: string;
  status?: string;
  summary?: string;
};

export type PartyDto = {
  id: string;
  name: string;
  type: "person" | "organization";
  description?: string;
};

export type PartiesGroup = {
  plaintiffs: PartyDto[];
  defendants: PartyDto[];
  other: PartyDto[];
};

export type LegalCountSummary = {
  id: string;
  name: string;
};

export type CaseStats = {
  allegations_total: number;
  allegations_proven: number;
  evidence_count: number;
  document_count: number;
  damages_total: number;
  legal_counts: number;
  legal_count_details: LegalCountSummary[];
};

export type CaseResponse = {
  case: CaseInfo;
  parties: PartiesGroup;
  stats: CaseStats;
};

export async function getCase(): Promise<CaseResponse> {
  const response = await fetch(`${API_BASE_URL}/case`);

  if (!response.ok) {
    throw new Error(`Failed to fetch case: ${response.status}`);
  }

  return response.json();
}
