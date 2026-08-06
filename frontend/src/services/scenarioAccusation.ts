// =============================================================================
// scenarioAccusation.ts — client for the accusation section (task 2.11 B1)
// -----------------------------------------------------------------------------
// Endpoints:
//   GET    /api/cases/:slug/scenarios/:id/accusation
//   PUT    /api/cases/:slug/scenarios/:id/accusation
//   POST   /api/cases/:slug/scenarios/:id/accusation/instances/:nodeId
//   DELETE /api/cases/:slug/scenarios/:id/accusation/instances/:nodeId
//   PUT    /api/cases/:slug/scenarios/:id/accusation/instances/:nodeId/answer
//   DELETE /api/cases/:slug/scenarios/:id/accusation/instances/:nodeId/answer
//
// Same idioms as `scenarioAugmentation.ts`: `authFetch` (credentials + a 30s
// AbortController timeout), `encodeURIComponent` on every path segment,
// `readErrorMessage` to surface the backend's `{message}`, and every non-2xx
// throws (Standing Rule 1).
//
// ## The frontend composes nothing here
//
// `count_line` arrives as "Said 3 times, in 2 documents." and every gap arrives
// as a finished sentence with its `{code}` already in it. This file does not hold
// the templates they were built from, and could not rebuild them: that absence is
// the §10 exclusion made structural, not an omission. See
// backend/src/dto/scenario_accusation.rs.
//
// The one template that DOES arrive is `save_failed_template`, whose `{detail}`
// is the failure's own text — the single value that exists only on this side.
//
// The shapes mirror backend/src/dto/scenario_accusation.rs verbatim.
// =============================================================================

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { readErrorMessage } from "./fetchUtils";

/** The stable gap tokens, mirroring `services::scenario_accusation_panel`. */
export const GAP_NO_ANSWER = "no_answer_prepared";
export const GAP_ACCUSATION_REMOVED = "accusation_removed";
export const GAP_ANSWER_REMOVED = "answer_removed";

export type AccusationInstanceDto = {
  graph_node_id: string;
  /** "C-14", or `null` for a candidate nothing has numbered yet. */
  code: string | null;
  answers_graph_node_id: string | null;
  answer_code: string | null;
  /** Whether the paired answer is still in the scenario (the Remove law). */
  answer_present: boolean;
};

export type AccusationGapDto = {
  /** One of the three `GAP_*` tokens. Branch on this, never on the message. */
  kind: string;
  graph_node_id: string;
  /** The stored sentence, already naming the fact it is about. */
  message: string;
};

export type AccusationWordingDto = {
  section_heading: string;
  section_hint: string;
  text_label: string;
  text_placeholder: string;
  text_missing_notice: string;
  text_edit_label: string;
  text_save_label: string;
  text_clear_label: string;
  text_cancel_label: string;
  mark_label: string;
  unmark_label: string;
  pair_label: string;
  repair_label: string;
  unpair_label: string;
  answer_label: string;
  picker_prompt: string;
  picker_cancel_label: string;
  /** Shown when the picker's filter leaves nothing. */
  picker_no_match_notice: string;
  /**
   * Shown when the picker had nothing to offer in the first place.
   *
   * Two sentences rather than one because the remedies differ — type something
   * else, versus there is nothing left to choose. Collapsing them would leave a
   * human retyping against an empty list (Standing Rule 1).
   */
  picker_empty_notice: string;
  gaps_heading: string;
  no_gaps_notice: string;
  /** Carries `{detail}` — filled by {@link fillDetail}. */
  save_failed_template: string;
};

export type AccusationPanelDto = {
  /** `null` until a human writes it; never stood in for. */
  accusation_text: string | null;
  /** "Said 3 times, in 2 documents." — `null` when nothing is marked. */
  count_line: string | null;
  /** The nothing-marked notice — `null` once anything is marked. */
  no_instances_notice: string | null;
  instances: AccusationInstanceDto[];
  gaps: AccusationGapDto[];
  wording: AccusationWordingDto;
};

