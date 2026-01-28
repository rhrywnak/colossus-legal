import { API_BASE_URL } from "./api";

export type HarmDto = {
  id: string;
  title: string;
  category?: string;
  subcategory?: string;
  amount?: number;
  description?: string;
  date?: string;
  source_reference?: string;
  caused_by_allegations: string[];
  damages_for_counts: string[];
};

export type HarmsResponse = {
  harms: HarmDto[];
  total: number;
  total_damages: number;
  by_category: Record<string, number>;
};

export async function getHarms(): Promise<HarmsResponse> {
  const response = await fetch(`${API_BASE_URL}/harms`);

  if (!response.ok) {
    throw new Error(`Failed to fetch harms: ${response.status}`);
  }

  return response.json();
}
