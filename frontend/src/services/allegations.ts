import { API_BASE_URL } from "./api";

export type AllegationDto = {
  id: string;
  paragraph?: string;
  title: string;
  allegation?: string;
  evidence_status?: string;
  category?: string;
  severity?: number;
  legal_counts?: string[];
};

export type AllegationSummary = {
  proven: number;
  partial: number;
  unproven: number;
};

export type AllegationsResponse = {
  allegations: AllegationDto[];
  total: number;
  summary: AllegationSummary;
};

export async function getAllegations(): Promise<AllegationsResponse> {
  const response = await fetch(`${API_BASE_URL}/allegations`);

  if (!response.ok) {
    throw new Error(`Failed to fetch allegations: ${response.status}`);
  }

  return response.json();
}
