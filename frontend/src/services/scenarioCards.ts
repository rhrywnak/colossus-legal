// =============================================================================
// scenarioCards.ts — client for the §7 candidate card payload (task 1.2/1.3)
// -----------------------------------------------------------------------------
// Endpoint:
//   GET /api/cases/:slug/scenarios/:scenarioId/facts/cards
//       → { pool, set_aside } — one complete card per candidate, everything
//         display-ready and in plain trial language.
//
// Same idioms as `scenarioGather.ts`: `authFetch` (credentials + 30s
// AbortController timeout), `encodeURIComponent` on every path param,
// `readErrorMessage` to surface the backend `{message}`, boundary validation of
// the load-bearing fields, and every non-2xx throws (Standing Rule 1).
//
// ## The frontend computes NOTHING from these types
//
// Every string below was composed by the backend. There is no percentage to
// format, no enum to translate, no URL to assemble, no vocabulary to map. If a
// value would need a decision to display, that decision was already made
// server-side (v2 §7 item 2, and the language law). A component that finds
// itself building a sentence out of these fields is doing something wrong.
//
// The shapes mirror backend/src/dto/scenario_card.rs verbatim.
// =============================================================================

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { readErrorMessage } from "./fetchUtils";

// ─── DTO mirrors ────────────────────────────────────────────────────────────

/** A candidate's workbench state for one scenario (backend `FactStatus`). */
export type FactStatus = "undecided" | "included" | "dropped";

/** How confident the scan reported itself to be (backend `ConfidenceBand`). */
export type ConfidenceBand = "high" | "medium" | "low" | "unscored";

/** The quote with enough surrounding text to be read in context (§7.1). */
export type CardQuote = {
  text: string;
  /** Source text immediately before the quote. Empty when unavailable. */
  context_before: string;
  /** Source text immediately after the quote. Empty when unavailable. */
  context_after: string;
  /** The interrogatory this answers, when the item is a discovery answer. */
  question: string | null;
};

/** Where the quote is, and how to get there (§7.2). */
export type CardPinpoint = {
  document_id: string;
  document_title: string;
  /** The full pinpoint line, pre-composed — "CFS responses at 26". Rendered
   *  verbatim; the client never joins the title and page itself. */
  label: string;
  page: number | null;
  /** Assembled server-side — the browser builds no URLs. */
  viewer_href: string;
};

/** Who said it, and on what authority we say so (§7.3). */
export type CardSpeaker = {
  /** `null` for documentary evidence, which genuinely has no speaker. */
  name: string | null;
  /** `"extracted"` today — the card says whose attribution this is. */
  attribution: string;
};

/** The item's position toward one accusation — verb AND object (§7.5). */
export type CardStance = {
  verb: string;
  object: string;
  /** Pre-composed, so this component concatenates nothing. */
  summary: string;
};

/** One accusation this item bears on (§7.6). */
export type CardBearsOn = {
  accusation: string;
  /** Every element the accusation goes to; empty is a real state. */
  elements: string[];
  count: string | null;
};

/** Whether the quote was located in its source (§7.7). */
export type CardGrounding = { state: string; label: string };

/** How confident the scan was, banded (§7.8). */
export type CardConfidence = { band: ConfidenceBand; label: string };

/**
 * One complete candidate card.
 *
 * TS-learning: the optional fields are `T | null`, not `T | undefined`. The
 * backend serializes them present-as-null rather than omitting them, so the
 * component can branch on `=== null` without also handling a missing key. The
 * one exception is a field the backend genuinely skips — none here.
 */
export type ScenarioCard = {
  /** `"C-14"`, or `null` for a candidate not yet numbered. */
  code: string | null;
  /** Not shown; used to address the ruling routes. */
  graph_node_id: string;
  quote: CardQuote;
  pinpoint: CardPinpoint;
  speaker: CardSpeaker;
  statement_kind: string | null;
  /** `null` — never a bare verb. See `defer_required` for why. */
  stance: CardStance | null;
  bears_on: CardBearsOn[];
  grounding: CardGrounding | null;
  confidence: CardConfidence;
  /** The workbench state as a token to branch on — `undecided` | `included` |
   *  `dropped`. Paired with `status_label` the same way `CardGrounding` pairs
   *  `state` with `label`: filtering on the LABEL would mean reading state out
   *  of prose, which breaks the day the wording changes. */
  status: FactStatus;
  status_label: string;
  /** True when this card cannot be ruled on as it stands. */
  defer_required: boolean;
  /** Why the SYSTEM says it cannot be ruled on. Present iff `defer_required`. */
  defer_required_reason: string | null;
  /** Why a HUMAN parked it. Distinct from the field above — different authors. */
  defer_reason: string | null;
};

export type ScenarioCardsResponse = {
  pool: ScenarioCard[];
  set_aside: ScenarioCard[];
};

// ─── Client ─────────────────────────────────────────────────────────────────

function cardsUrl(slug: string, scenarioId: string): string {
  return (
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}` +
    `/scenarios/${encodeURIComponent(scenarioId)}/facts/cards`
  );
}

/**
 * Load every candidate card for one scenario.
 *
 * Throws on any non-2xx and on a contract mismatch, with context — a silent
 * empty queue would look identical to a scenario with no candidates, and those
 * are very different states (Standing Rule 1).
 */
export async function fetchScenarioCards(
  slug: string,
  scenarioId: string,
): Promise<ScenarioCardsResponse> {
  const response = await authFetch(cardsUrl(slug, scenarioId));

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to load candidate cards for scenario "${scenarioId}" ` +
        `(HTTP ${response.status}${detail}). Try again.`,
    );
  }

  const data: unknown = await response.json();
  const parsed = data as Partial<ScenarioCardsResponse>;

  // Validate the two load-bearing arrays. A shape mismatch throws HERE with
  // context rather than surfacing as an `undefined.map` deep in the queue.
  if (!Array.isArray(parsed.pool) || !Array.isArray(parsed.set_aside)) {
    throw new Error(
      `Candidate-card response for scenario "${scenarioId}" is missing ` +
        `pool/set_aside — backend/frontend contract mismatch. ` +
        `If this persists, report it to the site administrator.`,
    );
  }
  return parsed as ScenarioCardsResponse;
}
