// =============================================================================
// evidenceSummary.ts — correcting a machine-written question (task 1.7F Part B)
// =============================================================================
//
// Two writes: save a human's replacement for `Evidence.question`, and revert to
// the machine's own wording.
//
// ## Domain note: the route has no scenario in it, deliberately (ruling R1)
//
// `/cases/:slug/evidence/:id/summary`. One summary per piece of evidence,
// case-wide: correct it while working S-2 and it reads corrected in S-4, because
// the same quote is the same quote. A scenario segment here would be the first
// step back toward one C-code meaning two things depending on which page a reader
// opened it from.
//
// ## Revert is a DELETE, and there is nothing to send
//
// The machine's words live in Neo4j and are never touched by this feature. Revert
// removes the override row; the next card read composes the machine's sentence
// again, unchanged. That is why this file has no "restore" payload — there is no
// copy of the original on this side to send back.

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";

/** What the server reports after a save or a revert. */
export type SummaryOverrideResult = {
  graph_node_id: string;
  /** The text now in force. Absent after a revert — the machine's sentence is in
   *  the graph, and the caller re-reads the card rather than being handed a copy
   *  from a table that does not hold it. */
  summary_text?: string;
  /** "roman · 4 Aug 2026", composed server-side. Absent once reverted. */
  authorship_label?: string;
  /** Whether a stored correction is now in force. Branch on this rather than on
   *  the presence of a string. */
  overridden: boolean;
};

function summaryUrl(slug: string, graphNodeId: string): string {
  return (
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}` +
    `/evidence/${encodeURIComponent(graphNodeId)}/summary`
  );
}

/**
 * Read a refusal's own words out of an error response.
 *
 * The backend refuses a blank correction with a sentence that names the
 * alternative ("use revert instead"). Dropping it for a bare status code would
 * leave the human guessing what to do next (Standing Rule 1).
 */
async function readErrorMessage(response: Response): Promise<string> {
  try {
    const body: unknown = await response.json();
    const message = (body as { message?: unknown }).message;
    return typeof message === "string" && message.trim() ? ` — ${message}` : "";
  } catch {
    // best-effort: a non-JSON error body is still a failure worth reporting, and
    // the status code below carries it. Nothing is swallowed — the caller throws
    // either way.
    return "";
  }
}

/**
 * Save a human's correction of a machine-written question.
 *
 * @param slug        the case, for authorisation and context — not part of the key
 * @param graphNodeId the evidence node this corrects; the whole key (ruling R1)
 * @param summaryText the replacement question. Blank is refused by the backend
 *                    with a message pointing at revert.
 */
export async function saveQuestionOverride(
  slug: string,
  graphNodeId: string,
  summaryText: string,
): Promise<SummaryOverrideResult> {
  const response = await authFetch(summaryUrl(slug, graphNodeId), {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ summary_text: summaryText }),
  });

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to save the corrected question for "${graphNodeId}" ` +
        `(HTTP ${response.status}${detail}). Your text was not stored.`,
    );
  }
  return (await response.json()) as SummaryOverrideResult;
}

/**
 * Restore the machine's own question by deleting the human's correction.
 *
 * Succeeds whether or not a correction existed: the caller asked for the
 * machine's text and the machine's text is what they get. The backend logs the
 * difference, because a revert that removed nothing means the badge was stale.
 */
export async function revertQuestionOverride(
  slug: string,
  graphNodeId: string,
): Promise<SummaryOverrideResult> {
  const response = await authFetch(summaryUrl(slug, graphNodeId), { method: "DELETE" });

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to restore the system's question for "${graphNodeId}" ` +
        `(HTTP ${response.status}${detail}). The correction is still in place.`,
    );
  }
  return (await response.json()) as SummaryOverrideResult;
}
