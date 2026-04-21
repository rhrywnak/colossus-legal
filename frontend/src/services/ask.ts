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

export type RetrievalDetail = {
    node_id: string;
    node_type: string;
    title: string;
    score: number;
    origin: "qdrant" | "graph";
    document_title?: string;
    document_id?: string;
    page_number?: number;
    quote_preview?: string;
    relationship_count: number;
};

export type AnswerSource = {
    document_id: string;
    document_title: string;
    page_number?: number;
    evidence_title: string;
    node_id: string;
};

export type AskResponse = {
    question: string;
    answer: string;
    provider: string;
    retrieval_stats: RetrievalStats;
    qa_id?: string;
    strategy?: string;
    retrieval_details?: RetrievalDetail[];
    sources?: AnswerSource[];
};

export type ChatModel = {
    model_id: string;
    display_name: string;
    is_default: boolean;
};

export type ChatModelsResponse = {
    models: ChatModel[];
    default_model: string;
};

/** Fetch the active-model catalog from the backend.
 *
 * The Chat model picker is populated from this response. On any failure
 * (network, 5xx, schema drift) we fall back to a single Sonnet 4.6 entry
 * so Chat still works — the backend also treats an absent `model` field
 * as "use the default", so this fallback is additionally safe against
 * new backends that don't yet ship `/api/chat/models`.
 */
export async function fetchChatModels(): Promise<ChatModelsResponse> {
    try {
        const response = await authFetch(`${API_BASE_URL}/api/chat/models`);
        if (!response.ok) {
            throw new Error(`Failed to fetch models: ${response.status}`);
        }
        return response.json();
    } catch {
        return {
            models: [
                { model_id: "claude-sonnet-4-6", display_name: "Claude Sonnet 4.6", is_default: true },
            ],
            default_model: "claude-sonnet-4-6",
        };
    }
}

export async function askTheCase(
    question: string,
    parentQaId?: string | null,
    model?: string | null,
): Promise<AskResponse> {
    const body: Record<string, string> = { question };
    if (parentQaId) body.parent_qa_id = parentQaId;
    if (model) body.model = model;

    const response = await authFetch(`${API_BASE_URL}/api/ask`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
        timeoutMs: 120000,  // 2 minutes for RAG synthesis
    });
    if (!response.ok) {
        const body = await response.text();
        throw new Error(`Ask failed (${response.status}): ${body}`);
    }
    return response.json();
}
