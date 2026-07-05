// =============================================================================
// scenarioCrud.ts — client for the scenario CRUD routes (Chunk 1 backend)
// -----------------------------------------------------------------------------
// Endpoints (Postgres-backed, pipeline DB):
//   POST /api/cases/:slug/scenarios  → create one scenario
//   GET  /api/cases/:slug/scenarios  → list a case's scenarios
//
// Mirrors `services/trialPrep.ts`: `authFetch` (credentials + timeout) +
// `API_BASE_URL`, boundary validation of the load-bearing fields, and thrown
// contextual errors at the boundary so a malformed body never crashes a
// component later (Standing Rule 1 — no silent failures).
//
// `ScenarioDto` mirrors the backend `dto/scenario_crud.rs` field-for-field.
// =============================================================================

import type { ScenarioDefinition, ScenarioStatus } from "../pages/trialPrepData";
import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { DEFAULT_CASE_SLUG } from "./caseHeader";

/** Scenario posture — matches the backend `direction` CHECK vocabulary. */
export type ScenarioDirection = "offense" | "defense";

/**
 * One scenario as the create/list routes return it (mirrors the backend
 * `ScenarioDto`). `anchor_allegation_ids` is always an array (the backend
 * flattens a SQL NULL to `[]`); `definition` is opaque authored JSON.
 */
export interface ScenarioDto {
  scenario_id: string;
  name: string;
  direction: string;
  status: string;
  case_slug: string;
  feeds_count_id: string | null;
  anchor_allegation_ids: string[];
  definition: unknown;
}

/**
 * The create-scenario request body (the backend sources `case_slug` from the
 * URL, never the body). Mirrors the backend `ScenarioCreateRequest`: `name` and
 * `direction` are required; the rest are optional with server-applied defaults.
 * `feeds_count_id` and `definition` have no form field yet (a later chunk) but
 * are typed here so the payload faithfully matches the backend request shape.
 */
export interface ScenarioCreatePayload {
  name: string;
  direction: ScenarioDirection;
  status?: ScenarioStatus;
  feeds_count_id?: string;
  anchor_allegation_ids?: string[];
  definition?: unknown;
}

/**
 * The update-scenario request body (mirrors the backend `ScenarioUpdateRequest`).
 * Partial update: every field is optional and an ABSENT field leaves that column
 * unchanged (the backend merges provided fields via SQL COALESCE).
 *
 * Two deliberate omissions, matching the backend: `direction` is NOT updatable
 * (offense/defense is scenario identity), and `case_slug` is NOT here (the URL
 * path is the only source of the case). Unlike `ScenarioCreatePayload.definition`
 * (opaque `unknown`), this `definition` is the TYPED `ScenarioDefinition` — the
 * authoring form builds a real definition, and the backend re-validates it.
 */
export interface ScenarioUpdatePayload {
  name?: string;
  status?: ScenarioStatus;
  feeds_count_id?: string;
  anchor_allegation_ids?: string[];
  definition?: ScenarioDefinition;
}

/**
 * Best-effort: pull the backend's human-readable `message` out of an error body
 * so a validation failure (e.g. "name must not be empty") reaches the user. The
 * create route returns `{ error, message, details }` on `BadRequest`.
 */
async function readErrorMessage(response: Response): Promise<string> {
  try {
    const body: unknown = await response.json();
    if (
      body !== null &&
      typeof body === "object" &&
      typeof (body as { message?: unknown }).message === "string"
    ) {
      return ` — ${(body as { message: string }).message}`;
    }
  } catch {
    // Body absent or not JSON — no extra detail to surface.
  }
  return "";
}

/**
 * Create a scenario in `slug`'s case via `POST /api/cases/:slug/scenarios`.
 *
 * @throws Error on non-2xx (with the HTTP status and the backend's field-named
 *   message when present), an unparseable body, or a body missing the
 *   load-bearing fields.
 */
export async function createScenario(
  slug: string = DEFAULT_CASE_SLUG,
  payload: ScenarioCreatePayload,
): Promise<ScenarioDto> {
  // authFetch adds credentials + a 30s timeout (AbortController) — Rule 13.
  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/scenarios`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to create scenario for "${slug}" (HTTP ${response.status}${detail}).`,
    );
  }

  let data: unknown;
  try {
    data = await response.json();
  } catch {
    throw new Error(
      `Create-scenario response for "${slug}" was not valid JSON (the backend may be down).`,
    );
  }

  // Validate the fields the form + dashboard card actually render, so a backend
  // rename/omission throws HERE (with context) rather than surfacing as an
  // `undefined` deep in a component later.
  const parsed = data as Partial<ScenarioDto>;
  if (
    typeof parsed.scenario_id !== "string" ||
    typeof parsed.name !== "string" ||
    typeof parsed.status !== "string" ||
    !Array.isArray(parsed.anchor_allegation_ids)
  ) {
    throw new Error(
      `Create-scenario response for "${slug}" is missing required fields ` +
        `(scenario_id/name/status/anchor_allegation_ids) — backend/frontend contract mismatch. ` +
        `If this persists, report it to the site administrator.`,
    );
  }
  return parsed as ScenarioDto;
}

