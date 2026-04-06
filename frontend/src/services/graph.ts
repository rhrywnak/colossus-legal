import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

/** Node type from the graph API. Now a string to support any schema-defined type. */
export type GraphNodeType = string;

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
    ? `${API_BASE_URL}/api/graph/legal-proof?count_id=${encodeURIComponent(countId)}`
    : `${API_BASE_URL}/api/graph/legal-proof`;

  const response = await authFetch(url);

  if (!response.ok) {
    throw new Error(`Failed to fetch graph: ${response.status}`);
  }

  return response.json();
}
