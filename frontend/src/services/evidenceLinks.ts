// =============================================================================
// evidenceLinks.ts — linking a statement to an accusation (task 2.10)
// =============================================================================
//
// Three calls:
//   GET    /api/cases/:slug/scenarios/:id/allegation-options
//          → the panel's words and the accusations it offers, short list first
//   POST   /api/cases/:slug/evidence/:nodeId/links
//          → link one or more accusations, with one cut
//   DELETE /api/cases/:slug/evidence/:nodeId/links/:allegationId
//          → take one back
//
// ## Domain note: the two WRITES have no scenario in the path (case-wide)
//
// A statement that bears on ¶41 bears on ¶41 in every scenario, exactly as the
// machine's own graph edges do — the same ruling that made the question override
// case-wide in 1.7F. The READ is scenario-scoped because the ORDER is: the short
// list is "the accusations this scenario already serves".
//
// ## The frontend composes NOTHING here either
//
// Every string in these types was built by the backend, including "Show all 120"
// with its count already in it. A component that finds itself joining two of
// these fields into a sentence is doing something wrong (v2 §7 item 2).

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { readErrorMessage } from "./fetchUtils";

// ─── DTO mirrors (backend/src/dto/evidence_links.rs) ────────────────────────

/** Which way a linked statement cuts. The token the client branches on. */
export type LinkCut = "supports" | "against";

/** One accusation a human may tick. */
export type AllegationOption = {
  allegation_id: string;
  /** "¶41 — Defendants attempted to justify the auction…", pre-composed. */
  label: string;
  /** "Count 2 — Breach of fiduciary duty", or null when it sits under none. */
  count_label: string | null;
  /** The lowercased haystack the filter box matches. The backend decides what is
   *  searchable; this file lowercases nothing and searches nothing else. */
  filter_text: string;
};

/**
 * Every word the link panel shows, read from the settings store (task 2.10, R4).
 *
 * There is no default for any of these anywhere in the frontend. A literal
 * fallback here would be exactly the compiled-in string Roman's ruling deletes,
 * and it would be the one on screen on the day the store failed to load — which
 * is the day you most want to know.
 */
export type LinkPanelWording = {
  intro: string;
  scope_notice: string;
  allegations_heading: string;
  /** "Show all 120" — the count already filled in. */
  show_all_label: string;
  filter_placeholder: string;
  no_match_notice: string;
  empty_options_notice: string;
  cut_heading: string;
  cut_supports_label: string;
  cut_against_label: string;
  save_label: string;
  cancel_label: string;
  unlink_label: string;
  missing_cut_refusal: string;
  missing_allegation_refusal: string;
  /** The 1.7F fold-in: the control that restores a machine-written question. */
  question_revert_label: string;
  /** Said when Unlink removed nothing because the pair was not linked. */
  unlink_found_nothing: string;
  /** Said when a link or unlink could not be written. Carries `{detail}`. */
  save_failed_template: string;
  /** Why Include and Exclude are greyed while the panel holds unsaved choices
   *  (task 2.12, item B). */
  save_blocks_ruling: string;
  /** The control that takes a fact back out of a scenario (item G). */
  fact_remove_label: string;
  /** The question asked before a removal. Carries `{code}`. */
  fact_remove_confirm_template: string;
  /** The button that confirms a removal. */
  fact_remove_confirm_yes: string;
  /** The button that abandons it. */
  fact_remove_confirm_cancel: string;
  /** Said when a removal could not be written. Carries `{detail}`. */
  fact_remove_failed_template: string;
};

/**
 * Drop one value into the slot a stored sentence left for it.
 *
 * ## Why this substitution happens in the browser at all
 *
 * The language law puts COMPOSITION on the server, and it stays there: the words,
 * their order and their punctuation are all the stored template's. What is dropped
 * in is the failure's own text — which only exists here, because the thing that
 * failed was this browser's own request. There is no server round trip to ask for
 * a sentence about a round trip that did not happen.
 *
 * Deliberately one named slot and no expression language: a stored string must not
 * be able to reach for anything this function was not handed.
 */
export function fillDetail(template: string, detail: string): string {
  return template.replace("{detail}", detail);
}

