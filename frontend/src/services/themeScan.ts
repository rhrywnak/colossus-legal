// =============================================================================
// themeScan.ts — client for the background Theme Scan (start + poll) + models.
// -----------------------------------------------------------------------------
// Endpoints (backend, pipeline DB):
//   POST /api/cases/:slug/scenarios/:scenarioId/theme-scan
//        → { run_id, status, candidates_total } — starts a BACKGROUND scan and
//          returns immediately (the ~94-candidate fan-out runs in a tokio task).
//   GET  /api/cases/:slug/scenarios/:scenarioId/scan-runs/:runId
//        → live progress while `running`; the full `summary` once `completed`.
//   POST /api/cases/:slug/scenarios/:scenarioId/scan-runs/:runId/cancel
//        → 202 { run_id, status: "cancelling" } — signals the backend's judging
//          task to stop. The row becomes `cancelled` a moment later; the POLL is
//          what observes that, not this response.
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
  /** Every node id this ONE pick speaks for — `graph_node_id` first, then any
   *  byte-identical twin the scan folded into it (task 2.15 Tier 2).
   *
   *  Merging sends this whole list, not the single id: the twins were never
   *  judged separately (one quote, one call), so one ruling covers the set —
   *  otherwise the identical sentence returns tomorrow as an unruled candidate. */
  covers_node_ids: string[];
  /** How many pool rows this pick settles. `1` normally; `2` for a folded twin. */
  duplicate_count: number;
  content: BiasInstance;
  /** The candidate's persisted scenario ordinal, rendered `C-{ordinal}` — the SAME
   *  chip the fact wears in Candidate Facts, which is what makes the two listings
   *  cross-referencable. `null` when the candidate has no ordinal yet (never a
   *  fabricated 0: "C-0" is not a card that exists).
   *
   *  Annotated by the backend at read time — it is not part of the stored scan
   *  summary, because a scenario may assign the ordinal after the run judged it. */
  ordinal: number | null;
  /** Whether THIS run's judgment for this pick has already been merged into the
   *  scenario. Derived server-side from `scenario_fact_refs.source_run_id`, so it
   *  is exact rather than inferred. An applied pick renders as applied instead of
   *  offering a checkbox — re-merging it would be a no-op the human cannot see. */
  applied: boolean;
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
  input_tokens: number | null;
  output_tokens: number | null;
  computed_cost: number | null;
  duration_ms: number;
  candidates_read: number;
  /** Verdicts judged relevant — picks awaiting the human's decision. NOT a count
   *  of anything written: a scan never adds facts to the scenario. */
  relevant: number;
  irrelevant: number;
  failed: number;
  suggestions: ThemeScanSuggestion[];
  rejected_sample: ThemeScanRejected[];
  /** Where every gathered row went (task 2.15 item 1c). Frozen into the run when
   *  it completed, so a run recorded before that task carries none — hence
   *  optional, and the panel shows no reconciliation for those. */
  conservation?: ScanConservation;
  /** The reconciliation sentence, composed BY THE BACKEND at read time from the
   *  counts above and the stored template. Absent for a run with no counts.
   *
   *  The browser renders it and computes nothing: the numbers are the run's, the
   *  words are the settings store's, and neither is the client's to invent. */
  conservation_line?: string;
};

/** pool → excluded → collapsed → judged, as one run measured it. */
export type ScanConservation = {
  pool: number;
  excluded_empty: number;
  excluded_statement_type: number;
  excluded_too_short: number;
  duplicates_collapsed: number;
  judged: number;
  /** Judged calls that came back with no verdict.
   *
   *  Added 2026-08-09 (ruling R4). `scan_runs.failed_count` had recorded 104
   *  dead calls all along, but the report's tiles and reconciliation sentence are
   *  built from THIS block — which had no field for them — so the screen read
   *  "Complete · 104 judged · 0 relevant". The law the block now lets a reader
   *  check is `judged = relevant + irrelevant + failed`. */
  failed: number;
};

