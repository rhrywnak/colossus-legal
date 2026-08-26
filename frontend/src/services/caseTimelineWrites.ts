// =============================================================================
// caseTimelineWrites.ts — every write the case chronology makes
// =============================================================================
//
// Chronology Phase C, §C1/§C3. A sibling of `caseTimeline.ts` rather than a
// growth of it, for the reason `practiceEditor.ts` is a sibling of
// `practice.ts`: that module SERVES the page and this one changes what the page
// is about, and the read module is already at the size where one more concern
// makes it two files anyway.
//
// ## ⚑ EVERY WRITE RETURNS THE EVENT, AND THE CALLER USES THAT
//
// §C3: "After any write, the list/page reflects the server's response — no
// optimistic divergence." So every function below resolves to the server's
// composed `CaseTimelineEvent`, and no caller is given the option of applying
// its own guess. That is not defensive style — the server trims, normalises and
// clears things (a padded title, an empty fact, a de-duplicated tag list), and a
// surface that guessed would disagree with itself after the next reload.
//
// ## Nothing is swallowed
//
// Every function throws a sentence naming what failed and quoting the server's
// own message where there is one (`readErrorMessage`). The pages render that
// sentence through the stored `write_failed_template`; a failed write never ends
// as a button that quietly does nothing.
//
// ## Same idioms as its siblings
//
// `authFetch` with an `AbortController` timeout, `encodeURIComponent` on every
// path parameter, no hardcoded base URL.

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import type { CaseTimelineEvent } from "./caseTimeline";
import { readErrorMessage } from "./fetchUtils";

/**
 * The standing ceiling for a chronology write.
 *
 * 30s, the house normal (CLAUDE.md §13) — these are small writes against a
 * table of a few hundred rows, not a synthesis. Named rather than repeated at
 * nine call sites so raising it is one edit.
 */
const TIMELINE_WRITE_TIMEOUT_MS = 30000;

const TIMELINE_PATH = "/api/timeline";

/** One link, as the form and the event page submit it. */
export type SubmittedLink = {
  target_type: string;
  target_id: string;
  label?: string;
  /** Absent is MEANINGFUL — the surface marks it "no pinpoint" (design R9). */
  pinpoint?: string;
};

/**
 * One event as the form submits it.
 *
 * `links` is only accepted on a CREATE — the server refuses it on an edit,
 * deliberately, because an edit that replaced an event's link set would delete a
 * colleague's link while somebody re-typed a title.
 */
export type SubmittedEvent = {
  event_date: string;
  title: string;
  phase: string;
  fact?: string;
  date_precision?: string;
  approximate?: boolean;
  tags?: string[];
  links?: SubmittedLink[];
};

/** One document the picker offers. */
export type DocumentChoice = { id: string; title: string };

/**
 * The picker's answer.
 *
 * ⚑ `total` is how many matched, `matches` is how many came back. The surface
 * says so when they differ: a truncated list that looked complete is how
 * somebody links the wrong document with no idea a better match was cut off.
 */
export type DocumentSearchResult = {
  matches: DocumentChoice[];
  total: number;
  shown_limit: number;
};

/**
 * Send one request and return the event the server says now exists.
 *
 * Every distinct failure — network/timeout, non-2xx, unparseable body — throws
 * its own sentence naming the action (Standing Rule 1). The server's `message`
 * is appended when there is one, which is what carries "no phase named 'apeals'"
 * to the person who typed it.
 */
async function writeEvent(
  path: string,
  method: "POST" | "PUT" | "DELETE",
  body: unknown,
  action: string,
): Promise<CaseTimelineEvent> {
  let response: Response;
  try {
    response = await authFetch(`${API_BASE_URL}${path}`, {
      method,
      headers: body === undefined ? undefined : { "Content-Type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
      timeoutMs: TIMELINE_WRITE_TIMEOUT_MS,
    });
  } catch (err) {
    const cause = err instanceof Error ? err.message : "network error";
    throw new Error(`${action} (${cause}).`);
  }
  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(`${action} (HTTP ${response.status}${detail}).`);
  }
  let payload: unknown;
  try {
    payload = await response.json();
  } catch {
    throw new Error(`${action} — the server's answer was not valid JSON.`);
  }
  return asEvent(payload, action);
}

/**
 * Assert the load-bearing shape of a write's response.
 *
 * A 200 carrying the wrong shape would otherwise become an event with an
 * `undefined` title rendered into the list — a screen that looks like a data
 * problem and is a contract problem. Named here so the failure says which.
 */
function asEvent(payload: unknown, action: string): CaseTimelineEvent {
  const event = payload as Partial<CaseTimelineEvent> | null;
  if (event === null || typeof event.id !== "string" || typeof event.title !== "string") {
    throw new Error(
      `${action} — the server answered without an event id or title. The backend ` +
        `and this build disagree about the payload shape.`,
    );
  }
  return {
    ...(event as CaseTimelineEvent),
    links: event.links ?? [],
    tags: event.tags ?? [],
    notes: event.notes ?? [],
    history: event.history ?? [],
  };
}