/**
 * Drop a candidate's code into the removal confirmation (task 2.12, item G).
 *
 * The same substitution `fillDetail` performs, for the same reason and with the
 * same limits: the sentence is the store's, only the code is this browser's. A
 * confirmation that does not name what it is about is not a confirmation, which
 * is why `{code}` is a REQUIRED placeholder on that key.
 */
export function fillCode(template: string, code: string): string {
  return template.replace("{code}", code);
}

export type AllegationOptions = {
  wording: LinkPanelWording;
  /** The accusations this scenario already serves — anchor first. */
  serving: AllegationOption[];
  /** Everything else in the complaint, in paragraph order. */
  others: AllegationOption[];
  total: number;
};

export type SaveLinksResult = {
  graph_node_id: string;
  linked: number;
  recut: number;
};

export type UnlinkResult = {
  graph_node_id: string;
  allegation_id: string;
  /** False means the pair was not linked — the view was stale. */
  removed: boolean;
};

// ─── Client ─────────────────────────────────────────────────────────────────

function optionsUrl(slug: string, scenarioId: string): string {
  return (
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}` +
    `/scenarios/${encodeURIComponent(scenarioId)}/allegation-options`
  );
}

function linksUrl(slug: string, graphNodeId: string): string {
  return (
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}` +
    `/evidence/${encodeURIComponent(graphNodeId)}/links`
  );
}

/**
 * Load the accusations this scenario's panel offers, and its words.
 *
 * Read ONCE per scenario, not per card and not per ruling: the catalogue is 120
 * rows on DEV and changes only when a document is processed, while the cards
 * payload is re-read after every keystroke of a triage session.
 *
 * Throws on any non-2xx and on a contract mismatch, with context — a silently
 * empty list would look identical to a complaint with no accusations, and those
 * are very different states (Standing Rule 1).
 */
export async function fetchAllegationOptions(
  slug: string,
  scenarioId: string,
): Promise<AllegationOptions> {
  const response = await authFetch(optionsUrl(slug, scenarioId));

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to load the accusations for scenario "${scenarioId}" ` +
        `(HTTP ${response.status}${detail}). Linking is unavailable until this loads.`,
    );
  }

  const data: unknown = await response.json();
  const parsed = data as Partial<AllegationOptions>;

  // The wording is load-bearing: without it the panel would have no words at all,
  // and there is deliberately no fallback set to render instead.
  if (!Array.isArray(parsed.serving) || !Array.isArray(parsed.others) || !parsed.wording) {
    throw new Error(
      `The accusation list for scenario "${scenarioId}" is missing serving/others/wording ` +
        `— backend/frontend contract mismatch. ` +
        `If this persists, report it to the site administrator.`,
    );
  }
  return parsed as AllegationOptions;
}

/**
 * Link one statement to one or more accusations, with one cut.
 *
 * @param slug          the case, for authorisation — not part of the key
 * @param graphNodeId   the statement; the whole key, case-wide
 * @param allegationIds what the human ticked. An empty list is refused by the
 *                      backend in the stored words the panel also uses.
 * @param cut           required — a link with no cut cannot tell ammunition from
 *                      hazard, and the backend refuses one.
 */
export async function saveLinks(
  slug: string,
  graphNodeId: string,
  allegationIds: string[],
  cut: LinkCut,
): Promise<SaveLinksResult> {
  const response = await authFetch(linksUrl(slug, graphNodeId), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ allegation_ids: allegationIds, cut }),
  });

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to link "${graphNodeId}" to the accusations you chose ` +
        `(HTTP ${response.status}${detail}). Nothing was saved.`,
    );
  }
  return (await response.json()) as SaveLinksResult;
}

/**
 * Take one link back.
 *
 * Succeeds whether or not the pair was linked: the caller asked for it unlinked
 * and unlinked is what it is. The result says which happened, because "removed
 * nothing" means the view was stale.
 */
export async function unlink(
  slug: string,
  graphNodeId: string,
  allegationId: string,
): Promise<UnlinkResult> {
  const url = `${linksUrl(slug, graphNodeId)}/${encodeURIComponent(allegationId)}`;
  const response = await authFetch(url, { method: "DELETE" });

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to unlink "${graphNodeId}" from that accusation ` +
        `(HTTP ${response.status}${detail}). The link is still in place.`,
    );
  }
  return (await response.json()) as UnlinkResult;
}