/** One run's lifecycle state, as the backend's `SCAN_STATUS_*` consts write it.
 *
 *  `cancelled` is a human's Stop (task SCAN_SERVER_STATE part C) and is TERMINAL
 *  like the other two, but it is neither of them: not `completed` (the run never
 *  judged its whole pool, so its counts are partial), and not `failed` (nothing
 *  went wrong). The verdicts it judged before the stop are kept and keep
 *  projecting — which is the entire reason to stop rather than delete. */
export type ScanRunState = "running" | "completed" | "cancelled" | "failed";

/** The poll response (backend `ScanRunStatusResponse`). While `running`, the
 *  counts are a LIVE, advancing ESTIMATE; `summary` is present only once
 *  `completed`; `error` is present only when `failed`. A `cancelled` run has
 *  NEITHER — no summary (there is no finished whole) and no error (nothing went
 *  wrong): its counts on the row are the whole record. */
export type ScanRunStatus = {
  run_id: string;
  status: ScanRunState;
  model_id: string;
  candidates_total: number | null;
  candidates_judged: number;
  relevant_count: number;
  irrelevant_count: number;
  failed_count: number;
  error?: string;
  summary?: ThemeScanSummary;
};

/** One selectable model (backend `ChatModelEntry`). */
export type ScanModel = {
  model_id: string;
  display_name: string;
  is_default: boolean;
  /** The name as the scan CONFIRMATION says it, with the cost stated for BOTH
   *  classes — "Qwen3.8 27B (NVFP4, local) (local · $0)". Composed by the
   *  backend, like `display_label`, and deliberately worded differently: the
   *  picker leaves the free case undecorated because a label on every row is a
   *  label nobody reads, and the confirmation says `$0` out loud because "is
   *  this the free one?" is the question it exists to answer. */
  confirm_label: string;
  /** Seconds per candidate this deployment has MEASURED for this model, from
   *  its own completed runs — or `undefined` when nothing has timed it yet.
   *
   *  Absent rather than zero, and absent from the payload entirely: the
   *  confirmation multiplies it by the candidate count to tell a human how long
   *  their afternoon is about to be, and a defaulted `0` would promise "about 0
   *  minutes" for a scan nobody has ever run on this model. The time clause is
   *  dropped when this is absent (`estimatedMinutes`). */
  measured_seconds_per_candidate?: number;
  /** `local` (self-hosted, no metered cost) or `billed` (third-party API).
   *  The STATE, carried beside the label so a client branches on the token
   *  rather than reading meaning out of display prose (task 1.7B). */
  billing_class: string;
  /** The name as the picker shows it, with the cost warning already attached:
   *  "Opus 4.8 (API — billed)". Composed by the backend — which models cost
   *  money is a deployment fact, and a browser that mapped a provider name to
   *  English would be guessing on this deployment's behalf. */
  display_label: string;
};

/** One row of the scan-run HISTORY list (backend `ScanRunHeader`). Headers only —
 *  the full result (suggestions + rejected sample) is fetched lazily per-run via
 *  [`getScanRun`] when a row is opened. `computed_cost` is `null` for a local
 *  model or when no token usage was reported. `started_at` is ISO-8601 and drives
 *  the newest-first order the backend already applied. */