/**
 * List a case's scenarios via `GET /api/cases/:slug/scenarios`. Used to refresh
 * the form's list / the dashboard after a create.
 *
 * @throws Error on non-2xx, an unparseable body, or a non-array body.
 */
export async function listScenarios(
  slug: string = DEFAULT_CASE_SLUG,
): Promise<ScenarioDto[]> {
  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/scenarios`,
  );

  if (!response.ok) {
    throw new Error(
      `Failed to load scenarios for "${slug}" (HTTP ${response.status}). Try reloading the page.`,
    );
  }

  let data: unknown;
  try {
    data = await response.json();
  } catch {
    throw new Error(
      `Scenarios response for "${slug}" was not valid JSON (the backend may be down).`,
    );
  }

  if (!Array.isArray(data)) {
    throw new Error(
      `Scenarios response for "${slug}" was not an array — backend/frontend contract mismatch.`,
    );
  }
  return data as ScenarioDto[];
}

/**
 * Update a scenario via `PUT /api/cases/:slug/scenarios/:scenarioId` (B1's
 * partial-update route). Returns the updated `ScenarioDto`.
 *
 * Mirrors {@link createScenario}'s idiom exactly, for `PUT`: `authFetch`
 * (credentials + 30s timeout), the backend's field-named `message` surfaced on a
 * non-2xx, and boundary validation of the load-bearing response fields so a
 * contract drift throws HERE rather than deep in a component.
 *
 * A `PUT` to a missing/cross-case scenario returns 404 — which is an ERROR here
 * (the caller is editing a scenario it just loaded), not an empty state. It is
 * surfaced as a thrown, contextual error like any other non-2xx; this function
 * never returns null.
 *
 * @throws Error on any non-2xx (with the HTTP status and the backend's message
 *   when present — a 404 for an unknown id, a 400 for a malformed definition), an
 *   unparseable body, or a body missing the load-bearing fields.
 */
export async function updateScenario(
  slug: string = DEFAULT_CASE_SLUG,
  scenarioId: string,
  payload: ScenarioUpdatePayload,
): Promise<ScenarioDto> {
  // authFetch adds credentials + a 30s timeout (AbortController) — Rule 13.
  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/scenarios/${encodeURIComponent(scenarioId)}`,
    {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to update scenario "${scenarioId}" for "${slug}" (HTTP ${response.status}${detail}).`,
    );
  }

  let data: unknown;
  try {
    data = await response.json();
  } catch {
    throw new Error(
      `Update-scenario response for "${scenarioId}" was not valid JSON (the backend may be down).`,
    );
  }

  // Validate the fields the caller relies on, so a backend rename/omission throws
  // HERE (with context) rather than surfacing as an `undefined` later.
  const parsed = data as Partial<ScenarioDto>;
  if (
    typeof parsed.scenario_id !== "string" ||
    typeof parsed.name !== "string" ||
    typeof parsed.status !== "string" ||
    !Array.isArray(parsed.anchor_allegation_ids)
  ) {
    throw new Error(
      `Update-scenario response for "${scenarioId}" is missing required fields ` +
        `(scenario_id/name/status/anchor_allegation_ids) — backend/frontend contract mismatch. ` +
        `If this persists, report it to the site administrator.`,
    );
  }
  return parsed as ScenarioDto;
}

/**
 * Hard-delete a scenario via `DELETE /api/cases/:slug/scenarios/:scenarioId`.
 *
 * The backend returns `204 No Content` on success — so, unlike create/update,
 * there is NO body to parse: the resolved promise (void) IS the success signal.
 * A non-2xx (404 for an unknown id / wrong case, 400 for a malformed id, 500 for
 * a store fault) is surfaced as a thrown, contextual error — never swallowed, so
 * the caller can keep the confirm dialog open and show the failure rather than
 * report a delete that did not happen (Standing Rule 1).
 *
 * Mirrors `removeScenarioFact`'s no-body DELETE idiom, plus `updateScenario`'s
 * `readErrorMessage` so the backend's message reaches the user on a 4xx.
 *
 * @throws Error on any non-2xx (with the HTTP status and the backend's message
 *   when present).
 */
export async function deleteScenario(
  slug: string = DEFAULT_CASE_SLUG,
  scenarioId: string,
): Promise<void> {
  // authFetch adds credentials + a 30s timeout (AbortController) — Rule 13.
  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/scenarios/${encodeURIComponent(scenarioId)}`,
    { method: "DELETE" },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to delete scenario "${scenarioId}" for "${slug}" (HTTP ${response.status}${detail}).`,
    );
  }
  // 204 No Content — nothing to read back; the absence of a throw is success.
}
