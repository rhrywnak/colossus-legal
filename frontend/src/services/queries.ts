import { API_BASE_URL } from "./api";

// ─── List endpoint types ─────────────────────────────────────────────────────

export type QueryInfo = {
  id: string;
  title: string;
  description: string;
  category: string;
};

export type QueryCategory = {
  name: string;
  description: string;
  queries: QueryInfo[];
};

export type QueryListResponse = {
  categories: QueryCategory[];
};

// ─── Run endpoint types ──────────────────────────────────────────────────────

export type QueryResultResponse = {
  query_id: string;
  title: string;
  description: string;
  columns: string[];
  rows: Record<string, unknown>[];
  row_count: number;
};

// ─── Fetch functions ─────────────────────────────────────────────────────────

export async function getQueries(): Promise<QueryListResponse> {
  const response = await fetch(`${API_BASE_URL}/queries`);
  if (!response.ok) {
    throw new Error(`Failed to fetch queries: ${response.status}`);
  }
  return response.json();
}

export async function runQuery(id: string): Promise<QueryResultResponse> {
  const response = await fetch(
    `${API_BASE_URL}/queries/${encodeURIComponent(id)}/run`,
  );
  if (!response.ok) {
    if (response.status === 404) {
      throw new Error("Query not found");
    }
    throw new Error(`Failed to run query: ${response.status}`);
  }
  return response.json();
}
