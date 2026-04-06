import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

export interface EntityTypeInfo {
  name: string;
  description: string;
}

export interface SchemaResponse {
  total_nodes: number;
  total_relationships: number;
  node_counts: Record<string, number>;
  relationship_counts: Record<string, number>;
  document_type?: string;
  entity_types?: EntityTypeInfo[];
  relationship_types?: EntityTypeInfo[];
}

export async function getSchema(): Promise<SchemaResponse> {
  const response = await authFetch(`${API_BASE_URL}/api/schema`);

  if (!response.ok) {
    throw new Error(`Schema request failed: ${response.status}`);
  }

  return response.json();
}
