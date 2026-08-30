// =============================================================================
// caseTimelineSubsets.ts — the timeline's named subsets, read and written
// =============================================================================
//
// Timeline subsets, task 2. A sibling of `caseTimeline.ts` and
// `caseTimelineWrites.ts` for the same reason those are siblings of each other:
// this module is about a different noun. The chronology is events; a subset is a
// named, ordered list of REFERENCES to events that already exist. It never
// carries a copy of one (TIMELINE_SUBSET_DESIGN_v1 §4), which is why every shape
// below holds an `event_id` and the event itself arrives composed by the server.
//
// ## ⚑ THE REPLACE IS THE WHOLE SET, ALWAYS
//
// `replaceSubsetEvents` sends the COMPLETE ordered set, never a per-row add or
// remove. That is T1's write contract and it is not an implementation detail:
// the picker is a screen where somebody ticks, unticks and drags for a minute
// and then presses Save once, so one human act becomes one history row instead
// of a dozen. A per-row endpoint would also leave the subset legal-but-wrong at
// every intermediate step — the exact state a reader on another screen would
// happen to load.
//
// ## Nothing is swallowed
//
// Every function throws a sentence naming what failed and quoting the server's
// own message where there is one, through the same `readErrorMessage` its
// siblings use. T1 answers 400/409/422 with the offending field and value, so
// "a subset needs a name" reaches the person who left it blank rather than
// dying in a console.
//
// ## Same idioms as its siblings
//
// `authFetch` with an `AbortController` timeout, `encodeURIComponent` on every
// path parameter, no hardcoded base URL.

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import type { TimelineEvent } from "./caseTimeline";
import { readErrorMessage } from "./fetchUtils";

/**
 * The standing ceiling for a subset read or write.
 *
 * 30s, the house normal (CLAUDE.md §13) — the same value
 * `caseTimelineWrites.ts` names, for the same reason: these are small
 * statements against tables of a few hundred rows, not a synthesis.
 */
const SUBSET_TIMEOUT_MS = 30000;

const SUBSETS_PATH = "/api/timeline/subsets";

/** One subset as the Subsets section lists it. */
export type SubsetSummary = {
  id: string;
  name: string;
  description: string;
  /** Every reference the subset holds, gaps included. */
  event_count: number;
  /** How many of those events have been removed from the chronology. */
  gap_count: number;
  /** The scenario codes carrying this subset — "S-11", "S-12". */
  carried_by: string[];
  created_by: string;
  created_at: string;
  updated_by: string;
  updated_at: string;
};

/**
 * One event inside a subset.
 *
 * `removed` is the gap flag: the reference is still here and still counted, but
 * the event behind it has been soft-deleted on the chronology (design R1). The
 * row is MARKED, never dropped — dropping it would silently shorten a story
 * somebody counted.
 */
export type SubsetEvent = {
  event: TimelineEvent;
  /** The author's one line on why this event is in the story. May be empty. */
  subset_note: string;
  removed: boolean;
};

/** One subset in full, with its ordered events joined. */
export type SubsetDetail = {
  id: string;
  name: string;
  description: string;
  events: SubsetEvent[];
  carried_by: string[];
  event_count: number;
  gap_count: number;
  created_by: string;
  created_at: string;
  updated_by: string;
  updated_at: string;
  /** Present only on the answer to a DELETE, which is what drives the undo line. */
  deleted_at?: string;
};

/**
 * One reference as the picker submits it.
 *
 * `position` is 1-based and dense — see `toSubsetPayload` in `subsetPicker.ts`,
 * which is the only thing that builds these. `note` is omitted rather than sent
 * empty, because an absent note and a note somebody blanked are the same fact
 * and the wire should say it once.
 */
export type SubsetEventRef = {
  event_id: string;
  position: number;
  note?: string;
};

/** Read one JSON body, or throw a sentence naming the resource. */
async function readSubsetJson(path: string, label: string): Promise<unknown> {
  let response: Response;
  try {
    response = await authFetch(`${API_BASE_URL}${path}`, { timeoutMs: SUBSET_TIMEOUT_MS });
  } catch (err) {
    const cause = err instanceof Error ? err.message : "network error";
    throw new Error(`Failed to load ${label} (${cause}).`);
  }
  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(`Failed to load ${label} (HTTP ${response.status}${detail}).`);
  }
  try {
    return await response.json();
  } catch {
    throw new Error(`${label} did not come back as valid JSON.`);
  }
}

