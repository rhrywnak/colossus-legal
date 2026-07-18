// =============================================================================
// scenarioGather.ts — client for the candidate-workbench routes (Phase 1a.2/1a.3)
// -----------------------------------------------------------------------------
// Endpoints (backend, pipeline DB):
//   GET  /api/cases/:slug/scenarios/:scenarioId/facts/gather
//        → { pool, dropped } — every Evidence node ABOUT the scenario's subject,
//          each tagged with its derived three-state status for THIS scenario.
//   POST /api/cases/:slug/scenarios/:scenarioId/facts/:graphNodeId/action
//        → apply a human ruling (include / drop / undrop) to one candidate.
//
// Mirrors `scenarioCrud.ts` idioms exactly: `authFetch` (credentials + 30s
// AbortController timeout — Rule 13) + `API_BASE_URL`, `encodeURIComponent` on
// every path param, `readErrorMessage` to surface the backend `{message}` on a
// non-2xx, boundary validation of the load-bearing fields (throw a contextual
// error HERE on a contract mismatch), and every non-2xx throws (Standing Rule 1
// — no silent failures).
//
// The DTO shapes mirror backend/src/dto/scenario_facts.rs verbatim.
// =============================================================================

import type { BiasInstance } from "./bias";
import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { readErrorMessage } from "./fetchUtils";

// ─── DTO mirrors ────────────────────────────────────────────────────────────

/** A candidate's derived workbench state for one scenario (backend `FactStatus`). */
export type FactStatus = "undecided" | "included" | "dropped";

/** A human ruling to apply (backend `FactAction`). Note the vocabularies differ
 *  from `FactStatus`: `undrop` is a verb, not a state — it maps to `undecided`. */
export type FactAction = "include" | "drop" | "undrop";

/**
 * One candidate in the workbench pool (backend `CandidateDto`).
 *
 * TS-learning: `content` REUSES the `BiasInstance` type from `bias.ts` rather
 * than redeclaring its fields — composition over duplication. The exact same
 * card shape renders both a Bias Explorer candidate and a saved scenario fact
 * (one `EvidenceCard`, two paths). If the two redeclared the fields, they would
 * diverge the first time someone edited one of them; sharing the type makes a
 * backend change to the card content a single edit in `bias.ts`.
 */
export type CandidateDto = {
  content: BiasInstance;
  status: FactStatus;
  role: string | null;
  /**
   * The scan/merge model's confidence in this fact's role, in `[0, 1]`, or
   * `null`/absent when the ref carries no model score (an undecided candidate
   * with no ref row, or a human-curated include/drop).
   *
   * TS-learning: the backend `#[serde(skip_serializing_if = "Option::is_none")]`
   * means an unscored candidate OMITS this key entirely, so it deserializes as
   * `undefined` here — not `null`. Both mean "unscored", and the workbench's
   * null-ish check (`== null`) covers both, so it renders "unscored", never
   * "0%". This is deliberately distinct from a real `0` (a model score of zero):
   * `undefined`/`null` = never scored; `0` = scored zero. Do not `?? 0` this
   * value anywhere — that would erase the distinction (Standing Rule 1).
   */
  confidence: number | null;
  note: string | null;
};

/** The gather response: the working pool (undecided + included) and the dropped
 *  tray, kept as two lists (backend `GatherCandidatesResponse`). */
export type GatherCandidatesResponse = {
  pool: CandidateDto[];
  dropped: CandidateDto[];
};

// ─── URL helpers ────────────────────────────────────────────────────────────

/** Base URL for one scenario's facts collection. */
function factsUrl(slug: string, scenarioId: string): string {
  return `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/scenarios/${encodeURIComponent(
    scenarioId,
  )}/facts`;
}

// ─── Service functions ──────────────────────────────────────────────────────

/**
 * Fetch the candidate workbench pool via `GET …/facts/gather`.
 *
 * Returns the whole bounded set in one response, so the caller filters by status
 * in memory (no per-filter refetch). Validates that `pool` and `dropped` are
 * arrays so a backend contract drift throws HERE with context, not as an
 * `undefined.map` deep in a component later.
 *
 * @throws Error on non-2xx, an unparseable body, or a body missing the
 *   `pool`/`dropped` arrays.
 */
export async function gatherCandidates(
  slug: string,
  scenarioId: string,
): Promise<GatherCandidatesResponse> {
  const response = await authFetch(`${factsUrl(slug, scenarioId)}/gather`);

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to load the candidate pool for scenario "${scenarioId}" ` +
        `(HTTP ${response.status}${detail}). Try reloading the page.`,
    );
  }

  let data: unknown;
  try {
    data = await response.json();
  } catch {
    throw new Error(
      `Gather response for scenario "${scenarioId}" was not valid JSON ` +
        `(the backend may be down). Try reloading the page.`,
    );
  }

  const parsed = data as Partial<GatherCandidatesResponse>;
  if (!Array.isArray(parsed.pool) || !Array.isArray(parsed.dropped)) {
    throw new Error(
      `Gather response for scenario "${scenarioId}" is missing the pool/dropped ` +
        `lists — backend/frontend contract mismatch. Report this to the site administrator.`,
    );
  }
  return parsed as GatherCandidatesResponse;
}

/**
 * Apply one human ruling to a candidate via `POST …/facts/:graphNodeId/action`.
 *
 * The backend returns `200 OK` with no meaningful body — the resolved promise
 * (void) IS the success signal, mirroring `deleteScenario`'s no-body idiom. A
 * non-2xx (400 bad action token / bad UUID, 404 wrong scenario/case, 500 store
 * fault) throws a contextual error so the caller can surface it and refresh
 * rather than report a ruling that did not persist (Standing Rule 1).
 *
 * @throws Error on any non-2xx (with the HTTP status and the backend's message
 *   when present).
 */
export async function applyFactAction(
  slug: string,
  scenarioId: string,
  graphNodeId: string,
  action: FactAction,
): Promise<void> {
  const response = await authFetch(
    `${factsUrl(slug, scenarioId)}/${encodeURIComponent(graphNodeId)}/action`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ action }),
    },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to ${action} fact "${graphNodeId}" on scenario "${scenarioId}" ` +
        `(HTTP ${response.status}${detail}). Try again.`,
    );
  }
  // 200 OK — nothing to read back; the absence of a throw is success.
}
