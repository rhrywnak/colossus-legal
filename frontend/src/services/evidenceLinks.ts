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

  // ── Task 2.13 slice 1 ──────────────────────────────────────────────────────
  /** Marks the interrogatory question above a discovery answer. */
  fact_question_label: string;
  /** Introduces the kind of statement a fact is. */
  fact_statement_kind_label: string;
  /** The heaviest weight's name. */
  fact_tier_carries_label: string;
  /** The middle weight's name — where a newly included fact lands. */
  fact_tier_backup_label: string;
  /** The lightest weight's name. */
  fact_tier_background_label: string;
  /** The prompt on a row's weight control. */
  fact_tier_prompt: string;
  /** Whether the background pile starts folded. Decided by the SERVER from the
   *  stored two-token setting, so this browser never parses it — a `collapsed` /
   *  `expanded` comparison here would treat every typo as `expanded` in silence. */
  fact_background_starts_collapsed: boolean;
  /** The folded pile's line. Carries `{count}`, the one placeholder only this
   *  side can fill: the server does not know how many rows survive the filter. */
  fact_background_count_template: string;
  /** The control that folds the pile away again. */
  fact_background_hide_label: string;
  /** The drag handle's accessible label. */
  fact_order_drag_hint: string;
  /** Said when a weight could not be stored. Carries `{code}` and `{reason}`. */
  fact_tier_save_failed_template: string;
  /** Said when a drag could not be stored. Carries `{code}` and `{reason}`. */
  fact_order_save_failed_template: string;
  /** The queue's summary when there is no pool at all — including before it has
   *  loaded, which is the state that used to claim the work was finished. */
  queue_empty_pool_summary: string;
  /** The queue's summary when a real pool exists and none of it is outstanding. */
  queue_all_ruled_summary: string;
  /** The queue's summary while the counts are not known yet — the third state,
   *  distinct from both "no candidates" and "all ruled". */
  queue_counting_summary: string;
  /** The opt-in that opens the raw evidence pool on a never-scanned scenario.
   *  Carries `{count}` (task 2.15, piece 3a). */
  queue_raw_pool_toggle_template: string;

  // ── Scan → ruling (2026-08-08) ──────────────────────────────────────────────
  /** The queue's heading when a completed scan is proposing candidates. Carries
   *  `{count}` and `{when}`. */
  queue_proposed_heading_template: string;
  /** The attribution line on a proposed card. Carries `{when}`. */
  card_proposed_attribution_template: string;
  /** The proposed-role chip. Carries `{verb}` — the chip NAMES THE SCAN as the
   *  speaker, exactly as the banded-confidence label does. */
  card_proposed_role_template: string;
  /** The badge on a card whose one ruling settles a byte-identical twin. Carries
   *  `{count}` and `{codes}`. */
  card_proposed_covers_template: string;

  // ── Ruling acknowledgment (2026-08-08) ─────────────────────────────────────
  /** Said when a ruling is STORED. Carries `{code}` and `{state}`. */
  card_ruling_saved_template: string;
  /** Said when a stored ruling takes the card out of the list the human is
   *  looking at — the "vanish". Carries `{code}` and `{filter}`. */
  card_ruling_left_filter_template: string;
  /** Said when a locked card's one-press defer records the system's own
   *  sentence, so the human can read what they signed. Carries `{reason}`. */
  card_defer_recorded_template: string;
  /** Said when a ruling could NOT be stored. Carries `{code}` and `{detail}`. */
  card_ruling_failed_template: string;
  /** Introduces the standing reason a card's Include and Exclude are shut,
   *  stated on the card's face rather than in a tooltip (D3a). */
  card_locked_condition_label: string;

  // ── Task 2.13c ──────────────────────────────────────────────────────────────
  /** Marks the answer beneath its question — the question label's partner. */
  fact_answer_label: string;
  /** Said when a fact is weighed into the background pile. Carries `{code}`. */
  fact_background_move_notice: string;
  /** The line under the section heading naming the weights and the drag. */
  fact_weights_hint: string;
  /** The count beneath the list. Carries `{shown}` and `{background}`. */
  fact_footer_template: string;
  /** The control that forgets where a human dragged one fact. */
  fact_unplace_label: string;
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

/**
 * Fill a template's `{code}` and `{reason}` in ONE pass (task 2.13).
 *
 * ## Why not two chained `.replace()` calls
 *
 * A value substituted by the first call is re-scanned by the second. A failure
 * message whose `{reason}` text happened to contain `{code}` — server prose is
 * not ours to constrain — would have that chewed out of it. Walking the template
 * once means a substituted value is never looked at again, which is the same
 * argument the backend's `render` makes for the same reason.
 */
export function fillCodeAndReason(
  template: string,
  code: string,
  reason: string,
): string {
  const values: Record<string, string> = { code, reason };
  return template.replace(/\{(code|reason)\}/g, (token, name: string) =>
    name in values ? values[name] : token,
  );
}

/**
 * Fill the footer's two counts in ONE pass (task 2.13c).
 *
 * Same single-walk shape as `fillCodeAndReason`, but for a WEAKER reason, and the
 * difference is worth stating so nobody relies on the wrong guarantee. There the
 * risk is real: `reason` is server prose that may itself contain `{code}`, and a
 * second pass would chew it out. Here both arguments are `number`, so a
 * substituted value can never contain either token and the re-scan is
 * unreachable — the type eliminates it, not the implementation.
 *
 * The single walk is kept anyway: it matches its sibling, so neither has to be
 * read differently, and it stays correct if these ever take strings.
 */
