// practiceReviewLoop.ts — the review loop's four writes (CC_TASK_REVIEW_LOOP_v1).
//
// Chuck's side: Done reviewing moves his read cursor on a deck. Both sides:
// a note on an answer or a question, and striking one through.
//
// ## Why its own module
//
// `practice.ts` is over Rule 17's limit already (pre-existing), and these four
// calls are one feature with one audience — the loop between the person
// reviewing a deck and the person rehearsing it.
//
// Every call goes through `authFetch` with `PRACTICE_TIMEOUT_MS` (its
// AbortController timeout), and every failure throws a sentence naming what did
// not happen — the component shows it; nothing here swallows one.

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { readErrorMessage } from "./fetchUtils";
import { PRACTICE_TIMEOUT_MS, type PracticeNote } from "./practice";

/** Throw a sentence naming the failed act, or return the decoded body. */
async function orThrow<T>(response: Response, what: string): Promise<T> {
  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(`${what} (HTTP ${response.status}${detail}).`);
  }
  return (await response.json()) as T;
}

/** A JSON write to one API path. */
function send(method: "POST" | "PUT", path: string, body?: unknown): Promise<Response> {
  return authFetch(`${API_BASE_URL}${path}`, {
    method,
    headers: body === undefined ? undefined : { "Content-Type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
    timeoutMs: PRACTICE_TIMEOUT_MS,
  });
}

/**
 * A stored note, checked. Checked BY EYE against `PracticeNoteDto`: an absent
 * `when` would render a note with no date, and an absent `struck` key would be
 * read as "still standing".
 */
function asNote(body: Partial<PracticeNote>, what: string): PracticeNote {
  if (typeof body.id !== "string" || typeof body.when !== "string" || body.struck === undefined) {
    throw new Error(`${what}: the response is not a note — backend/frontend contract mismatch.`);
  }
  return body as PracticeNote;
}

/**
 * "Done reviewing": move the signed-in user's read cursor on this deck to now.
 *
 * Returns the moment the server recorded. The caller re-reads the deck, so the
 * count the bar shows is always the server's, never a local zero.
 */
export async function markDeckReviewed(slug: string, scenarioId: string): Promise<string> {
  const response = await send(
    "PUT",
    `/api/cases/${encodeURIComponent(slug)}/scenarios/${encodeURIComponent(scenarioId)}` +
      "/practice/review-cursor",
  );
  const body = await orThrow<{ looked_at?: string }>(response, "The deck was not marked reviewed");
  if (typeof body.looked_at !== "string") {
    throw new Error("The review mark response carried no time — backend/frontend contract mismatch.");
  }
  return body.looked_at;
}

/** Write a note on one attempt at a question. */
export async function addAnswerNote(answerId: string, text: string): Promise<PracticeNote> {
  const what = "The note was not saved";
  const response = await send("POST", `/api/practice/answers/${encodeURIComponent(answerId)}/notes`, {
    text,
  });
  return asNote(await orThrow<Partial<PracticeNote>>(response, what), what);
}

/** Write a note on a question rather than on one attempt. */
export async function addQuestionNote(questionId: string, text: string): Promise<PracticeNote> {
  const what = "The note was not saved";
  const response = await send(
    "POST",
    `/api/practice/questions/${encodeURIComponent(questionId)}/notes`,
    { text },
  );
  return asNote(await orThrow<Partial<PracticeNote>>(response, what), what);
}

/** Strike a note through. It stays visible, with the day it was struck. */
export async function strikeNote(noteId: string): Promise<PracticeNote> {
  const what = "The note was not struck";
  const response = await send("PUT", `/api/practice/notes/${encodeURIComponent(noteId)}/strike`);
  return asNote(await orThrow<Partial<PracticeNote>>(response, what), what);
}
