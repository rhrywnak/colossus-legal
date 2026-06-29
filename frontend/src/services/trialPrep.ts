// =============================================================================
// trialPrep.ts — client for GET /api/cases/:slug/trial-prep/dashboard
// -----------------------------------------------------------------------------
// Endpoint: GET /api/cases/:slug/trial-prep/dashboard → backend handler
// `api::trial_prep`. Neo4j-backed, read-only. Returns the War Room dashboard
// payload (metrics band · alerts strip · scenario cards).
//
// Stage-2 wiring (thin vertical slice): exactly ONE number in this payload is
// graph-derived — `marie-obstructive`'s `instance_count` (the live ¶54 REBUTS
// count). Every other value is still the backend's slice baseline. This service
// is the data-source swap for the dashboard page; it returns the SAME contract
// type the placeholder did, so the page component renders unchanged.
//
// The return type is the EXISTING `TrialPrepDashboard` contract in
// `pages/trialPrepData.ts` — imported, never redefined, so backend and frontend
// share one shape (the backend serde DTO mirrors it field-for-field).
// =============================================================================

import type { ScenarioDetail, TrialPrepDashboard } from "../pages/trialPrepData";
import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { DEFAULT_CASE_SLUG } from "./caseHeader";

/**
 * Fetch the War Room dashboard for `slug` (defaults to the single seeded case).
 *
 * Mirrors `getProofMatrixRollup`: it validates the load-bearing fields
 * (`scenarios` is an array, `metrics` is present) and throws a contextual error
 * at the boundary rather than letting a malformed body crash a component later —
 * Standing Rule 1 (no silent failures). The caller (the gating fetch in
 * `TrialPrepDashboardPage`) surfaces the thrown message in the error UI.
 *
 * @param slug case slug; defaults to {@link DEFAULT_CASE_SLUG}
 * @returns the typed dashboard payload
 * @throws Error on non-2xx, unparseable body, or a body missing `scenarios`/`metrics`
 */
export async function getTrialPrepDashboard(
  slug: string = DEFAULT_CASE_SLUG,
): Promise<TrialPrepDashboard> {
  // authFetch adds credentials + a 30s timeout (AbortController) — Rule 13.
  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/trial-prep/dashboard`,
  );

  if (!response.ok) {
    throw new Error(
      `Failed to load the Trial Prep dashboard for "${slug}" (HTTP ${response.status}). Try reloading the page.`,
    );
  }

  let data: unknown;
  try {
    data = await response.json();
  } catch {
    throw new Error(
      `Trial Prep dashboard response for "${slug}" was not valid JSON (the backend may be down). Try reloading the page.`,
    );
  }

  // Validate the two load-bearing fields the page renders. A contract mismatch
  // here (backend/frontend field drift) is the most likely break, so name it
  // explicitly instead of letting a downstream `.map`/`.length` throw obscurely.
  const parsed = data as Partial<TrialPrepDashboard>;
  if (!Array.isArray(parsed.scenarios) || parsed.metrics == null) {
    throw new Error(
      `Trial Prep dashboard response for "${slug}" is missing "scenarios"/"metrics" — ` +
        `backend/frontend contract mismatch. If reloading does not help, report this to the site administrator.`,
    );
  }

  return parsed as TrialPrepDashboard;
}

/**
 * Fetch one scenario's detail via
 * `GET /api/cases/:slug/trial-prep/scenarios/:scenarioId`.
 *
 * Returns `null` on a 404 (a deleted/unknown scenario) so the page shows its
 * "Scenario not found" empty state rather than an error banner. Any OTHER
 * failure throws a contextual error (Standing Rule 1), mirroring
 * {@link getTrialPrepDashboard}.
 *
 * @returns the typed detail payload, or `null` when no such scenario exists
 * @throws Error on a non-404 non-2xx, an unparseable body, or a body missing
 *   `attack`/`timeline`
 */
export async function getScenarioDetailLive(
  slug: string,
  scenarioId: string,
): Promise<ScenarioDetail | null> {
  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/trial-prep/scenarios/${encodeURIComponent(scenarioId)}`,
  );

  // A real 404 (deleted/unknown id) is not an error — it's the empty state.
  if (response.status === 404) {
    return null;
  }
  if (!response.ok) {
    throw new Error(
      `Failed to load scenario "${scenarioId}" for "${slug}" (HTTP ${response.status}). Try reloading the page.`,
    );
  }

  let data: unknown;
  try {
    data = await response.json();
  } catch {
    throw new Error(
      `Scenario detail response for "${scenarioId}" was not valid JSON (the backend may be down).`,
    );
  }

  // Validate the fields the page renders, so a contract drift throws here (with
  // context) rather than as an `undefined` deep in the timeline render.
  const parsed = data as Partial<ScenarioDetail>;
  if (typeof parsed.attack !== "string" || !Array.isArray(parsed.timeline)) {
    throw new Error(
      `Scenario detail response for "${scenarioId}" is missing "attack"/"timeline" — ` +
        `backend/frontend contract mismatch. If this persists, report it to the site administrator.`,
    );
  }
  return parsed as ScenarioDetail;
}
