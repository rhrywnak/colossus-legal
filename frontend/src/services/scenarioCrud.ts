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

import type { ScenarioStatus } from "../pages/trialPrepData";
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
