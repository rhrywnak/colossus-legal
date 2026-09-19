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
 * ## ⚑ IT NO LONGER INVENTS A MODEL ON FAILURE (2026-09-19)
 *
 * This used to `catch` and return a single hand-written entry — `claude-sonnet-4-6`,
 * flagged `is_default: true`. That was the SAME defect as the backend constant
 * this task removed, in the other half of the stack, and it was worse in one
 * respect: it presented a fabricated catalogue as a real one, so a picker
 * showing one model looked like a deployment that offered one model rather than
 * like a page that could not reach its server. When that id was deactivated in
 * the Admin list the fallback went on naming it, and every ask made through it
 * answered 400.
 *
 * So the failure THROWS, and the page says it could not load the catalogue
 * (Standing Rule 1). Chat still works while it is down: an absent `model` field
 * means "use the server default", which is now a stored row the backend refuses
 * to boot without.
 *
 * @throws when the request fails, the status is not ok, or the body is not the
 *         shape this build expects — each with a sentence naming which.
 */
export async function fetchChatModels(): Promise<ChatModelsResponse> {
    const response = await authFetch(`${API_BASE_URL}/api/chat/models`);
    if (!response.ok) {
        throw new Error(`Failed to load the chat models (HTTP ${response.status}).`);
    }
    const parsed = (await response.json()) as Partial<ChatModelsResponse>;
    // An EMPTY list is legitimate — a deployment with no active Anthropic row —
    // but it must be an array, and the default must be a string. `undefined`
    // here would put `undefined` into the picker's value and send it as the
    // selected model on the next ask.
    if (!Array.isArray(parsed.models) || typeof parsed.default_model !== "string") {
        throw new Error(
            "The chat models response is missing models/default_model — " +
                "backend/frontend contract mismatch. Report it to the site administrator.",
        );
    }
    return { models: parsed.models, default_model: parsed.default_model };
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
