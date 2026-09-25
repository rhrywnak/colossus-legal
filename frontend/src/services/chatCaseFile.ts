// chatCaseFile.ts — the Admin "Chat case file" box's two calls.
//
// Both are administrator-only on the server. Every value arrives ready to show
// (times already in the case's timezone); the box composes sentences around
// them and decides nothing.

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

/**
 * The box's activity lines.
 *
 * ## ⚑ Checked BY EYE against `backend/src/services/chat_keepwarm_button.rs::ActivityDto`
 *
 *   pub last_question_at: Option<String>    → last_question_at: string | null
 *   pub last_question_by: Option<String>    → last_question_by: string | null
 *   pub loaded: bool                        → loaded: boolean
 *   pub loaded_until: Option<String>        → loaded_until: string | null
 *   pub reload_cost_dollars: Option<f64>    → reload_cost_dollars: number | null
 *   pub automatic_line: String              → automatic_line: string
 */
export type ChatCaseFileActivity = {
  last_question_at: string | null;
  last_question_by: string | null;
  loaded: boolean;
  loaded_until: string | null;
  reload_cost_dollars: number | null;
  /**
   * The automatic ping's line, finished by the server from the stored wording:
   * on with its hours, on but paused until tomorrow, or off
   * (CC_TASK_CACHE_KEEPWARM_v1). Shown as-is; the box decides nothing.
   */
  automatic_line: string;
};

/** One tap's result (`KeepLoadedDto`): `outcome` is `read` or `wrote`. */
export type KeepLoadedResult = {
  outcome: "read" | "wrote";
  loaded_until: string;
  cost_dollars: number | null;
  activity: ChatCaseFileActivity;
};

/** An ordinary read: the same 30s every read in this app uses. */
const READ_TIMEOUT_MS = 30_000;
/**
 * A ping that has to reload the whole case file took 13–15s when measured
 * (CC_TASK_CACHE_KEEPWARM_v1 Stage P). 90s is the app's long-call budget, which
 * leaves room for a slow provider without leaving the button spinning forever.
 */
const PING_TIMEOUT_MS = 90_000;

const CASE_FILE_URL = `${API_BASE_URL}/api/admin/chat-case-file`;

/** Throw a named error for a non-OK response, keeping the server's message. */
async function failure(res: Response, what: string): Promise<Error> {
  const detail = await res
    .text()
    .catch((e: unknown) => `(the error body could not be read: ${String(e)})`);
  return new Error(`${what} failed with HTTP ${res.status}: ${detail}`);
}

/** Read the box's activity lines. Throws on any failure; the box shows it. */
export async function getChatCaseFile(): Promise<ChatCaseFileActivity> {
  const res = await authFetch(CASE_FILE_URL, { timeoutMs: READ_TIMEOUT_MS });
  if (!res.ok) throw await failure(res, "Reading the chat case file");
  return (await res.json()) as ChatCaseFileActivity;
}

/** Send one keep-loaded ping. Throws on any failure; the box shows it. */
export async function keepLoaded(): Promise<KeepLoadedResult> {
  const res = await authFetch(`${CASE_FILE_URL}/keep-loaded`, {
    method: "POST",
    timeoutMs: PING_TIMEOUT_MS,
  });
  if (!res.ok) throw await failure(res, "Keeping the chat case file loaded");
  return (await res.json()) as KeepLoadedResult;
}