export function fillCounts(
  template: string,
  shown: number,
  background: number,
): string {
  const values: Record<string, string> = {
    shown: String(shown),
    background: String(background),
  };
  return template.replace(/\{(shown|background)\}/g, (token, name: string) =>
    name in values ? values[name] : token,
  );
}

/** Drop a count into the background pile's line. `{count}` is required on that key. */
export function fillCount(template: string, count: number): string {
  return template.replace("{count}", String(count));
}

/**
 * Fill any set of named slots in ONE pass (ONE_CARD_GRAMMAR).
 *
 * ## Why a general filler now, after five specific ones
 *
 * The five above each name their own slots in their own regexes, which was
 * right while there were two of them and is five copies of one walk now. This
 * task adds templates carrying `{ruled}/{total}/{filter}`, `{tier}` and
 * `{value}`, and a sixth and seventh hand-written regex would be the point at
 * which the pattern stops paying for itself.
 *
 * ## The single walk is the whole safety property
 *
 * `String.replace` with a function callback visits each match of the ORIGINAL
 * string once; a substituted value is never re-scanned. That matters because
 * some of these values are server prose — a failure `{reason}` that happened to
 * contain `{code}` would have it chewed out by a second chained `.replace`.
 * Chaining is the bug this shape exists to prevent, and it is the same argument
 * the backend's `render` makes.
 *
 * An UNKNOWN slot is left standing rather than blanked: `{tier}` still on screen
 * says loudly that a caller forgot to supply it, where an empty gap would read
 * as a sentence that simply says less. Standing Rule 1, applied to prose.
 */
export function fillSlots(template: string, values: Record<string, string>): string {
  return template.replace(/\{(\w+)\}/g, (token, name: string) =>
    name in values ? values[name] : token,
  );
}

/**
 * The words ONE EVIDENCE CARD speaks, under either wrapper (ruling R6).
 *
 * Mirrors `backend/src/dto/card_grammar_wording.rs` field for field. A separate
 * block from `LinkPanelWording` because the two vocabularies move independently:
 * that one is the curation surface's, and this is the shape a piece of evidence
 * takes wherever it appears — which is the thing the one-card task made
 * independent of the two wrappers that render it.
 *
 * There is no default for any of these anywhere in the frontend, for the same
 * reason `LinkPanelWording` has none: a literal fallback would be exactly the
 * compiled-in string the language law deletes, and it would be the one on screen
 * on the day the store failed to load.
 */
export type CardGrammarWording = {
  // The queue frame.
  filter_proposed_label: string;
  filter_deferred_label: string;
  filter_included_label: string;
  filter_excluded_label: string;
  filter_full_pool_label: string;
  /** The ⓘ popup: what "everything ever gathered" means, in plain words. */
  full_pool_explainer: string;
  /** Carries `{ruled}`, `{total}` and `{filter}` — the ACTIVE filter's progress. */
  filter_progress_template: string;

  // The card body.
  question_expand_label: string;
  question_collapse_label: string;
  /**
   * The badge saying who transcribed the QUESTION.
   *
   * Never a speaker (ruling R2). It replaced a compiled-in "System" that sat
   * under a seven-line interrogatory, where a bare noun reads as the attribution
   * of the text above it.
   */
  question_machine_authorship_label: string;
  speaker_extracted_label: string;
  /** The chip on an item whose source recorded nobody — never a guessed name. */
  speaker_absent_label: string;
  /** Carries `{count}`. */
  elements_more_template: string;
  elements_fewer_label: string;
  context_show_label: string;
  context_hide_label: string;
  scan_reason_label: string;

  // Linking.
  link_typeahead_placeholder: string;
  link_typeahead_intro: string;
  /**
   * The same panel's sentence when the card ALREADY holds a link (ruling (a),
   * 2026-08-12). The control no longer disappears after the first link, so it
   * has to say which of the two states it is in.
   */
  already_linked_note: string;
  link_typeahead_no_match: string;
  /** Carries `{code}`. */
  link_woke_ruling_template: string;

  // The fact wrapper.
  weight_picker_label: string;
  /** Carries `{code}` and `{tier}`. */
  weight_changed_template: string;
  weight_undo_label: string;
  reset_order_label: string;
  reset_order_confirm: string;
  reset_order_confirm_yes: string;
  reset_order_confirm_cancel: string;
  /** Carries `{count}`. */
  reset_order_done_template: string;
  /** Carries `{reason}`. */
  reset_order_failed_template: string;

  // Chips as currency.
  /** Carries `{value}`. */
  chip_filter_hint_template: string;
  /** Carries `{value}`. */
  chip_filter_clear_template: string;
};

export type AllegationOptions = {
  wording: LinkPanelWording;
  /**
   * The card's own words (ruling R6).
   *
   * Served on THIS payload rather than from a second endpoint because this is
   * the scenario page's one stored-words read: `ScenarioDetailPage` fetches it
   * once and hands it to both the queue and the facts section, precisely so two
   * surfaces cannot hold two copies of one catalogue and disagree mid-session.
   */
  card_grammar: CardGrammarWording;
  /** The accusations this scenario already serves — anchor first. */
  serving: AllegationOption[];
  /** Everything else in the complaint, in paragraph order. */
  others: AllegationOption[];
  total: number;
  /**
   * How much of a discovery question a card shows before ellipsizing it (§2b).
   *
   * The NUMBER crosses the wire and not a pre-truncated string: the full
   * question has to be on the card anyway, since one click unfolds it, so
   * sending both halves would send the same sentence twice. Applying a length is
   * presentation mechanics — it chooses how the same text is SEEN, never what it
   * says — while the length itself stays a stored judgment.
   */
  card_question_truncate_chars: number;
  /** How many element chips stand before the rest fold behind "+N more" (§2b). */
  card_element_chips_visible_k: number;
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