/** Create one event, with any links the form picked. */
export function createTimelineEvent(event: SubmittedEvent): Promise<CaseTimelineEvent> {
  return writeEvent(`${TIMELINE_PATH}/events`, "POST", event, "That event was not saved");
}

/**
 * Edit one event.
 *
 * `links` is stripped here rather than relied on being absent: the form is one
 * component used for both add and edit, and sending a create's payload to the
 * edit endpoint would be refused by the server (`deny_unknown_fields`) with a
 * message about a field the author never saw.
 */
export function updateTimelineEvent(
  id: string,
  event: SubmittedEvent,
): Promise<CaseTimelineEvent> {
  const { links: _links, ...editable } = event;
  return writeEvent(
    `${TIMELINE_PATH}/events/${encodeURIComponent(id)}`,
    "PUT",
    editable,
    "That edit was not saved",
  );
}

/**
 * Soft-delete one event (design R10).
 *
 * There is no confirm dialog anywhere, by ruling. The response carries the event
 * with `deleted_at` set, which is what the undo line is drawn from — the surface
 * never infers "it is gone" from a status code.
 */
export function deleteTimelineEvent(id: string): Promise<CaseTimelineEvent> {
  return writeEvent(
    `${TIMELINE_PATH}/events/${encodeURIComponent(id)}`,
    "DELETE",
    undefined,
    "That event was not deleted",
  );
}

/** Undo a delete. The safety R10 chose instead of a confirm dialog. */
export function undeleteTimelineEvent(id: string): Promise<CaseTimelineEvent> {
  return writeEvent(
    `${TIMELINE_PATH}/events/${encodeURIComponent(id)}/undelete`,
    "POST",
    {},
    "That event was not restored",
  );
}

/** Link one target to one event (design R9). */
export function linkTimelineDocument(
  id: string,
  link: SubmittedLink,
): Promise<CaseTimelineEvent> {
  return writeEvent(
    `${TIMELINE_PATH}/events/${encodeURIComponent(id)}/links`,
    "POST",
    link,
    "That document was not linked",
  );
}

/**
 * Remove one link, addressed by its natural key.
 *
 * The key rides the query string rather than a body: a DELETE with a body is
 * legal and widely mishandled, and the key is the address of the thing being
 * removed.
 */
export function unlinkTimelineDocument(
  id: string,
  targetType: string,
  targetId: string,
): Promise<CaseTimelineEvent> {
  const query = new URLSearchParams({ target_type: targetType, target_id: targetId });
  return writeEvent(
    `${TIMELINE_PATH}/events/${encodeURIComponent(id)}/links?${query.toString()}`,
    "DELETE",
    undefined,
    "That link was not removed",
  );
}

/** Add one attributed note (design R8). */
export function addTimelineNote(id: string, note: string): Promise<CaseTimelineEvent> {
  return writeEvent(
    `${TIMELINE_PATH}/events/${encodeURIComponent(id)}/notes`,
    "POST",
    { note },
    "That note was not saved",
  );
}

/** Retire one of your own notes. The server refuses anybody else's. */
export function deleteTimelineNote(id: string, noteId: string): Promise<CaseTimelineEvent> {
  return writeEvent(
    `${TIMELINE_PATH}/events/${encodeURIComponent(id)}/notes/${encodeURIComponent(noteId)}`,
    "DELETE",
    undefined,
    "That note was not deleted",
  );
}

/**
 * Search the document store for the picker.
 *
 * A READ, so it needs no guard — but it throws like the writes do, because a
 * picker that silently returned nothing on a failed request is a picker that
 * says a document does not exist when the truth is that nobody asked.
 */
export async function searchTimelineDocuments(q: string): Promise<DocumentSearchResult> {
  const query = new URLSearchParams({ q });
  let response: Response;
  try {
    response = await authFetch(`${API_BASE_URL}${TIMELINE_PATH}/documents?${query.toString()}`, {
      timeoutMs: TIMELINE_WRITE_TIMEOUT_MS,
    });
  } catch (err) {
    const cause = err instanceof Error ? err.message : "network error";
    throw new Error(`The document search failed (${cause}).`);
  }
  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(`The document search failed (HTTP ${response.status}${detail}).`);
  }
  let payload: unknown;
  try {
    payload = await response.json();
  } catch {
    throw new Error(`The document search did not come back as valid JSON.`);
  }
  const result = payload as Partial<DocumentSearchResult> | null;
  if (result === null || !Array.isArray(result.matches)) {
    throw new Error(
      `The document search came back without its matches. The backend and this ` +
        `build disagree about the payload shape.`,
    );
  }
  return {
    matches: result.matches,
    // `total` absent would make the cap invisible, which is the one thing this
    // payload exists to prevent — so it falls back to what was shown, and the
    // surface then simply never claims anything was cut off.
    total: typeof result.total === "number" ? result.total : result.matches.length,
    shown_limit:
      typeof result.shown_limit === "number" ? result.shown_limit : result.matches.length,
  };
}
