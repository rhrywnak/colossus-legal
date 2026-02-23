import { API_BASE_URL } from "./api";

export type SearchHit = {
    node_id: string;
    node_type: string;
    title: string;
    score: number;
    document_id: string | null;
    page_number: string | null;
};

export type SearchResponse = {
    query: string;
    results: SearchHit[];
    total: number;
    duration_ms: number;
};

export async function semanticSearch(
    query: string,
    limit?: number,
    nodeTypes?: string[],
): Promise<SearchResponse> {
    const response = await fetch(`${API_BASE_URL}/search`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
            query,
            limit: limit ?? 10,
            node_types: nodeTypes?.length ? nodeTypes : undefined,
        }),
    });
    if (!response.ok) {
        throw new Error(`Search failed: ${response.status}`);
    }
    return response.json();
}