/**
 * Send one subset write and return the subset the server says now exists.
 *
 * Every distinct failure — network/timeout, non-2xx, unparseable body — throws
 * its own sentence naming the action (Standing Rule 1). T1's `message` is
 * appended when there is one, which is what carries "a subset with that name is
 * already on this case" to the person who typed it.
 */
async function writeSubset(
  path: string,
  method: "POST" | "PUT" | "DELETE",
  body: unknown,
  action: string,
): Promise<SubsetDetail> {
  let response: Response;
  try {
    response = await authFetch(`${API_BASE_URL}${path}`, {
      method,
      headers: body === undefined ? undefined : { "Content-Type": "application/json" },
      body: body === undefined ? undefined : JSON.stringify(body),
      timeoutMs: SUBSET_TIMEOUT_MS,
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
  return asSubsetDetail(payload, action);
}

/**
 * Assert the load-bearing shape of a subset response.
 *
 * A 200 carrying the wrong shape would otherwise become a subset with an
 * `undefined` name rendered into the section — a screen that looks like a data
 * problem and is a contract problem. Named here so the failure says which.
 */
function asSubsetDetail(payload: unknown, action: string): SubsetDetail {
  const subset = payload as Partial<SubsetDetail>;
  if (typeof subset.id !== "string" || typeof subset.name !== "string") {
    throw new Error(
      `${action} — the server's answer was not a subset. The backend and this ` +
        `build disagree about the payload shape.`,
    );
  }
  return { ...subset, events: subset.events ?? [] } as SubsetDetail;
}

/** Every live subset on the case, for the Subsets section. */
export async function listSubsets(): Promise<SubsetSummary[]> {
  const data = await readSubsetJson(SUBSETS_PATH, "the timeline subsets");
  if (!Array.isArray(data)) {
    throw new Error(
      `The timeline subsets came back as something other than a list. The ` +
        `backend and this build disagree about the payload shape.`,
    );
  }
  return data as SubsetSummary[];
}

/** One subset in full, with its ordered events joined. */
export async function getSubset(id: string): Promise<SubsetDetail> {
  const data = await readSubsetJson(
    `${SUBSETS_PATH}/${encodeURIComponent(id)}`,
    "this timeline subset",
  );
  return asSubsetDetail(data, "That timeline subset could not be read");
}

/** Create one subset, with the events it arrived with. ONE act, ONE history row. */
export async function createSubset(
  name: string,
  description: string,
  events: SubsetEventRef[],
): Promise<SubsetDetail> {
  return writeSubset(
    SUBSETS_PATH,
    "POST",
    { name, description, events },
    "That subset was not created",
  );
}

/** Edit one subset's name and/or description. */
export async function updateSubset(
  id: string,
  name: string,
  description: string,
): Promise<SubsetDetail> {
  return writeSubset(
    `${SUBSETS_PATH}/${encodeURIComponent(id)}`,
    "PUT",
    { name, description },
    "That subset was not saved",
  );
}

/** REPLACE one subset's ordered event set — the picker's Save. */
export async function replaceSubsetEvents(
  id: string,
  events: SubsetEventRef[],
): Promise<SubsetDetail> {
  return writeSubset(
    `${SUBSETS_PATH}/${encodeURIComponent(id)}/events`,
    "PUT",
    { events },
    "That subset's events were not saved",
  );
}

/** SOFT-delete one subset. The undo line that replaces the row IS the safety. */
export async function deleteSubset(id: string): Promise<SubsetDetail> {
  return writeSubset(
    `${SUBSETS_PATH}/${encodeURIComponent(id)}`,
    "DELETE",
    undefined,
    "That subset was not deleted",
  );
}

/** Restore one soft-deleted subset — the Undo. */
export async function undeleteSubset(id: string): Promise<SubsetDetail> {
  return writeSubset(
    `${SUBSETS_PATH}/${encodeURIComponent(id)}/undelete`,
    "POST",
    undefined,
    "That subset was not restored",
  );
}
