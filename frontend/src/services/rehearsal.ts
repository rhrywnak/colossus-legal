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
// No motivation, no confidence, no verdicts, no tier, no internal status
// vocabulary, no database ids, no candidate handles (v2 §10). Enforced by the
// backend DTO's shape and pinned by a test there. The types below mirror it
// verbatim, so anything missing from these shapes is missing on purpose.
//
// It DOES carry a document and a page, on instance and answer rows only — ruled
// 2026-08-06 from REHEARSAL_VIEW_DESIGN_v2, because a witness who cannot produce
// the source on the spot loses credibility. A talking point still carries none.
// =============================================================================

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { readErrorMessage } from "./fetchUtils";

/** One talking point, optionally paired with the exhibit that backs it. */
export type RehearsalPoint = {
  /**
   * The row number a human reads — AND this point's address.
   *
   * `PUT …/talking-points/:position` matches this number, and it is the number
   * printed in the pill beside the text. One number, or editing point 2 lands
   * on point 1.
   */
  position: number;
  text: string;
  /** Her own phrasing — "My certified letter". `null` until task 3.9. */
  exhibit: string | null;
  /** The stored sentence shown when `exhibit` is null. Never a blank. */
  exhibit_notice: string;
};

/** One thing to watch for, and its address. */
export type RehearsalWatchItem = {
  /** The row id `PUT …/human-facts/:fact_id` needs. Renders nowhere. */
  id: string;
  text: string;
};

/** Where a statement was made, and how to open it at that page. */
export type RehearsalSource = {
  /** "Hearing to approve plan, p. 24" — composed from the stored template. */
  label: string;
  /** Empty when the record cannot say WHICH document; render no control then. */
  href: string;
  open_label: string;
};

/** What we said back to one instance — paired by a human, never guessed. */
export type RehearsalAnswer = {
  who: string;
  when: string | null;
  when_gap: string | null;
  source: RehearsalSource;
  quote: string;
};

/** One marked instance of them making the accusation. */
export type RehearsalInstance = {
  /** The printed row number, 1-based. */
  position: number;
  who: string;
  when: string | null;
  when_gap: string | null;
  source: RehearsalSource;
  kind_label: string;
  quote: string;
  /** The opening, cut server-side so one rule decides it. */
  quote_first_line: string;
  answer: RehearsalAnswer | null;
  /** The stored ANSWERED / NO ANSWER word. Chosen server-side, always present. */
  answer_tag: string;
  /**
   * The short red banner inside an OPENED row with no answer.
   *
   * Deliberately NOT the gap sentence: that names who/when/where and lives once,
   * in the prep list. Rendering both was the beta.381 duplicate-gap defect.
   */
  answer_banner: string | null;
};

/** The four gap tokens. Branch on these, never on the sentence. */
export const GAP_NO_ANSWER = "no_answer_prepared";
export const GAP_ACCUSATION_REMOVED = "accusation_removed";
export const GAP_ANSWER_REMOVED = "answer_removed";
export const GAP_INSTANCE_UNAVAILABLE = "instance_unavailable";

export type RehearsalGap = {
  kind: string;
  message: string;
  /**
   * The instance row this gap is about, so the entry can jump to it.
   *
   * `null` for the two kinds with no rendered row — a removed accusation, and an
   * instance the record store cannot produce. A link to nowhere would scroll a
   * reader to somebody else's statement.
   */
  position: number | null;
};

export type RehearsalAccusation = {
  text: string | null;
  text_gap: string | null;
  /**
   * Who wrote the sentence, and when — a FINISHED line, or the stored
   * "author not recorded" notice. `null` when there is no sentence to attribute.
   */
  attribution: string | null;
  count_line: string | null;
  no_instances_notice: string | null;
  instances: RehearsalInstance[];
  gaps: RehearsalGap[];
  /**
   * How many gaps there are, as its own number.
   *
   * Read this for the header rather than counting `gaps`: the header stays
   * visible when the section is FOLDED, and a count derived from a list a client
   * might filter is a count that can quietly disagree with it.
   */
  gap_count: number;
};

/** `their_words` or `our_answer`. */
export const SIDE_THEIR_WORDS = "their_words";
export const SIDE_OUR_ANSWER = "our_answer";

export type RehearsalTimelineEntry = {
  when: string;
  who: string;
  /** The token — branch on this for the colour, never on the label. */
  side: string;
  /** The stored word for that token: "THEY SAY" / "OUR ANSWER". */
  side_label: string;
  quote: string;
  /** Where it was said, and how to open it (ruling C7). */
  source: RehearsalSource;
};

