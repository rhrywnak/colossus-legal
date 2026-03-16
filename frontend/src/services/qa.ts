import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { AskResponse } from "./ask";

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

export interface QAEntryFull {
  id: string;
  scope_type: string;
  scope_id: string;
  session_id: string | null;
  question: string;
  answer: string;
  asked_by: string;
  asked_at: string;
  model: string;
  rating: string | null;
  rating_by: string | null;
  parent_qa_id: string | null;
  metadata: {
    context_tokens: number;
    expand_ms: number;
    graph_nodes_expanded: number;
    input_tokens: number;
    output_tokens: number;
    qdrant_hits: number;
    search_ms: number;
    synthesis_ms: number;
    total_ms: number;
  } | null;
}

export async function getQAEntry(id: string): Promise<QAEntryFull> {
  const resp = await authFetch(`${API_BASE_URL}/api/qa/${id}`);
  if (!resp.ok) throw new Error(`qa entry fetch failed: ${resp.status}`);
  return resp.json();
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

/** Map a full QAEntry (from history) to AskResponse shape for AnswerDisplay. */
export function mapEntryToResponse(entry: QAEntryFull): AskResponse {
  const m = entry.metadata;
  return {
    question: entry.question,
    answer: entry.answer,
    provider: entry.model,
    retrieval_stats: {
      qdrant_hits: m?.qdrant_hits ?? 0,
      graph_nodes_expanded: m?.graph_nodes_expanded ?? 0,
      context_tokens: m?.context_tokens ?? 0,
      input_tokens: m?.input_tokens ?? 0,
      output_tokens: m?.output_tokens ?? 0,
      search_ms: m?.search_ms ?? 0,
      expand_ms: m?.expand_ms ?? 0,
      synthesis_ms: m?.synthesis_ms ?? 0,
      total_ms: m?.total_ms ?? 0,
    },
    sources: [],  // Historical entries don't have sources
  };
}

export async function deleteQAEntry(id: string): Promise<boolean> {
  try {
    const resp = await authFetch(`${API_BASE_URL}/api/qa/${id}`, {
      method: "DELETE",
    });
    return resp.ok; // true if 204, false if 403/404
  } catch {
    return false;
  }
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