function accusationUrl(slug: string, scenarioId: string): string {
  return (
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}` +
    `/scenarios/${encodeURIComponent(scenarioId)}/accusation`
  );
}

function instanceUrl(slug: string, scenarioId: string, graphNodeId: string): string {
  return `${accusationUrl(slug, scenarioId)}/instances/${encodeURIComponent(graphNodeId)}`;
}

/**
 * Put the failure's own words into the stored sentence.
 *
 * The one substitution this file performs, and it is not composition: the
 * template is the human's, and `{detail}` is the slot it leaves for something
 * only the browser knows. Mirrors `evidenceLinks.fillDetail`.
 */
export function fillDetail(template: string, detail: string): string {
  return template.replace("{detail}", detail);
}

/** Load the whole accusation section. */
export async function fetchAccusationPanel(
  slug: string,
  scenarioId: string,
): Promise<AccusationPanelDto> {
  const response = await authFetch(accusationUrl(slug, scenarioId));

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to load the accusation for scenario "${scenarioId}" ` +
        `(HTTP ${response.status}${detail}). Try again.`,
    );
  }

  const data: unknown = await response.json();
  const parsed = data as Partial<AccusationPanelDto>;

  // The shapes this section cannot render without. A missing `wording` would
  // leave every control unlabelled and a missing list would render as "nothing
  // marked" — indistinguishable from a scenario nobody has started, which is
  // exactly the confusion the honest-gap law exists to prevent.
  if (
    !Array.isArray(parsed.instances) ||
    !Array.isArray(parsed.gaps) ||
    parsed.wording == null
  ) {
    throw new Error(
      `The accusation payload for scenario "${scenarioId}" is missing ` +
        `instances/gaps/wording — backend/frontend contract mismatch. If this ` +
        `persists, report it to the site administrator.`,
    );
  }
  return parsed as AccusationPanelDto;
}

/**
 * Store the plain-words accusation, or withdraw it.
 *
 * `null` withdraws — a real act, which returns the rehearsal block to its named
 * gap. An empty string is NOT the way to withdraw: the backend refuses it,
 * because "withdraw this" and a slip of the keyboard are different intentions.
 */
export async function setAccusationText(
  slug: string,
  scenarioId: string,
  text: string | null,
): Promise<void> {
  const response = await authFetch(accusationUrl(slug, scenarioId), {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ text }),
  });

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to save the accusation for scenario "${scenarioId}" ` +
        `(HTTP ${response.status}${detail}).`,
    );
  }
}

/** Mark one included fact as an instance of the accusation. */
export async function markInstance(
  slug: string,
  scenarioId: string,
  graphNodeId: string,
): Promise<void> {
  const response = await authFetch(instanceUrl(slug, scenarioId, graphNodeId), {
    method: "POST",
  });

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to mark that statement as an accusation instance ` +
        `(HTTP ${response.status}${detail}).`,
    );
  }
}

/** Withdraw a marking. The fact stays in the scenario. */
export async function unmarkInstance(
  slug: string,
  scenarioId: string,
  graphNodeId: string,
): Promise<void> {
  const response = await authFetch(instanceUrl(slug, scenarioId, graphNodeId), {
    method: "DELETE",
  });

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to withdraw that accusation instance ` +
        `(HTTP ${response.status}${detail}).`,
    );
  }
}

/**
 * Pair — or re-pair — the record item that answers one instance.
 *
 * Re-pairing REPLACES: one answer per accusation, enforced by a partial unique
 * index. Sending it twice leaves the same one stored, which is why this is a PUT.
 */
export async function pairAnswer(
  slug: string,
  scenarioId: string,
  graphNodeId: string,
  answersGraphNodeId: string,
): Promise<void> {
  const response = await authFetch(`${instanceUrl(slug, scenarioId, graphNodeId)}/answer`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ answers_graph_node_id: answersGraphNodeId }),
  });

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to pair that answer to the accusation instance ` +
        `(HTTP ${response.status}${detail}).`,
    );
  }
}

/** Remove the answer paired to one instance, returning it to the prep list. */
export async function unpairAnswer(
  slug: string,
  scenarioId: string,
  graphNodeId: string,
): Promise<void> {
  const response = await authFetch(`${instanceUrl(slug, scenarioId, graphNodeId)}/answer`, {
    method: "DELETE",
  });

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to un-pair that answer (HTTP ${response.status}${detail}).`,
    );
  }
}
