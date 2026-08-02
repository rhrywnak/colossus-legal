// =============================================================================
// rehearsal.ts — client for rehearsal mode and the ready gate (task 1.5)
// -----------------------------------------------------------------------------
// Endpoints:
//   POST /api/cases/:slug/scenarios/:id/ready → declare ready / take back out
//   GET  /api/cases/:slug/rehearsal           → every READY scenario + the card
//
// Same idioms as `scenarioAugmentation.ts`: `authFetch` (credentials + a 30s
// AbortController timeout), `encodeURIComponent` on every path param,
// `readErrorMessage` to surface the backend `{message}`, every non-2xx throws.
//
// ## The gate is not applied here
//
// `fetchRehearsal` sends no status parameter and the payload contains only ready
// scenarios, because the SERVER decides what is rehearsable. A client-side filter
// would be a filter someone could forget — and forgetting it would put a drafted
// scenario in front of a witness.
//
// ## What this payload deliberately does NOT contain
//
// No motivation, no confidence, no verdicts, no internal status vocabulary, no
// document/page citations (v2 §10). That is enforced by the backend DTO's shape
// and pinned by a test there. The types below mirror it verbatim, so anything
// missing from these shapes is missing on purpose.
// =============================================================================

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { readErrorMessage } from "./fetchUtils";

/** One talking point, optionally paired with the exhibit that backs it. */
export type RehearsalPoint = {
  text: string;
  /** A plain label such as "Exhibit 14". `null` until the pairing is authored. */
  exhibit: string | null;
};

/** One ready scenario, as the four §10 blocks. */
export type RehearsalScenario = {
  code: string;
  /** Our one sentence. `null` when nobody has framed it yet. */
  theme: string | null;
  /** Their claim, in plain words. `null` when the definition carries none. */
  attack: string | null;
  points: RehearsalPoint[];
  /** What the other side will wave around — human-flagged notes. */
  watch_list: string[];
};

export type RehearsalPayload = {
  scenarios: RehearsalScenario[];
  /** The four lines shown on every screen, composed by the backend. */
  standing_card: string[];
};

/** What a readiness change reports back. */
export type ReadyChange = {
  status: string;
  in_rehearsal: boolean;
  /** The plain confirmation, composed server-side. Rendered verbatim. */
  message: string;
};

/** Load every ready scenario for a case, plus the standing card. */
export async function fetchRehearsal(slug: string): Promise<RehearsalPayload> {
  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/rehearsal`,
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to load rehearsal mode for "${slug}" ` +
        `(HTTP ${response.status}${detail}). Try again.`,
    );
  }

  const data: unknown = await response.json();
  const parsed = data as Partial<RehearsalPayload>;

  // Validate the load-bearing shapes so a contract mismatch throws HERE with
  // context, rather than as an `undefined.map` mid-rehearsal.
  if (!Array.isArray(parsed.scenarios) || !Array.isArray(parsed.standing_card)) {
    throw new Error(
      `The rehearsal response for "${slug}" is missing scenarios or the ` +
        `standing card — backend/frontend contract mismatch. If this ` +
        `persists, report it to the site administrator.`,
    );
  }
  return parsed as RehearsalPayload;
}

/**
 * Declare a scenario ready, or take it back out of rehearsal.
 *
 * `ready` states the TARGET rather than toggling: two people with the page open
 * could each press a toggle and land on the opposite of what both intended.
 */
export async function setScenarioReady(
  slug: string,
  scenarioId: string,
  ready: boolean,
): Promise<ReadyChange> {
  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}` +
      `/scenarios/${encodeURIComponent(scenarioId)}/ready`,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ ready }),
    },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to ${ready ? "declare" : "withdraw"} scenario "${scenarioId}" ` +
        `${ready ? "ready" : "from rehearsal"} (HTTP ${response.status}${detail}).`,
    );
  }

  const data: unknown = await response.json();
  const parsed = data as Partial<ReadyChange>;
  if (typeof parsed.message !== "string" || typeof parsed.in_rehearsal !== "boolean") {
    throw new Error(
      `The readiness response for scenario "${scenarioId}" is missing its ` +
        `confirmation — the change may or may not have been recorded. Reload ` +
        `the page to see the scenario's actual state.`,
    );
  }
  return parsed as ReadyChange;
}