export type ScanRunHeader = {
  run_id: string;
  model_id: string;
  status: ScanRunState;
  candidates_total: number | null;
  candidates_judged: number;
  relevant_count: number;
  irrelevant_count: number;
  failed_count: number;
  computed_cost: number | null;
  duration_ms: number;
  started_at: string;
  /** The candidate pool this run READ — the history table's Candidates column and
   *  the basis of the pool delta (task 1.7C, ruling R2).
   *
   *  `0` means the run never reached the pool read. Rendered as an em dash, NEVER
   *  as the number zero: "read nothing" and "read a pool of zero" are different
   *  states (see `scanHistoryRows`). */
  candidates_read: number;
  /** Why a failed run failed, verbatim, so the history can say
   *  "Failed — vLLM offline" instead of just "Failed". `null` unless failed. */
  error: string | null;
  /** Whether this run was a dry run (judged, nothing merged), so the row can be
   *  labelled. Measured on DEV: 3 of the 4 stored runs are dry runs. */
  dry_run: boolean;
  /** How much bigger this run's pool was than the previous MEASURABLE run's —
   *  the history's New column and the meta line's "+Δ since the previous scan".
   *
   *  SIGNED (a shrinking pool is real after task 2.5's re-anchoring), and `null`
   *  on the first measurable run — rendered as an em dash, never `0`. Computed
   *  BACKEND-side (`services::scan_run_delta`, Standing Rule 12): it is a
   *  derivation over the whole history with two non-obvious rules in it, and a
   *  browser reimplementing them would eventually disagree with the server about
   *  what the pool did. */
  pool_delta: number | null;
  // No merge_count / last_merged_at: merge is pick-keyed, so a per-RUN merge
  // counter answers a question the workbench no longer asks. Whether a given pick
  // was applied is carried per-suggestion (`ThemeScanSuggestion.applied`).
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
  body: { model_id?: string },
): Promise<ScanStartedResponse> {
  const response = await authFetch(`${scenarioBase(slug, scenarioId)}/theme-scan`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (response.status === 409) {
    // The body is read ONCE here, so `readErrorMessage` (which also calls
    // `response.json()`) must not run on this path — a Response body can only be
    // consumed one time.
    const already = await readAlreadyRunning(response);
    if (already) throw already;
    // A 409 that is not the already-running one has no other meaning on this
    // route today, but it is not swallowed: fall through to the ordinary throw
    // below, which cannot re-read the body and says so honestly.
    throw new Error("Failed to start theme scan — the request conflicted with the scan's state.");
  }
  if (!response.ok) {
    // The backend message rides through verbatim (names the endpoint / models on
    // a 503 hard-gate refusal) — surface it, do not flatten to a generic error.
    throw new Error(`Failed to start theme scan${await readErrorMessage(response)}`);
  }
  return (await response.json()) as ScanStartedResponse;
}

/**
 * The 409 that means "a scan of this scenario is already going, here is its id".
 *
 * ## Why a typed error rather than a message the caller pattern-matches
 *
 * The panel's response to this is not to show an error at all — it is to ADOPT
 * the running run and start polling it, so the human sees the scan that is
 * actually happening instead of being told off for asking twice. That branch
 * needs the `run_id` as data. Sniffing it out of a prose message would work until
 * somebody edited the sentence; an `instanceof` check and a field cannot drift.
 */
export class ScanAlreadyRunningError extends Error {
  /** The run the browser should adopt and poll. */
  readonly runId: string;

  constructor(runId: string, message: string) {
    super(message);
    // ## TS note: restoring the prototype after extending a built-in
    //
    // `Error` is a built-in whose constructor returns a fresh object, so a
    // subclass compiled to ES5-era output loses its prototype link and
    // `instanceof` silently returns false. Setting it explicitly is the standard
    // fix and costs one line; without it the panel's adopt branch would never
    // run and the failure would look like the backend not sending a 409.
    Object.setPrototypeOf(this, ScanAlreadyRunningError.prototype);
    this.name = "ScanAlreadyRunningError";
    this.runId = runId;
  }
}

/**
 * Read a 409 body and return the typed error when it is the already-running one.
 *
 * The backend's error envelope is `{ error, message, details }` for every 4xx,
 * with the top-level `error` fixed at `"conflict"` for all 409s — so the specific
 * conflict is identified by `details.reason` (the shape the run-cited refusal
 * established). `null` when the body is absent, unparseable, or a different
 * conflict; the caller then throws its own error rather than guessing.
 */
async function readAlreadyRunning(response: Response): Promise<ScanAlreadyRunningError | null> {
  try {
    const body: unknown = await response.json();
    if (body === null || typeof body !== "object") return null;
    const { message, details } = body as { message?: unknown; details?: unknown };
    if (details === null || typeof details !== "object") return null;
    const { reason, run_id: runId } = details as { reason?: unknown; run_id?: unknown };
    if (reason !== "scan_already_running" || typeof runId !== "string") return null;
    return new ScanAlreadyRunningError(
      runId,
      typeof message === "string" ? message : "A scan of this scenario is already running.",
    );
  } catch {
    // Body absent or not JSON. Not swallowed — the caller throws either way; this
    // only decides whether the throw carries a run id to adopt.
    return null;
  }
}

/**
 * Ask the backend to STOP a run that is currently judging.
 *
 * Resolves on the 202. That is deliberately not the same as "it stopped": the
 * backend has signalled its judging task, and the task writes `cancelled` when it
 * comes out of its loop. The POLL is what observes the transition, so a caller
 * must keep polling after this resolves rather than assuming the run is settled.
 *
 * A non-2xx throws with the backend's message (Standing Rule 1). The two 409s a
 * caller can meet are "it is not running any more" and "no live task owns it";
 * both name the state in the message, which is what the panel surfaces.
 */
export async function cancelScanRun(
  slug: string,
  scenarioId: string,
  runId: string,
): Promise<void> {
  const response = await authFetch(
    `${scenarioBase(slug, scenarioId)}/scan-runs/${encodeURIComponent(runId)}/cancel`,
    { method: "POST" },
  );
  if (!response.ok) {
    throw new Error(`Failed to stop the scan${await readErrorMessage(response)}`);
  }
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
): Promise<ScanRunList> {
  const response = await authFetch(`${scenarioBase(slug, scenarioId)}/scan-runs`);
  if (!response.ok) {
    throw new Error(`Failed to load scan history${await readErrorMessage(response)}`);
  }
  const body = (await response.json()) as {
    runs?: ScanRunHeader[];
    wording?: ScanWording;
  };
  // `wording` is `null` rather than invented when the payload predates it: the
  // table renders no control it has no words for, which is the absent-not-fake
  // law — a button labelled with a compiled-in fallback would be a literal on a
  // surface the configuration law covers.
  return { runs: body.runs ?? [], wording: body.wording ?? null };
}

/** The history list and the words its own controls speak. */
export type ScanRunList = {
  runs: ScanRunHeader[];
  wording: ScanWording | null;
};

/**
 * Every stored string the scan panel renders (backend `ScanPanelWording`).
 *
 * Named for the SURFACE rather than the history row it started as: 2.15 shipped
 * two strings, and the projection added the collapsed card's summary and the
 * numbers-only report's seven.
 */
export type ScanWording = {
  view_label: string;
  /** Carries `{run}` — filled with the row's own when-label, which is formatted
   *  in the reader's locale and is therefore the one part the server cannot. */
  delete_confirm_template: string;
  /** The one line a COLLAPSED scan card shows. Carries `{when}`, `{model}` and
   *  `{count}` — the three facts that let a human decide not to open it. */
  card_collapsed_summary_template: string;
  /** The line under the report's heading, saying it needs no click. */
  report_advisory_note: string;
  /** The report's LIVE proposed line. Carries `{count}`. Kept separate from the
   *  frozen conservation sentence on purpose: everything above it describes what
   *  the run did and never changes, and this falls as you rule. */
  report_proposed_line_template: string;
  /** The five tile captions, in display order. */
  report_tile_gathered: string;
  report_tile_folded: string;
  report_tile_set_aside: string;
  report_tile_judged: string;
  report_tile_proposed: string;
  /** The failed tile's caption. Rendered only when the count is nonzero — a
   *  permanent zero tile reads as decoration. */
  report_tile_failed: string;
  /** The two status pills. A run whose every judged call failed reads FAILED and
   *  does not project; anything else reads Complete. */
  status_complete_label: string;
  status_failed_label: string;
  /** The collapsed card's one line when the latest run FAILED. Carries `{when}`,
   *  `{model}` and `{count}` (the failed count). */
  card_collapsed_failed_template: string;

  // ── The scenario-facts header (2026-09-11) ─────────────────────────────────
  /** The left half of the split Scan|model pill. */
  header_scan_label: string;
  /** The pill that opens the run history. */
  header_history_label: string;
  /** Why the scan control is refused while a run is in flight. */
  header_running_notice: string;
  /** Why the scan control is refused when the catalogue offers nothing. */
  header_no_model_notice: string;
  /** The honest last-scan line. Carries `{when}`, `{status}` and `{count}`. */
  header_last_scan_template: string;
  /** The same line for a run that never reached its pool read. Carries
   *  `{when}` and `{status}` — and deliberately no `{count}`, because a run
   *  that died before reading the pool did not read zero of it. */
  header_last_scan_no_count_template: string;
  /** The four run states as the last-scan line says them. Lowercase: they sit
   *  mid-sentence, where the `status_*_label` pills would read as proper nouns. */
  header_status_completed: string;
  header_status_cancelled: string;
  header_status_failed: string;
  header_status_running: string;
  /** The confirmation with a measured estimate. Carries `{model}`, `{count}`
   *  and `{minutes}`. */
  header_confirm_timed_template: string;
  /** The confirmation with no estimate. Carries `{model}` and `{count}`. */
  header_confirm_template: string;
  /** The confirmation with no candidate count. Carries `{model}`. */
  header_confirm_no_count_template: string;
  /** The button that starts the run — the only control wired to it. */
  header_confirm_run_label: string;
  /** The button that dismisses the confirmation. */
  header_confirm_cancel_label: string;
};

// There is no merge client (2026-08-08). A completed run's admitted verdicts
// reach the queue as a read-time projection served with the cards, and the
// human's ruling is the only write — so there is nothing here to POST.

/** Delete one scan run (and its per-candidate verdicts, which cascade backend-side).
 *
 *  DELETE is idempotent-shaped from the caller's view but the backend distinguishes
 *  "deleted" (204) from "no such run in this scenario" (404); a non-2xx throws with
 *  the backend message (Standing Rule 1 — a failed delete is observable, never
 *  swallowed). Returns nothing on success (the backend sends 204 No Content). */
export async function deleteScanRun(
  slug: string,
  scenarioId: string,
  runId: string,
): Promise<void> {
  const response = await authFetch(
    `${scenarioBase(slug, scenarioId)}/scan-runs/${encodeURIComponent(runId)}`,
    { method: "DELETE" },
  );
  if (!response.ok) {
    throw new Error(`Failed to delete scan run${await readErrorMessage(response)}`);
  }
}

/** The scan catalog: the models on offer, and why any were withheld.
 *
 *  `warnings` is empty on a healthy deployment. It is non-empty when the backend
 *  refused to list a row — today, one whose stored `billing_class` it cannot read
 *  — and it must be SHOWN: a picker one row shorter than the database looks
 *  exactly like a complete one, and the person who can repair the row is the one
 *  reading the screen (task 1.7B). */
export type ScanCatalog = {
  models: ScanModel[];
  warnings: string[];
};

/** Fetch the SCAN model catalog for the picker (active AND scan_eligible ids).
 *
 *  Uses the dedicated `/api/scan/models` endpoint — NOT `/api/chat/models` — so
 *  the scan picker shows only scan-eligible models (retired-but-extraction-active
 *  Claude rows are filtered out), while the chat dropdown is left untouched. */
export async function fetchScanModels(): Promise<ScanCatalog> {
  const response = await authFetch(`${API_BASE_URL}/api/scan/models`);
  if (!response.ok) {
    throw new Error(`Failed to load models${await readErrorMessage(response)}`);
  }
  const body = (await response.json()) as {
    models?: ScanModel[];
    warnings?: string[];
  };
  return { models: body.models ?? [], warnings: body.warnings ?? [] };
}
