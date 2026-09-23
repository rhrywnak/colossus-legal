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
import type { SweptItem } from "../components/practice/deckSweep";

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
 * "Done reviewing": record that this reviewer has READ the items the page showed.
 *
 * ## Domain note: the ids, not a moment (CC_TASK_FOR_YOU_v1 L2)
 *
 * It used to move one timestamp for the whole deck. It now sends exactly what
 * was on screen, because "never an item that arrived after the page loaded" is
 * a promise only a list of ids can keep — `now()` covers the note that arrived
 * while the deck was being read.
 *
 * `servedAt` rides along for the one item no row can name: a note about the
 * whole scenario. It is the SERVER's own stamp from the payload this page was
 * drawn from, handed straight back, so no browser clock decides anything.
 *
 * Returns how many rows the press actually WROTE — zero when everything shown
 * had already been read, which is idempotence working rather than a failure.
 * The caller re-reads the deck, so the count the bar shows is the server's.
 */
export async function markDeckReviewed(
  slug: string,
  scenarioId: string,
  items: SweptItem[],
  servedAt: string,
): Promise<number> {
  const response = await send(
    "PUT",
    `/api/cases/${encodeURIComponent(slug)}/scenarios/${encodeURIComponent(scenarioId)}` +
      "/practice/review-cursor",
    { items, served_at: servedAt },
  );
  const body = await orThrow<{ marked?: number }>(response, "The deck was not marked reviewed");
  if (typeof body.marked !== "number") {
    throw new Error("The review response carried no count — backend/frontend contract mismatch.");
  }
  return body.marked;
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