export type RehearsalTimeline = {
  /** Empty when the placed items cannot draw one — `notice` then says so. */
  entries: RehearsalTimelineEntry[];
  notice: string | null;
  people: string[];
  filter_prompt: string;
  filter_all_label: string;
};

/** The count line on each section header, visible open or folded. */
export type RehearsalHeaders = {
  accusation: string;
  timeline: string;
  points: string;
  watch_for: string;
};

/** One ready scenario, as the seven blocks. */
export type RehearsalScenario = {
  code: string;
  /**
   * The scenario's address on the guarded write routes. Renders NOWHERE.
   *
   * It is here because this page edits — the accusation sentence, "What this
   * is", the points and the watch items all call the same routes the working
   * view calls, and a route needs an address (task 2.11 C, ruling C1).
   */
  scenario_id: string;
  title: string;
  what_this_is: string | null;
  what_this_is_gap: string | null;
  /** The finished authorship line, or the stored unknown notice. */
  what_this_is_attribution: string | null;
  accusation: RehearsalAccusation;
  timeline: RehearsalTimeline;
  points: RehearsalPoint[];
  points_gap: string | null;
  watch_for: RehearsalWatchItem[];
  watch_for_gap: string | null;
  headers: RehearsalHeaders;
  /**
   * Whether the instance rows arrive open or one line tall.
   *
   * The SERVER's decision, from `rehearsal_instance_rows_expand_max` and the
   * count it just rendered. Not a limit on what is shown: every instance is on
   * the payload and the list is never paginated, at any size.
   */
  instances_start_expanded: boolean;
};

/** The standing card. Never collapsible. */
export type RehearsalAlways = { heading: string; lines: string[] };

/** Which sections start open — decided on the server, never parsed here. */
export type RehearsalCollapse = {
  what_open: boolean;
  accusation_open: boolean;
  timeline_open: boolean;
  points_open: boolean;
  watch_for_open: boolean;
};

/**
 * Every word the page renders that is not already a composed sentence.
 *
 * The templates are absent by construction — there is no field for the count
 * template or a gap template to travel in, so this client could not recompose a
 * count if it tried. That absence IS the §10 exclusion, made structural.
 */
export type RehearsalWording = {
  /** Marks the paired answer beneath its instance. Borrowed from B1's rows. */
  answer_label: string;
  page_heading: string;
  purpose_line: string;
  previous_label: string;
  next_label: string;
  nothing_ready_notice: string;
  /** Carries `{code}` — filled by `fillCode`. */
  not_ready_notice: string;
  expand_all_label: string;
  collapse_all_label: string;
  block_what_heading: string;
  block_accusation_heading: string;
  block_timeline_heading: string;
  block_points_heading: string;
  block_watch_heading: string;
  crumb_trial_prep_label: string;
  scenario_page_label: string;
  go_to_row_label: string;
  prep_list_heading: string;
  row_open_hint: string;
  add_point_label: string;
  add_watch_label: string;
  points_authoring_note: string;
  editor: RehearsalEditorWording;
};

/**
 * The words this page's editors speak — borrowed from the working view's rows
 * rather than seeded again (ruling C3). "Edit", "Save", "Withdraw it" and
 * "Cancel" are already stored and already rehearsal-voiced.
 */
export type RehearsalEditorWording = {
  edit_label: string;
  save_label: string;
  cancel_label: string;
  withdraw_label: string;
  accusation_placeholder: string;
  what_placeholder: string;
  /** Carries `{detail}` — the failure's own words, filled by {@link fillDetail}. */
  save_failed_template: string;
};

export type RehearsalPayload = {
  scenarios: RehearsalScenario[];
  always: RehearsalAlways;
  wording: RehearsalWording;
  collapse: RehearsalCollapse;
  /** "Scenario 2 of 5", one per scenario, already filled in. */
  positions: string[];
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
  // The shapes this page cannot render without. Missing `wording` would leave
  // every heading blank (R4: there is no literal to fall back to), and a missing
  // `always` would drop the one block §10 makes unmissable — silently, which is
  // worse than a stated failure.
  if (
    !Array.isArray(parsed.scenarios) ||
    !Array.isArray(parsed.positions) ||
    parsed.always == null ||
    !Array.isArray(parsed.always.lines) ||
    parsed.wording == null ||
    parsed.collapse == null
  ) {
    throw new Error(
      `The rehearsal response for "${slug}" is missing scenarios/positions/` +
        `always/wording/collapse — backend/frontend contract mismatch. If this ` +
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
