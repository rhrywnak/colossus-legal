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
import type { LinkCut } from "./evidenceLinks";

// ─── DTO mirrors ────────────────────────────────────────────────────────────

/** A candidate's workbench state for one scenario (backend `FactStatus`). */
export type FactStatus = "undecided" | "included" | "dropped";

/** How confident the scan reported itself to be (backend `ConfidenceBand`). */
export type ConfidenceBand = "high" | "medium" | "low" | "unscored";

/** The quote with enough surrounding text to be read in context (§7.1). */
export type CardQuote = {
  /** The verbatim words. NEVER groomed — the context around it is expanded to
   *  sentence boundaries and has its gutter numerals stripped (task 1.7C, defect
   *  D6), but this is the anchor and §12 makes anchors sacred. */
  text: string;
  /** Source text immediately before the quote. Empty when unavailable.
   *
   *  Sentence-complete at its left edge unless `context_before_complete` is false.
   *  Transcript gutter numerals are stripped for display; the stored page is
   *  untouched. */
  context_before: string;
  /** Source text immediately after the quote. Empty when unavailable. */
  context_after: string;
  /** Whether `context_before` starts at a real sentence start (task 1.7C, D6).
   *
   *  `false` means the PAGE begins mid-sentence — measured on DEV, 154 of the 499
   *  locatable cards (30.9%). The card must not imply a completeness the data does
   *  not have (Standing Rule 1), which is what the paired notice is for. */
  context_before_complete: boolean;
  /** Whether `context_after` ends at a real sentence end. `false` in 65 of 499
   *  cards, measured — the page ends mid-sentence. */
  context_after_complete: boolean;
  /** The page-edge notice for `context_before` ("— page begins here"), composed
   *  SERVER-side, present exactly when that flank is incomplete and has text to
   *  explain. `null` otherwise.
   *
   *  The state flag and the composed sentence travel together for the same reason
   *  `CardGrounding` carries both `state` and `label`: the boolean is what a client
   *  branches on, the string is the words, and the browser does not get to decide
   *  how a page edge reads. */
  context_before_notice: string | null;
  /** The page-edge notice for `context_after` ("— page ends here"). */
  context_after_notice: string | null;
  /** The interrogatory this answers, when the item is a discovery answer.
   *
   *  Task 1.7F: this is the DISPLAY text. When a human has corrected the
   *  machine's question it carries THEIR words, and `question_authorship` says
   *  so — the client renders one sentence rather than choosing between two. */
  question: string | null;
  /** Who wrote the question above, and when — present exactly when `question`
   *  is (task 1.7F Part B). Absent on a card with no question at all. */
  question_authorship?: CardQuestionAuthorship;
};

/** Who wrote a card's question (task 1.7F Part B). */
export type CardQuestionAuthorship = {
  /** `system` — straight from the graph; `human` — someone corrected it. The
   *  token the client branches on: the pencil's Revert control appears only for
   *  `human`. */
  source: "system" | "human";
  /** The composed badge — "System", or "roman · 4 Aug 2026". Rendered verbatim:
   *  the backend owns how authorship reads, the same way it owns every other
   *  sentence on the card. The ICON is the list's own control vocabulary, like
   *  the state chip's, and lives in the component. */
  label: string;
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

/**
 * One accusation a HUMAN linked this statement to (task 2.10).
 *
 * Separate from `bears_on`, which is the MACHINE's list. They render alike — both
 * are chips — but they are different claims about the world, and a client that
 * merged them could not answer "what did the extraction actually find?".
 */
export type CardHumanLink = {
  allegation_id: string;
  /** "¶41 — refused to divide the property amicably", pre-composed. */
  label: string;
  /** The token to branch on. */
  cut: LinkCut;
  /** The same choice in words — "They'll use it against us". Rendered verbatim. */
  cut_label: string;
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
  /** The accusations a HUMAN linked this statement to (task 2.10). Empty is the
   *  ordinary state; a non-empty list is what clears `defer_required` on a card
   *  the extraction never linked. */
  human_links: CardHumanLink[];
  /** What the human did, in their own terms — "You linked this to ¶41 · they'll
   *  use it against us." Present exactly when `human_links` is non-empty.
   *
   *  NOT a stance, and it must never become one: the extraction said nothing
   *  about this statement, so `stance` stays null and this fills the §7.5 slot
   *  instead (ruling R2). */
  human_link_summary: string | null;
  /** How much this fact carries THIS scenario — `carries` | `backup` |
   *  `background` (task 2.13). The stored TOKEN, never the display name: the
   *  three names live in the settings store, and the row pairs this token with
   *  the wording it was served. Absent for a candidate with no reference row —
   *  it is not in the scenario, so it has no weight here, which is a different
   *  thing from being in it and unweighed (`backup`). */
  tier?: FactTier;
  /** The human's stored position, absent when they have never placed this fact.
   *  Carried so a drag can name its neighbours — never so the browser can
   *  re-derive the order the server already computed. */
  sort_ordinal?: number;
  /** Where this fact sits in the list — the ONE number to sort on (task 2.13c).
   *  Computed server-side over the whole included set, because the rule for
   *  placing an unplaced fact relative to a placed one is arithmetic the browser
   *  must not own a second copy of. Absent for anything that is not an included
   *  fact. */
  display_ordinal?: number;
};

/** The three weights a fact can carry (task 2.13). Mirrors the backend enum. */
export type FactTier = "carries" | "backup" | "background";

export type ScenarioCardsResponse = {
  pool: ScenarioCard[];
  set_aside: ScenarioCard[];
  /** "38 of 94 linked." — how much of the stuck pile a human has worked through,
   *  composed server-side and counted from the POOL, never from this session's
   *  clicks (the 1.7E-a ruling). `null` when nothing in this pool is stuck. */
  link_progress: string | null;
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
