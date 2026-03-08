import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

export interface QAEntrySummary {
  id: string;
  scope_type: string;
  scope_id: string;
  session_id: string | null;
  question_preview: string;
  asked_by: string;
  asked_at: string;
  model: string;
  user_rating: number | null;
  parent_qa_id: string | null;
  total_ms: number | null;
}

export async function getQAHistory(
  scopeType: string,
  scopeId: string,
  limit = 20
): Promise<QAEntrySummary[]> {
  const resp = await authFetch(
    `${API_BASE_URL}/api/qa-history?scope_type=${scopeType}&scope_id=${scopeId}&limit=${limit}`
  );
  if (!resp.ok) throw new Error(`qa-history failed: ${resp.status}`);
  return resp.json();
}

// Fire-and-forget — rating failures are silent
export async function rateQAEntry(id: string, rating: number): Promise<void> {
  try {
    await authFetch(`${API_BASE_URL}/api/qa/${id}/rate`, {
      method: "PATCH",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ rating }),
    });
  } catch {
    // intentionally silent
  }
}
