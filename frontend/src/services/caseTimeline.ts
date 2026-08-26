// =============================================================================
// caseTimeline.ts — the case chronology, read from the API
// =============================================================================
//
// Phase B. This module used to fetch a static `/data/timeline.json` baked into
// the frontend image; it now reads `GET /api/timeline`, and the file is gone.
// Everything the timeline surfaces need arrives in ONE request: the phases with
// their descriptions, the tag vocabulary, the events with their links and note
// counts, every string these screens speak, and the scroll-window size.
//
// ## Why one request and not four
//
// The page cannot render a single row without the events, so a second request
// for twenty-nine strings fired at the same instant would buy nothing. The five
// label surfaces (`casePhases`) read the phases out of this same payload, which
// is what lets them keep their promise-cached, one-request-for-everyone shape.
//
// ## Standing Rule 1, and the two defects that die here
//
// The old `/timeline` page did `fetch(...).catch(() => {})` with no timeout.
// Both are gone: `authFetch` arms an `AbortController` at the standing 30s
// ceiling, and every failure below throws a contextual error that the page
// renders. Nothing is swallowed.

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

/** What this build could say about one link's target. */
export type LinkResolution = "resolves" | "missing" | "unchecked";

/** One tag of the case's vocabulary, from `chronology_tags`. */
export type TimelineTag = {
  id: string;
  label: string;
  color: string;
  sort_order: number;
};

/** One phase of the case, from `chronology_phases`. */
export type TimelinePhase = {
  id: string;
  label: string;
  date_range: string;
  color: string;
  /** The muted subtitle under the phase header (design R14). */
  description?: string;
  sort_order: number;
};

/** One link from an event to its evidence. */
export type TimelineLink = {
  target_type: string;
  target_id: string;
  label?: string;
  /** Absent is MEANINGFUL — the surface marks it (design R9). */
  pinpoint?: string;
  resolution: LinkResolution;
};

/** One dated fact. */
export type TimelineEvent = {
  id: string;
  event_date: string;
  date_precision: string;
  approximate: boolean;
  /** The phase slug — a real column with a foreign key, never a bag key. */
  phase: string;
  title: string;
  fact?: string;
  attributes: Record<string, unknown>;
  tags: string[];
  links: TimelineLink[];
  note_count: number;
  created_by?: string;
  created_at: string;
  updated_by?: string;
  updated_at: string;
  /**
   * When this event was soft-deleted, if it was (design R10).
   *
   * ABSENT on every read: the list and the event page never return a deleted
   * event. It is present because the WRITE endpoints answer with this same
   * shape, and a DELETE's response is the event it just deleted — which is what
   * lets the surface replace the card in place with the undo line instead of
   * inferring "it is gone" from a status code (§C3).
   */
  deleted_at?: string;
};

/** One attributed note. */
export type TimelineNote = {
  id: string;
  note: string;
  created_by?: string;
  created_at: string;
};

/** One history entry. Empty for every event until the write endpoints land. */
export type TimelineHistory = {
  id: string;
  action: string;
  snapshot: Record<string, unknown>;
  changed_by?: string;
  changed_at: string;
};

/**
 * Every string these surfaces speak.
 *
 * A `Record` and not a typed interface for the reason the practice page's
 * wording is one: the backend mirror test already pins the field set against
 * the boot loader's declared keys, and a hand-written TypeScript interface
 * would be a third copy of that list with nothing checking it.
 */
export type ChronologyWording = Record<string, string>;

/** The whole `GET /api/timeline` payload. */
export type CaseTimeline = {
  phases: TimelinePhase[];
  tags: TimelineTag[];
  events: TimelineEvent[];
  wording: ChronologyWording;
  phase_window_events: number;
};

/** One event in full, from `GET /api/timeline/events/:id`. */
export type CaseTimelineEvent = TimelineEvent & {
  notes: TimelineNote[];
  history: TimelineHistory[];
};

const TIMELINE_PATH = "/api/timeline";

/**
 * ⚑ THE ONLY USER-VISIBLE STRINGS THIS FEATURE SPEAKS FROM CODE, AND WHY.
 *
 * Every other word on the timeline is a settings row. These three cannot be,
 * and the reason is not laziness — it is a bootstrap: THE WORDING STORE IS
 * DELIVERED BY THE REQUEST THESE SENTENCES DESCRIBE THE FAILURE OF. A stored
 * "could not load" line is readable only once the load succeeded.
 *
 * Two keys were drafted for this and withdrawn rather than seeded unreachable —
 * a row nothing can read is a row that drifts unnoticed forever. They live here
 * instead, in one named place, so the exception is visible and countable rather
 * than sprinkled through two components.
 *
 * `loading` is deliberately not a sentence: it labels a moment nobody reads.
 */
export const BOOTSTRAP_TEXT = {
  loading: "Loading…",
  timelineFailed: (reason: string) =>
    `The case timeline could not be loaded (${reason}). Try reloading the page.`,
  eventFailed: (reason: string) =>
    `That timeline event could not be loaded (${reason}). Try reloading the page.`,
};

