// =============================================================================
// themeScan.ts — client for the background Theme Scan (start + poll) + models.
// -----------------------------------------------------------------------------
// Endpoints (backend, pipeline DB):
//   POST /api/cases/:slug/scenarios/:scenarioId/theme-scan
//        → { run_id, status, candidates_total } — starts a BACKGROUND scan and
//          returns immediately (the ~94-candidate fan-out runs in a tokio task).
//   GET  /api/cases/:slug/scenarios/:scenarioId/scan-runs/:runId
//        → live progress while `running`; the full `summary` once `completed`.
//   GET  /api/chat/models
//        → the active model catalog (registry ids) for the model picker.
//
// Mirrors `scenarioGather.ts` idioms exactly: `authFetch` (credentials + timeout,
// Rule 13) + `API_BASE_URL`, `encodeURIComponent` on every path param,
// `readErrorMessage` to surface the backend `{message}` on a non-2xx, and every
// non-2xx throws (Standing Rule 1 — no silent failures). The DTO shapes mirror
// backend/src/dto/theme_scan.rs verbatim.
//
// The 503 hard-gate message (vLLM endpoint down / wrong model loaded) is surfaced
// VERBATIM via `readErrorMessage` so the panel shows the backend's exact wording
// (it names the endpoint / both model ids), not a generic "failed".
// =============================================================================

import type { BiasInstance } from "./bias";
import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { readErrorMessage } from "./fetchUtils";

// ─── DTO mirrors (backend dto/theme_scan.rs) ─────────────────────────────────

/** Immediate response to the POST — the scan runs in the background. */
export type ScanStartedResponse = {
  run_id: string;
  status: string;
  candidates_total: number;
};

/** One RELEVANT verdict (backend `ThemeScanSuggestion`). */
export type ThemeScanSuggestion = {
  graph_node_id: string;
  proposed_role: string;
  reason: string;
  confidence: number;
  content: BiasInstance;
};

/** One REJECTED quote surfaced for the honesty check (backend `ThemeScanRejected`). */
export type ThemeScanRejected = {
  graph_node_id: string;
  reason: string;
  confidence: number;
  content: BiasInstance;
};

/** The full result of one completed run (backend `ThemeScanSummary`). */
export type ThemeScanSummary = {
  run_id: string;
  model_id: string;
  dry_run: boolean;
  input_tokens: number | null;
  output_tokens: number | null;
  computed_cost: number | null;
  duration_ms: number;
  candidates_read: number;
  relevant_written: number;
  irrelevant: number;
  failed: number;
  suggestions: ThemeScanSuggestion[];
  rejected_sample: ThemeScanRejected[];
};

/** The poll response (backend `ScanRunStatusResponse`). While `running`, the
 *  counts are a LIVE, advancing ESTIMATE; `summary` is present only once
 *  `completed`; `error` is present only when `failed`. */
export type ScanRunStatus = {
  run_id: string;
  status: "running" | "completed" | "failed";
  model_id: string;
  dry_run: boolean;
  candidates_total: number | null;
  candidates_judged: number;
  relevant_count: number;
  irrelevant_count: number;
  failed_count: number;
  error?: string;
  summary?: ThemeScanSummary;
};

/** One selectable model (backend `ChatModelEntry`). NOTE: the contract does not
 *  expose `provider`, so the panel cannot label Cloud vs Local from this alone. */
export type ScanModel = {
  model_id: string;
  display_name: string;
  is_default: boolean;
};

/** One row of the scan-run HISTORY list (backend `ScanRunHeader`). Headers only —
 *  the full result (suggestions + rejected sample) is fetched lazily per-run via
 *  [`getScanRun`] when a row is opened. `computed_cost` is `null` for a local
 *  model or when no token usage was reported. `started_at` is ISO-8601 and drives
 *  the newest-first order the backend already applied. */
export type ScanRunHeader = {
  run_id: string;
  model_id: string;
  dry_run: boolean;
  status: "running" | "completed" | "failed";
  candidates_total: number | null;
  candidates_judged: number;
  relevant_count: number;
  irrelevant_count: number;
  failed_count: number;
  computed_cost: number | null;
  duration_ms: number;
  started_at: string;
};

// ─── URL helpers ─────────────────────────────────────────────────────────────

function scenarioBase(slug: string, scenarioId: string): string {
  return `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/scenarios/${encodeURIComponent(
    scenarioId,
  )}`;
}

// ─── Service functions ───────────────────────────────────────────────────────

/** Start a background scan. Returns the run handle; poll [`getScanRun`].
 *
 *  A precondition failure (missing attack_meaning, bad model, or the vLLM hard
 *  gate) throws HERE with the backend's message — it is NOT a background failure. */
export async function startThemeScan(
  slug: string,
  scenarioId: string,
  body: { model_id?: string; dry_run: boolean },
): Promise<ScanStartedResponse> {
  const response = await authFetch(`${scenarioBase(slug, scenarioId)}/theme-scan`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!response.ok) {
    // The backend message rides through verbatim (names the endpoint / models on
    // a 503 hard-gate refusal) — surface it, do not flatten to a generic error.
    throw new Error(`Failed to start theme scan${await readErrorMessage(response)}`);
  }
  return (await response.json()) as ScanStartedResponse;
}

/** Poll one run's live status. `summary` is populated once `status` is `completed`. */
export async function getScanRun(
  slug: string,
  scenarioId: string,
  runId: string,
): Promise<ScanRunStatus> {
  const response = await authFetch(
    `${scenarioBase(slug, scenarioId)}/scan-runs/${encodeURIComponent(runId)}`,
  );
  if (!response.ok) {
    throw new Error(`Failed to read scan run${await readErrorMessage(response)}`);
  }
  return (await response.json()) as ScanRunStatus;
}

/** Fetch a scenario's scan-run HISTORY, newest first (backend already orders it).
 *
 *  Retrieval-only over the persisted `scan_runs` headers — this is the source of
 *  truth the panel hydrates from on mount, so history survives navigation and
 *  reloads. A non-2xx throws (Standing Rule 1); an unscanned scenario returns `[]`. */
export async function fetchScanRuns(
  slug: string,
  scenarioId: string,
): Promise<ScanRunHeader[]> {
  const response = await authFetch(`${scenarioBase(slug, scenarioId)}/scan-runs`);
  if (!response.ok) {
    throw new Error(`Failed to load scan history${await readErrorMessage(response)}`);
  }
  const body = (await response.json()) as { runs?: ScanRunHeader[] };
  return body.runs ?? [];
}

/** Fetch the SCAN model catalog for the picker (active AND scan_eligible ids).
 *
 *  Uses the dedicated `/api/scan/models` endpoint — NOT `/api/chat/models` — so
 *  the scan picker shows only scan-eligible models (retired-but-extraction-active
 *  Claude rows are filtered out), while the chat dropdown is left untouched. */
export async function fetchScanModels(): Promise<ScanModel[]> {
  const response = await authFetch(`${API_BASE_URL}/api/scan/models`);
  if (!response.ok) {
    throw new Error(`Failed to load models${await readErrorMessage(response)}`);
  }
  const body = (await response.json()) as { models?: ScanModel[] };
  return body.models ?? [];
}
