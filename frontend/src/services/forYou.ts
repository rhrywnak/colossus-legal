// forYou.ts — the three calls the "For you" page makes (CC_TASK_FOR_YOU_v1 L1).
//
// One person's list across every deck in the case, the badge's number, and the
// write that clears a question when she opens it.
//
// ## Every string on the page is the SERVER's
//
// A row arrives with its deck line, its body and its byline already composed,
// and the tab labels arrive with their counts in them. Nothing here fills a
// template, and nothing here decides what a row says — see
// `backend/src/services/for_you_rows.rs` for why.
//
// Every call goes through `authFetch` (its own AbortController timeout) and
// every failure throws a sentence naming what did not happen. Nothing is
// swallowed: the page shows the sentence.

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { readErrorMessage } from "./fetchUtils";
import { PRACTICE_TIMEOUT_MS } from "./practice";

/** Whose list this viewer is served. Mirrors `ForYouSide`. */
export type ForYouSide = "reviewers" | "witness" | "none";

/** Which heading a row sits under. Decided on the server, in the case's timezone. */
export type ForYouDay = "today" | "yesterday" | "earlier";

/** One row. Checked BY EYE against `ForYouRowDto`. */
export type ForYouRow = {
  kind: string;
  item_id: string;
  scenario_id: string;
  /** Absent for a note about a whole scenario, which opens the deck instead. */
  question_id?: string;
  deck_line: string;
  body: string;
  byline: string;
  when: string;
  day: ForYouDay;
  read: boolean;
};

/**
 * The page's words, as the browser receives them.
 *
 * A map rather than a named type for the reason `PracticeWording` is one: the
 * keys are settings rows and the set moves with the store. `wordingOf` below is
 * the single reader, and it throws BY NAME on a key the payload does not carry
 * — never a silent `undefined` rendered as an empty label.
 */
export type ForYouWording = Record<string, string>;

/** The whole page. Checked by eye against `ForYouPayload`. */
export type ForYouPage = {
  side: ForYouSide;
  subtitle: string;
  tab_unread_label: string;
  tab_everything_label: string;
  unread_count: number;
  unread: ForYouRow[];
  everything: ForYouRow[];
  empty_hint: string;
  /** Absent when this side has never had an item — the line is then not shown. */
  empty_last?: string;
  wording: ForYouWording;
};

/** The badge's number. Checked by eye against `ForYouSummaryDto`. */
export type ForYouSummary = {
  side: ForYouSide;
  unread_count: number;
};

/** One stored string, or a sentence naming the key that is missing. */
export function wordingOf(wording: ForYouWording, key: string): string {
  const value = wording[key];
  if (typeof value !== "string" || value.trim() === "") {
    throw new Error(
      `The For you page has no stored wording for "${key}". The backend and ` +
        `this build disagree about the wording store; report it to the site ` +
        `administrator.`,
    );
  }
  return value;
}

/** Throw a sentence naming the failed act, or return the decoded body. */
async function orThrow<T>(response: Response, what: string): Promise<T> {
  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(`${what} (HTTP ${response.status}${detail}).`);
  }
  return (await response.json()) as T;
}

/**
 * The page, in one request.
 *
 * The shape check is deliberate and narrow: an absent `wording` would make
 * every label on the page throw one by one, and an absent `unread` would render
 * as "nothing waiting" — a confident, false screen. Both are contract
 * mismatches and both say so here, once.
 */
export async function fetchForYou(slug: string): Promise<ForYouPage> {
  const what = "The For you list could not be loaded";
  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/for-you`,
    { timeoutMs: PRACTICE_TIMEOUT_MS },
  );
  const body = await orThrow<Partial<ForYouPage>>(response, what);
  if (
    typeof body.side !== "string" ||
    !Array.isArray(body.unread) ||
    !Array.isArray(body.everything) ||
    typeof body.wording !== "object" ||
    body.wording === null
  ) {
    throw new Error(`${what}: the response is not a For you page — backend/frontend contract mismatch.`);
  }
  return body as ForYouPage;
}

/** The badge's one number. Fetched once per app load and after a read-clear. */
export async function fetchForYouSummary(slug: string): Promise<ForYouSummary> {
  const what = "The For you count could not be read";
  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/for-you/summary`,
    { timeoutMs: PRACTICE_TIMEOUT_MS },
  );
  const body = await orThrow<Partial<ForYouSummary>>(response, what);
  if (typeof body.unread_count !== "number" || typeof body.side !== "string") {
    throw new Error(`${what}: the response carries no count — backend/frontend contract mismatch.`);
  }
  return body as ForYouSummary;
}

/**
 * Opening a question marks everything on it read, for the signed-in person.
 *
 * Returns how many rows were newly written — 0 when she had already read it,
 * which is a legitimate state and why this is a number rather than a boolean.
 */
export async function markQuestionSeen(questionId: string): Promise<number> {
  const what = "The question could not be marked as read";
  const response = await authFetch(
    `${API_BASE_URL}/api/practice/questions/${encodeURIComponent(questionId)}/seen`,
    { method: "POST", timeoutMs: PRACTICE_TIMEOUT_MS },
  );
  const body = await orThrow<{ marked?: number }>(response, what);
  if (typeof body.marked !== "number") {
    throw new Error(`${what}: the response carries no count — backend/frontend contract mismatch.`);
  }
  return body.marked;
}
