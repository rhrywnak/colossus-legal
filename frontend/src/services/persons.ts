import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

export type PersonDto = {
  id: string;
  name: string;
  role?: string;
  description?: string;
};

export type PersonsResponse = {
  persons: PersonDto[];
  total: number;
};

export async function getPersons(): Promise<PersonsResponse> {
  const response = await authFetch(`${API_BASE_URL}/api/persons`);

  if (!response.ok) {
    throw new Error(`Failed to fetch persons: ${response.status}`);
  }

  return response.json();
}