/**
 * One stored string, by key.
 *
 * ## ⚑ Named `cw` and not `w`, deliberately
 *
 * The practice surfaces use `w("…")`, and `dto::practice_wording_reach_tests`
 * scans `frontend/src/pages` for exactly that call — requiring every key it
 * finds to be a field of the PRACTICE wire object. A timeline page in the same
 * directory calling `w("page_title")` would fail that scan for a key that was
 * never practice's to carry. `cw(` is invisible to it (the character before the
 * `w` is alphanumeric, which that scanner explicitly rejects) and is scanned by
 * `dto::chronology_wording_reach_tests` instead.
 *
 * Throws by name rather than rendering blank: a control with no label is the
 * standing rule of 2026-08-19's named failure, and a missing key means the
 * backend and this build disagree about the wording store.
 */
export function cw(wording: ChronologyWording, key: string): string {
  const value = wording[key];
  if (typeof value !== "string" || value.trim() === "") {
    throw new Error(
      `The case timeline has no stored wording for "${key}". The backend and ` +
        `this build disagree about the wording store; report it to the site ` +
        `administrator.`,
    );
  }
  return value;
}

/** Fill `{name}` placeholders in a stored template. */
export function fill(template: string, values: Record<string, string | number>): string {
  return Object.entries(values).reduce(
    (out, [name, value]) => out.split(`{${name}}`).join(String(value)),
    template,
  );
}

/**
 * Read one JSON body from the API, or throw something a person can act on.
 *
 * Every distinct failure — network/timeout, non-2xx, unparseable body — throws
 * its own message naming the resource. Nothing is collapsed and nothing is
 * swallowed (Standing Rule 1).
 */
async function readJson(path: string, label: string): Promise<unknown> {
  let response: Response;
  try {
    response = await authFetch(`${API_BASE_URL}${path}`);
  } catch (err) {
    const cause = err instanceof Error ? err.message : "network error";
    throw new Error(`Failed to load ${label} (${cause}).`);
  }
  if (!response.ok) {
    throw new Error(`Failed to load ${label} (HTTP ${response.status}).`);
  }
  try {
    return await response.json();
  } catch {
    throw new Error(`${label} did not come back as valid JSON.`);
  }
}

/**
 * The whole timeline, validated.
 *
 * Asserts the load-bearing shapes are present. A payload missing `wording`
 * would render a page of thrown errors one control at a time; failing here
 * names the problem once, at the boundary.
 */
export async function getCaseTimeline(): Promise<CaseTimeline> {
  const data = (await readJson(TIMELINE_PATH, "the case timeline")) as Partial<CaseTimeline>;

  if (!Array.isArray(data.phases) || !Array.isArray(data.events) || !Array.isArray(data.tags)) {
    throw new Error(
      `The case timeline came back without its phases, tags or events. ` +
        `The backend and this build disagree about the payload shape.`,
    );
  }
  if (!data.wording || typeof data.wording !== "object") {
    throw new Error(
      `The case timeline came back with no wording. The backend and this build ` +
        `disagree about the payload shape.`,
    );
  }
  return {
    phases: data.phases,
    tags: data.tags,
    events: data.events,
    wording: data.wording,
    // A window of zero would render every phase empty and look like a data
    // failure, so an absent or nonsensical number falls back to showing them
    // all — visibly wrong is better than invisibly empty.
    phase_window_events:
      typeof data.phase_window_events === "number" && data.phase_window_events > 0
        ? data.phase_window_events
        : data.events.length,
  };
}

/** One event in full, for the event page. */
export async function getTimelineEvent(id: string): Promise<CaseTimelineEvent> {
  const data = (await readJson(
    `${TIMELINE_PATH}/events/${encodeURIComponent(id)}`,
    "this timeline event",
  )) as Partial<CaseTimelineEvent>;

  if (typeof data.id !== "string" || typeof data.title !== "string") {
    throw new Error(
      `That timeline event came back without an id or a title. The backend and ` +
        `this build disagree about the payload shape.`,
    );
  }
  return {
    ...(data as CaseTimelineEvent),
    links: data.links ?? [],
    tags: data.tags ?? [],
    notes: data.notes ?? [],
    history: data.history ?? [],
  };
}

/** One phase reduced to what a home-page pill renders. */
export type PhaseSummary = {
  id: string;
  label: string;
  date_range: string;
  color: string;
  eventCount: number;
};

/**
 * Reduce the timeline into per-phase pill summaries, and say what did not fit.
 *
 * ## ⚑ The band no longer drops an event in silence
 *
 * Every event is counted against its phase; any event whose phase has no row is
 * counted in `unmatched` and the band renders a marker saying so (design B6).
 * Until Phase B this function counted such an event NOWHERE and showed nothing
 * — an event nobody could see was an event nobody could fix.
 */
export function buildPhaseSummaries(data: CaseTimeline): {
  phases: PhaseSummary[];
  matched: number;
  unmatched: number;
} {
  const known = new Set(data.phases.map((phase) => phase.id));
  const phases = data.phases.map((phase) => ({
    id: phase.id,
    label: phase.label,
    date_range: phase.date_range,
    color: phase.color,
    eventCount: data.events.filter((event) => event.phase === phase.id).length,
  }));
  const matched = data.events.filter((event) => known.has(event.phase)).length;
  return { phases, matched, unmatched: data.events.length - matched };
}
