// =============================================================================
// scenarioFactCuration.ts — weighing and placing a fact (task 2.13 slice 1)
// -----------------------------------------------------------------------------
// Endpoints (backend, pipeline DB):
//   POST /api/cases/:slug/scenarios/:scenarioId/facts/:graphNodeId/tier
//        → record how much this fact carries the scenario.
//   POST /api/cases/:slug/scenarios/:scenarioId/facts/:graphNodeId/order
//        → record where a dragged fact landed, named by its two neighbours.
//
// Same idioms as `scenarioGather.ts`: `authFetch` (credentials + the 30s
// AbortController timeout — Rule 13), `encodeURIComponent` on every path param,
// `readErrorMessage` to surface the backend's own words, and every non-2xx
// throws (Standing Rule 1 — a write that did not land must never look like one
// that did).
//
// ## Why neither call returns the new list
//
// Both are followed by a cards re-read, which is the one source of truth for
// tier and order (no page-side copy that can go stale). Returning a second
// version of the same state from the write would create exactly the two-places
// disagreement the re-read exists to prevent.
// =============================================================================

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { readErrorMessage } from "./fetchUtils";
import type { FactTier } from "./scenarioCards";

/**
 * `…/api/cases/:slug/scenarios/:scenarioId/facts/:graphNodeId` — the shared stem.
 *
 * ## The `/api` prefix is NOT optional, and its absence is invisible in unit tests
 *
 * `API_BASE_URL` is the origin only; every service in this directory appends
 * `/api` itself. Shipping this without it produced a `405 Method Not Allowed` on
 * DEV — the un-prefixed path fell through to the SPA's own routes, which answer
 * GET and not POST, so the failure arrived as a method error rather than a 404
 * and pointed away from the real cause.
 *
 * Nothing in the type system or the backend can catch that: the URL is a string
 * built here. What catches it is the pair of tests in
 * `__tests__/scenarioFactCuration.test.ts` asserting the exact path both writes
 * request — the same guard `scenarioCards.test.ts` already had, and the one this
 * module shipped without.
 */
function factUrl(slug: string, scenarioId: string, graphNodeId: string): string {
  return (
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}` +
    `/scenarios/${encodeURIComponent(scenarioId)}` +
    `/facts/${encodeURIComponent(graphNodeId)}`
  );
}

/**
 * Record how much one fact carries one scenario.
 *
 * @throws Error on any non-2xx, carrying the backend's message. A 404 means the
 *   fact is not in this scenario — a stale page, or somebody removed it — and
 *   the caller surfaces that rather than leaving a star lit for a write that
 *   never happened.
 */
export async function setFactTier(
  slug: string,
  scenarioId: string,
  graphNodeId: string,
  tier: FactTier,
): Promise<void> {
  const response = await authFetch(
    `${factUrl(slug, scenarioId, graphNodeId)}/tier`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ tier }),
    },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(`HTTP ${response.status}${detail}`);
  }
}

/**
 * Record where a dragged fact landed.
 *
 * `after` is the fact it now sits BELOW, `before` the one it sits ABOVE; either
 * may be `null` for a drop at the very top or bottom.
 *
 * ## Why the browser sends NEIGHBOURS and not a position
 *
 * An index describes the list this browser last drew. The server computes the
 * ordinal from what is STORED, so naming the two rows lets it refuse by name
 * when one of them has gone — the honest answer to a stale page, where an index
 * would silently place the fact somewhere nobody asked for.
 *
 * @throws Error on any non-2xx. A 409 is the interesting one: either a neighbour
 *   has left the scenario, or there is no room left between the two — both of
 *   which the human can act on, so the backend's own words are carried through.
 */
export async function setFactOrder(
  slug: string,
  scenarioId: string,
  graphNodeId: string,
  after: string | null,
  before: string | null,
): Promise<void> {
  const response = await authFetch(
    `${factUrl(slug, scenarioId, graphNodeId)}/order`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      // Only the neighbours that exist are sent: the backend's body denies
      // unknown fields and treats an omitted key as "no neighbour that side",
      // so an explicit null and an absent key mean the same thing there. Sending
      // the keys we have keeps the request readable in a network log.
      body: JSON.stringify({
        ...(after === null ? {} : { after }),
        ...(before === null ? {} : { before }),
      }),
    },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(`HTTP ${response.status}${detail}`);
  }
}
