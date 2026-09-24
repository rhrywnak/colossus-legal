// aiJobs.ts — Admin → Overview's "Which model and which instructions each AI
// job uses" panel, and Admin → Data's "Last document run" (CC_TASK_MODEL_JOBS_PANEL_v1).
//
// Both are administrator-only on the server. Every string arrives finished: the
// page draws it and decides nothing (Standing Rule 12).

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

/** An ordinary read: the same 30s every read in this app uses. */
const READ_TIMEOUT_MS = 30_000;
/**
 * A save: the same 30s budget. Save may price a case-file reload, which reuses
 * the provider's remembered count, so it answers in about the time of a read.
 */
const SAVE_TIMEOUT_MS = 30_000;

/**
 * One entry of a dropdown.
 *
 * ## ⚑ Checked BY EYE against `backend/src/dto/ai_jobs.rs`
 *
 *   AiJobOptionDto { value: String, label: String, marker: Option<String>, current: bool }
 */
export type AiJobOption = {
  value: string;
  label: string;
  marker: string | null;
  current: boolean;
};

/**
 * What sits under a control's label — `AiJobControlBody`, internally tagged by
 * `kind` (`#[serde(tag = "kind", rename_all = "snake_case")]`).
 */
export type AiJobControlBody =
  | {
      kind: "choose";
      key: string;
      current: string;
      current_label: string;
      options: AiJobOption[];
      foot: string | null;
    }
  | { kind: "fixed"; text: string }
  | { kind: "empty"; text: string };

/** `AiJobControlDto`: the label, with the body flattened beside it. */
export type AiJobControl = { label: string } & AiJobControlBody;

/** `AiJobDto`: one row of the panel. */
export type AiJob = {
  job: string;
  title: string;
  description: string;
  first: AiJobControl;
  instructions: AiJobControl;
  meta: string;
  warnings: string[];
  /** "was Claude Opus 5" — set when the history holds a change to undo. */
  was: string | null;
  /** The Switch back label beside `was`; null exactly when `was` is. */
  switch_back: string | null;
};

/** `AiJobsLinkDto`: `target` is `models` | `files` | `data`. */
export type AiJobsLink = { target: string; lead: string; label: string };

/** `AiJobsPanelDto`. */
export type AiJobsPanel = {
  title: string;
  intro: string;
  save_label: string;
  jobs: AiJob[];
  links: AiJobsLink[];
  /** Settings that name a model but belong to no job yet (Law 22). */
  others_title: string;
  others: string[];
};

/** `AiJobChangeDto`: one of the job's settings, and the value Save writes. */
export type AiJobChange = { key: string; value: string };

/** `AiJobSavedDto`: board 3's confirmation, and the panel as it now stands. */
export type AiJobSaved = {
  job: string;
  confirmation: string;
  switch_back: string;
  panel: AiJobsPanel;
};

/**
 * A refusal a person caused (HTTP 4xx), carrying the server's own sentence —
 * "Claude Opus 5.5 is used by …", "Nothing changed: …". The panel shows it as
 * it is. Any other failure is a plain `Error` and the panel says so generally.
 */
export class AiJobRefusal extends Error {
  constructor(message: string) {
    super(message);
    this.name = "AiJobRefusal";
  }
}

/** `LastRunDto` (`backend/src/dto/last_run.rs`). */
export type LastRun = {
  empty: string | null;
  cards: { value: string; label: string }[];
  progress: string;
  columns: string[];
  steps: { label: string; avg: string; runs: string; failed: string }[];
  summary: string;
  warnings: string[];
};

/** Throw a named error for a non-OK response, keeping the server's message. */
async function failure(res: Response, what: string): Promise<Error> {
  const detail = await res
    .text()
    .catch((e: unknown) => `(the error body could not be read: ${String(e)})`);
  return new Error(`${what} failed with HTTP ${res.status}: ${detail}`);
}

/**
 * A 4xx becomes an `AiJobRefusal` with the server's `message`; anything else a
 * named `Error`. A body that cannot be read is said to be unreadable, never
 * treated as an empty refusal.
 */
async function writeFailure(res: Response, what: string): Promise<Error> {
  if (res.status >= 400 && res.status < 500) {
    const body: unknown = await res.json().catch((e: unknown) => {
      console.error(`${what}: the refusal body could not be read`, e);
      return null;
    });
    const message =
      typeof body === "object" && body !== null && "message" in body
        ? String((body as { message: unknown }).message)
        : "";
    if (message.length > 0) return new AiJobRefusal(message);
  }
  return failure(res, what);
}

/** Save one job's changed controls (board 3). Throws on any failure. */
export async function saveAiJob(job: string, changes: AiJobChange[]): Promise<AiJobSaved> {
  const res = await authFetch(`${API_BASE_URL}/api/admin/ai-jobs/${encodeURIComponent(job)}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ changes }),
    timeoutMs: SAVE_TIMEOUT_MS,
  });
  if (!res.ok) throw await writeFailure(res, "Saving the AI job");
  return (await res.json()) as AiJobSaved;
}

/** Restore what a job used before its last change. Throws on any failure. */
export async function switchBackAiJob(job: string): Promise<AiJobSaved> {
  const res = await authFetch(
    `${API_BASE_URL}/api/admin/ai-jobs/${encodeURIComponent(job)}/switch-back`,
    { method: "POST", timeoutMs: SAVE_TIMEOUT_MS },
  );
  if (!res.ok) throw await writeFailure(res, "Switching the AI job back");
  return (await res.json()) as AiJobSaved;
}

/** Read the jobs panel. Throws on any failure; the panel shows it. */
export async function getAiJobs(): Promise<AiJobsPanel> {
  const res = await authFetch(`${API_BASE_URL}/api/admin/ai-jobs`, {
    timeoutMs: READ_TIMEOUT_MS,
  });
  if (!res.ok) throw await failure(res, "Reading the AI jobs panel");
  return (await res.json()) as AiJobsPanel;
}

/** Read the last document run. Throws on any failure; the tab shows it. */
export async function getLastRun(): Promise<LastRun> {
  const res = await authFetch(`${API_BASE_URL}/api/admin/pipeline/last-run`, {
    timeoutMs: READ_TIMEOUT_MS,
  });
  if (!res.ok) throw await failure(res, "Reading the last document run");
  return (await res.json()) as LastRun;
}
