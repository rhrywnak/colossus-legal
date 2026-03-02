import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

export type PersonInfo = {
  id: string;
  name: string;
  role?: string;
};

export type PersonSummary = {
  total_statements: number;
  documents_count: number;
  characterizations_count: number;
  rebuttals_received_count: number;
};

export type CharacterizesInfo = {
  allegation_id: string;
  allegation_text?: string;
  characterization_label?: string;
};

export type RebuttalInfo = {
  evidence_id: string;
  title?: string;
  verbatim_quote?: string;
  stated_by?: string;
  document_title?: string;
};

export type StatementDetail = {
  evidence_id: string;
  title: string;
  verbatim_quote?: string;
  page_number?: number;
  kind?: string;
  significance?: string;
  characterizes: CharacterizesInfo[];
  rebutted_by: RebuttalInfo[];
};

export type DocumentGroup = {
  document_id: string;
  document_title: string;
  statement_count: number;
  statements: StatementDetail[];
};

export type PersonDetailResponse = {
  person: PersonInfo;
  summary: PersonSummary;
  documents: DocumentGroup[];
};

export async function getPersonDetail(
  personId: string,
): Promise<PersonDetailResponse> {
  const response = await authFetch(
    `${API_BASE_URL}/persons/${encodeURIComponent(personId)}/detail`,
  );

  if (!response.ok) {
    if (response.status === 404) {
      throw new Error("Person not found");
    }
    throw new Error(`Failed to fetch person detail: ${response.status}`);
  }

  return response.json();
}
