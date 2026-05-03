// =============================================================================
// Bias Explorer service — wire calls for /api/bias/*.
// =============================================================================
//
// Two endpoints:
//   GET  /api/bias/available-filters   — dropdown contents
//   POST /api/bias/query                — filtered Evidence list
//
// The DTO shapes mirror backend/src/bias/dto.rs verbatim. If a field is added
// there, mirror it here and in pages/BiasExplorer/types.ts (which re-exports).
//
// All requests go through `authFetch`, which already enforces a 30s timeout
// via AbortController. Standard error surfacing: throw on non-OK so the page
// can render the message and a Retry button.

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

// ─── DTO mirrors ────────────────────────────────────────────────────────────

export type ActorOption = {
    id: string;
    name: string;
    actor_type: string;
    tagged_statement_count: number;
};

export type AvailableFilters = {
    actors: ActorOption[];
    pattern_tags: string[];
    subjects: ActorOption[];
    /** Server-resolved id of the subject named CASE_DEFAULT_SUBJECT_NAME.
     *  Absent when the env var is unset or no subject matches. The page
     *  applies it as the initial value of the "About" filter. */
    default_subject_id?: string;
};

export type BiasQueryFilters = {
    actor_id?: string;
    pattern_tag?: string;
    subject_id?: string;
};

export type DocumentRef = {
    id: string;
    title: string;
    document_type?: string;
};

export type BiasInstance = {
    evidence_id: string;
    title: string;
    verbatim_quote?: string;
    page_number?: number;
    pattern_tags: string[];
    stated_by?: ActorOption;
    about: ActorOption[];
    document?: DocumentRef;
};

export type BiasQueryResult = {
    total_count: number;
    /** Count of all tagged Evidence regardless of filters. The frontend
     *  uses this to render "Filtered: X of Y" — distinct from total_count
     *  so a graph that happens to filter to 100% of items still shows
     *  Y separately. */
    total_unfiltered: number;
    instances: BiasInstance[];
    applied_filters: BiasQueryFilters;
};

// ─── Service functions ──────────────────────────────────────────────────────

/**
 * Fetch the dropdown contents for the Bias Explorer filter bar.
 *
 * Called once on page mount. The data here drives both the Actor and Pattern
 * dropdowns — neither is ever populated from a hardcoded list.
 */
export async function getAvailableFilters(): Promise<AvailableFilters> {
    const response = await authFetch(`${API_BASE_URL}/api/bias/available-filters`);
    if (!response.ok) {
        throw new Error(
            `Failed to load bias filters: ${response.status} ${response.statusText}`,
        );
    }
    return response.json();
}

/**
 * Run the structured bias query with the given filter object.
 *
 * Sends `filters` as JSON; the backend treats absent fields as "no filter".
 * `applied_filters` in the response echoes back what the server actually
 * applied (useful for displaying the current filter state).
 */
export async function runBiasQuery(filters: BiasQueryFilters): Promise<BiasQueryResult> {
    const response = await authFetch(`${API_BASE_URL}/api/bias/query`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(filters),
    });
    if (!response.ok) {
        throw new Error(
            `Failed to run bias query: ${response.status} ${response.statusText}`,
        );
    }
    return response.json();
}
