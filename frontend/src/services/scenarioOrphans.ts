// =============================================================================
// scenarioOrphans.ts — client for the humane orphan strip (task 1.7C, D9)
// =============================================================================
//
// `GET /api/cases/:slug/scenarios/:scenarioId/facts/orphans`
//
// The saved references whose graph node no longer resolves, grouped by the
// document they came from. Measured on DEV 2026-08-03: 26 of them on scenario S-2,
// including six human INCLUDE decisions — the July 24/25 re-extraction loss,
// surfaced rather than hidden. They are the origin of the durability law.
//
// ## Why the grouping arrives already done
//
// A ref carries only a `graph_node_id`. The document it came from is encoded in
// that id (`doc-…:evidence:hash`), and the document's TITLE lives in Postgres —
// so parsing and labelling happen server-side, where both are reachable. A browser
// splitting id strings would be deriving provenance from an identifier, and it
// could not resolve a title at all (Standing Rule 12 and v2 §7 item 2).
//
// Same idioms as `scenarioFacts.ts`: `authFetch` (credentials + 30s
// AbortController timeout), `encodeURIComponent` on every path param,
// `readErrorMessage` to surface the backend `{message}`, and every non-2xx throws
// (Standing Rule 1 — an orphan strip that silently renders empty would say
// "nothing was lost", which is the one thing it must never say).

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { readErrorMessage } from "./fetchUtils";

/**
 * How trustworthy a group's label is (backend `OrphanTitleState`).
 *
 * The state travels BESIDE the label because §2.8 forbids inventing titles, and a
 * single string cannot tell a real document title from a handle derived out of an
 * id. A client guessing from the shape of the text would guess wrong.
 */
export type OrphanTitleState = "resolved" | "unrecoverable" | "unparseable";

/** One document's worth of orphaned references (backend `OrphanGroup`). */
export type OrphanGroupDto = {
  /** The document id parsed from the refs' node ids. Empty for `unparseable`. */
  document_id: string;
  /** What to show: a real title, or an id-derived slug. Never invented. */
  label: string;
  title_state: OrphanTitleState;
  count: number;
};

/** The whole strip payload (backend `ScenarioOrphansResponse`). */
export type ScenarioOrphansDto = {
  /** Every orphaned reference, counted once — the summary line's number. Equals
   *  the sum of the groups' counts by construction, so a grouping failure cannot
   *  quietly shrink it. */
  total: number;
  /** Biggest loss first, then alphabetical. Empty when nothing is orphaned. */
  groups: OrphanGroupDto[];
};

/** Load this scenario's orphaned references, grouped by source document. */
export async function fetchScenarioOrphans(
  slug: string,
  scenarioId: string,
): Promise<ScenarioOrphansDto> {
  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}` +
      `/scenarios/${encodeURIComponent(scenarioId)}/facts/orphans`,
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to load the unavailable references for scenario "${scenarioId}" ` +
        `(HTTP ${response.status}${detail}). These are saved rulings whose ` +
        `content is gone; the count matters, so this is not skipped silently.`,
    );
  }

  const data: unknown = await response.json();
  const parsed = data as Partial<ScenarioOrphansDto>;

  // Validate the load-bearing shape so a contract mismatch throws HERE with
  // context rather than as an `undefined.map` inside the strip.
  if (typeof parsed.total !== "number" || !Array.isArray(parsed.groups)) {
    throw new Error(
      `The unavailable-references response for scenario "${scenarioId}" is ` +
        `missing total/groups — backend/frontend contract mismatch. If this ` +
        `persists, report it to the site administrator.`,
    );
  }
  return parsed as ScenarioOrphansDto;
}
