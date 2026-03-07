import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

export type RetrievalStats = {
    qdrant_hits: number;
    graph_nodes_expanded: number;
    context_tokens: number;
    input_tokens: number;
    output_tokens: number;
    search_ms: number;
    expand_ms: number;
    synthesis_ms: number;
    total_ms: number;
};

export type AskResponse = {
    question: string;
    answer: string;
    provider: string;
    retrieval_stats: RetrievalStats;
};

export async function askTheCase(question: string): Promise<AskResponse> {
    const response = await authFetch(`${API_BASE_URL}/ask`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ question }),
        timeoutMs: 120000,  // 2 minutes for RAG synthesis
    });
    if (!response.ok) {
        const body = await response.text();
        throw new Error(`Ask failed (${response.status}): ${body}`);
    }
    return response.json();
}
