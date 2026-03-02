import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

export type GraphNodeType =
  | "legal_count"
  | "allegation"
  | "motion_claim"
  | "evidence"
  | "document";

export type GraphNode = {
  id: string;
  label: string;
  node_type: GraphNodeType;
  subtitle?: string;
  details?: string;
};

export type GraphEdge = {
  source: string;
  target: string;
  relationship: string;
};

export type GraphResponse = {
  nodes: GraphNode[];
  edges: GraphEdge[];
  title: string;
  hierarchy_type: string;
};

export async function getLegalProofGraph(
  countId?: string
): Promise<GraphResponse> {
  const url = countId
    ? `${API_BASE_URL}/graph/legal-proof?count_id=${encodeURIComponent(countId)}`
    : `${API_BASE_URL}/graph/legal-proof`;

  const response = await authFetch(url);

  if (!response.ok) {
    throw new Error(`Failed to fetch graph: ${response.status}`);
  }

  return response.json();
}
