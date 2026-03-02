import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

export type Claim = {
  id: string;
  title: string;
  description?: string;
  status: string;
};

const stubClaims: Claim[] = [
  {
    id: "claim-1",
    title: "Breach of contract",
    description: "Alleged failure to deliver goods as agreed.",
    status: "open",
  },
  {
    id: "claim-2",
    title: "Negligence",
    description: "Failure to exercise reasonable care causing damages.",
    status: "pending",
  },
];

export async function getClaims(): Promise<Claim[]> {
  const response = await authFetch(`${API_BASE_URL}/claims`);

  if (!response.ok) {
    throw new Error(`Failed to fetch claims: ${response.status}`);
  }

  let data: unknown;
  try {
    data = await response.json();
  } catch (error) {
    throw new Error("Failed to parse claims response");
  }

  if (!Array.isArray(data)) {
    throw new Error("Invalid claims response shape");
  }

  return data as Claim[];
}

export async function getClaimsStub(): Promise<Claim[]> {
  // Simulate async fetch; can be extended to throw for error-state testing.
  return Promise.resolve(stubClaims);
}
