// =============================================================================
// scenarioTimeline.ts — the dock's own reads and writes
// =============================================================================
//
// The dock is self-contained by ruling (2026-08-30): it takes a case slug and a
// scenario id and NOTHING else, and fetches its own data. That is what lets one
// component serve five scenario surfaces which share no header component and no
// read between them — see the T3 report for what each of the five actually
// calls. No page passes it data; no page's existing read changes.
//
// ## ⚑ THE WORDS RIDE THE BUTTON'S READ
//
// `GET /cases/:slug/scenarios/:id/subsets` answers `{ subsets, wording }`. The
// dock has to make that call anyway — it is how it knows whether to draw
// anything at all — so its whole vocabulary arrives with it, in the SAME shape
// `GET /api/timeline` serves. One request, not two, and no second wording shape
// to drift from the first.
//
// ## Nothing is swallowed
//
// Every function throws a sentence naming what failed and quoting the server's
// own message where there is one, through the same `readErrorMessage` its
// siblings use.

import { API_BASE_URL } from "../../services/api";
import { authFetch } from "../../services/auth";
import type { ChronologyWording } from "../../services/caseTimeline";
import { readErrorMessage } from "../../services/fetchUtils";

/** The standing ceiling, the house normal (CLAUDE.md §13). */
const DOCK_TIMEOUT_MS = 30000;

/** One subset a scenario carries, as the button's read lists it. */
export type AttachedSubset = {
  id: string;
  name: string;
  event_count: number;
  gap_count: number;
  /** Attachment order — what the scenario's author chose. */
  position: number;
};

/** What the button's read answers with: the list, and every word the dock speaks. */
export type ScenarioSubsets = {
  subsets: AttachedSubset[];
  wording: ChronologyWording;
};

function scenarioPath(slug: string, scenarioId: string): string {
  return `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/scenarios/${encodeURIComponent(
    scenarioId,
  )}/subsets`;
}

/** Read one JSON body, or throw a sentence naming the resource. */
async function readDockJson(url: string, label: string, init?: RequestInit): Promise<unknown> {
  let response: Response;
  try {
    response = await authFetch(url, { ...init, timeoutMs: DOCK_TIMEOUT_MS });
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
 * What this scenario carries, and the words to draw it with.
 *
 * An empty `subsets` hides the button. It is NOT a 404 — that would mean "there
 * is no such scenario", and a surface collapsing the two would draw a working
 * dock over a scenario that does not exist.
 */
export async function getScenarioSubsets(
  slug: string,
  scenarioId: string,
): Promise<ScenarioSubsets> {
  const data = (await readDockJson(
    scenarioPath(slug, scenarioId),
    "this scenario's timeline subsets",
  )) as Partial<ScenarioSubsets>;

  if (!Array.isArray(data.subsets) || data.wording == null || typeof data.wording !== "object") {
    throw new Error(
      `This scenario's timeline subsets came back without their list or their ` +
        `wording. The backend and this build disagree about the payload shape.`,
    );
  }
  return { subsets: data.subsets, wording: data.wording };
}

/** Attach one subset to this scenario. Returns the list as it now stands. */
export async function attachSubset(
  slug: string,
  scenarioId: string,
  subsetId: string,
): Promise<AttachedSubset[]> {
  const data = await readDockJson(scenarioPath(slug, scenarioId), "the attached subsets", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ subset_id: subsetId }),
  });
  return Array.isArray(data) ? (data as AttachedSubset[]) : [];
}

/**
 * Detach one subset from this scenario.
 *
 * ⚑ The one HARD delete in the feature: a link is the SCENARIO's fact about the
 * subset, not the subset's content, so removing it writes no subset history and
 * cannot be undone from here. The subset itself is untouched and still on the
 * timeline — which is why this needs no confirm dialog either.
 */
export async function detachSubset(
  slug: string,
  scenarioId: string,
  subsetId: string,
): Promise<AttachedSubset[]> {
  const data = await readDockJson(
    `${scenarioPath(slug, scenarioId)}/${encodeURIComponent(subsetId)}`,
    "the attached subsets",
    { method: "DELETE" },
  );
  return Array.isArray(data) ? (data as AttachedSubset[]) : [];
}
