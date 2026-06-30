// =============================================================================
// Scenario fact-curation service — wire calls for
// /api/cases/:slug/scenarios/:scenarioId/facts.
// =============================================================================
//
// Phase A of scenario curation: a human saves graph facts onto a scenario,
// lists the saved set (with live content), and removes one. The DTO shapes
// mirror backend/src/dto/scenario_facts.rs verbatim.
//
// All requests go through `authFetch` (30s timeout via AbortController,
// credentials included). Standard error surfacing: throw a contextual error on
// any non-OK response so the caller can render it (Standing Rule 1 — no
// swallowed rejection).

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import type { BiasInstance } from "./bias";

// ─── DTO mirrors ──────────────────────────────────────────────────────────────

/** A saved fact on a scenario. `content` is the live graph card content, or
 *  `null` when the saved node id no longer resolves to a live Evidence node —
 *  a stale reference the UI must still show (never silently drop). */
export type ScenarioFactDto = {
  graph_node_id: string;
  role: string | null;
  note: string | null;
  content: BiasInstance | null;
};

/** Body for adding a fact. `role`/`note` are accepted by the backend but not
 *  surfaced in Phase A — callers send only `graph_node_id` for now. */
export type AddFactBody = {
  graph_node_id: string;
  role?: string;
  note?: string;
};

// ─── Service functions ──────────────────────────────────────────────────────

/** Build the facts collection URL for one scenario. */
function factsUrl(slug: string, scenarioId: string): string {
  return `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/scenarios/${encodeURIComponent(
    scenarioId,
  )}/facts`;
}

/**
 * Save a graph fact onto a scenario (`POST` → 201).
 *
 * Idempotent on `(scenario, graph_node_id)`: re-adding the same fact updates it
 * in place rather than erroring.
 */
export async function addScenarioFact(
  slug: string,
  scenarioId: string,
  body: AddFactBody,
): Promise<void> {
  const response = await authFetch(factsUrl(slug, scenarioId), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!response.ok) {
    throw new Error(
      `Failed to add the fact to scenario "${scenarioId}" (HTTP ${response.status}). ` +
        `If this persists, report it to the site administrator.`,
    );
  }
}

/**
 * Remove a saved fact from a scenario (`DELETE` → 204).
 *
 * A 404 means the fact was not on this scenario (already removed elsewhere);
 * it is surfaced as an error so the caller can refresh and reconcile, never
 * swallowed.
 */
export async function removeScenarioFact(
  slug: string,
  scenarioId: string,
  graphNodeId: string,
): Promise<void> {
  const response = await authFetch(
    `${factsUrl(slug, scenarioId)}/${encodeURIComponent(graphNodeId)}`,
    { method: "DELETE" },
  );
  if (!response.ok) {
    throw new Error(
      `Failed to remove the fact from scenario "${scenarioId}" (HTTP ${response.status}). ` +
        `Try reloading the page.`,
    );
  }
}

/**
 * List a scenario's saved facts with their live content (`GET` → 200 `[…]`).
 *
 * An existing scenario with no facts yet returns `[]`. A 404 (missing scenario)
 * throws — distinct from the empty-but-present case.
 */
export async function listScenarioFacts(
  slug: string,
  scenarioId: string,
): Promise<ScenarioFactDto[]> {
  const response = await authFetch(factsUrl(slug, scenarioId));
  if (!response.ok) {
    throw new Error(
      `Failed to load saved facts for scenario "${scenarioId}" (HTTP ${response.status}). ` +
        `Try reloading the page.`,
    );
  }

  let data: unknown;
  try {
    data = await response.json();
  } catch {
    throw new Error(
      `Saved-facts response for scenario "${scenarioId}" was not valid JSON ` +
        `(the backend may be down). Try reloading the page.`,
    );
  }

  if (!Array.isArray(data)) {
    throw new Error(
      `Saved-facts response for scenario "${scenarioId}" was not a list — ` +
        `backend/frontend contract mismatch. Report this to the site administrator.`,
    );
  }
  return data as ScenarioFactDto[];
}
