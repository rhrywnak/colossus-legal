// =============================================================================
// factCards.ts — the write behind an inline field edit (FACT_CARD_v2 §2)
// =============================================================================
//
//   PUT /api/cases/:slug/scenarios/:id/facts/:graph_node_id/card
//
// One field per request. The draft mark is per field precisely so a card with an
// edited Answer can still show a drafted Watch out, and a whole-card PUT would
// re-stamp all five authorships from one edit.
//
// ## Why a failure here is never swallowed
//
// The editor CLOSES on save and the row shows the new sentence. That makes a
// silent failure the worst outcome available: the screen would show a sentence
// the database does not hold, on a deck a witness reads from on the stand. Every
// call below throws with a message the caller renders through the stored
// `fact_card_save_failed_template` (Standing Rule 1).

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import type { CardFieldName } from "../components/factCard";
import type { CardFactStance } from "./scenarioCards";

/** One accusation, as the editor submits it. */
export type SupportInput = {
  allegation_id: string;
  stance: CardFactStance;
};

/**
 * The edit, as one of five typed shapes.
 *
 * Mirrors the backend's internally-tagged enum: `{"field": "answer", "value": …}`
 * with the value's type decided by the field. A union rather than
 * `{field: string, value: unknown}` for the same reason the backend uses one —
 * the shapes genuinely differ, and a caller that sent a string for
 * `backs_position` should not compile.
 */
export type CardFieldUpdate =
  | { field: "title"; value: string | null }
  | { field: "backs_position"; value: number | null }
  | { field: "supports"; value: SupportInput[] }
  | { field: "watch_out"; value: string | null }
  | { field: "answer"; value: string | null };

/**
 * Store one field of one card.
 *
 * @param slug       case slug
 * @param scenarioId the scenario this card belongs to — a card is SCENARIO-scoped,
 *                   because the same statement reads differently in two attacks
 * @param nodeId     the Evidence node the card is about
 * @param update     which field, and what it becomes
 * @throws Error naming the status, for the caller's error line
 */
export async function saveCardField(
  slug: string,
  scenarioId: string,
  nodeId: string,
  update: CardFieldUpdate,
): Promise<void> {
  // authFetch adds credentials + a 30s timeout (AbortController) — Rule 13.
  const response = await authFetch(
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}/scenarios/${encodeURIComponent(
      scenarioId,
    )}/facts/${encodeURIComponent(nodeId)}/card`,
    {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(update),
    },
  );

  if (response.ok) return;

  // Each status means something different to the person holding the keyboard, so
  // each says something different. A 403 is not a fault to report — it is a
  // permission they do not have, and telling them it broke sends them to an
  // operator for nothing.
  const reason =
    response.status === 403
      ? "you do not have permission to edit these cards"
      : response.status === 400
        ? "the server refused that value"
        : response.status === 404
          ? "this scenario is no longer in this case"
          : `the server answered HTTP ${response.status}`;
  throw new Error(`${reason}.`);
}

/**
 * The field name a row edits, as the API spells it.
 *
 * Re-exported so a component importing the editor does not have to reach into
 * `components/factCard` for a type the service also speaks.
 */
export type { CardFieldName };
