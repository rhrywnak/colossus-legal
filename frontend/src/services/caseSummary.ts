import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

export type PersonCharacterizationCount = {
  person: string;
  count: number;
};

// One Element of proof attached to a LegalCount. Mirrors the backend
// `ElementInfo` DTO. `controlling_authority` is absent (undefined) on the
// wire when the graph property is null — the frontend treats absent/empty
// the same way (placeholder text in the popover), but the InfoPopup icon
// is always rendered so "missing" and "pending" remain distinguishable.
export type ElementInfo = {
  id: string;
  element_name: string;
  title: string;
  order_in_count: number | null;
  allegation_count: number;
  controlling_authority?: string;
};

export type LegalCountInfo = {
  id: string;
  name: string;
  count_number: number;
  allegation_count: number;
  elements: ElementInfo[];
};

export type CaseSummaryResponse = {
  case_title: string;
  court?: string;
  case_number?: string;

  allegations_total: number;
  allegations_proven: number;
  legal_counts: number;
  legal_count_details: LegalCountInfo[];

  damages_total: number;
  damages_financial: number;
  damages_reputational_count: number;
  harms_total: number;

  characterizations_total: number;
  characterizations_by_person: PersonCharacterizationCount[];
  rebuttals_total: number;
  unique_characterization_labels: string[];

  evidence_total: number;
  evidence_grounded: number;
  documents_total: number;

  plaintiffs: string[];
  defendants: string[];
};

export async function getCaseSummary(): Promise<CaseSummaryResponse> {
  const response = await authFetch(`${API_BASE_URL}/api/case-summary`);

  if (!response.ok) {
    throw new Error(`Failed to fetch case summary: ${response.status}`);
  }

  return response.json();
}
