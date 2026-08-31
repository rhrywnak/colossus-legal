// =============================================================================
// scenarioAugmentation.ts — client for the augmentation panel (task 1.4)
// -----------------------------------------------------------------------------
// Endpoints:
//   GET    /api/cases/:slug/scenarios/:id/augmentation         → the whole panel
//   POST   /api/cases/:slug/scenarios/:id/human-facts          → add a C4 fact
//   DELETE /api/cases/:slug/scenarios/:id/human-facts/:factId  → remove one
//   PUT    /api/cases/:slug/scenarios/:id/talking-points       → replace C5
//
// Same idioms as `scenarioCards.ts`: `authFetch` (credentials + 30s
// AbortController timeout), `encodeURIComponent` on every path param,
// `readErrorMessage` to surface the backend `{message}`, and every non-2xx
// throws (Standing Rule 1).
//
// ## The frontend composes nothing here either
//
// `authored_tag` ("Added by Roman") and `date_label` ("Around 2009-04-21") arrive
// composed. The tag is this content's PROVENANCE — human facts carry no citation
// by design — and the date qualifier is a claim about precision. Neither is the
// browser's to assemble.
//
// The shapes mirror backend/src/dto/scenario_augmentation.rs verbatim.
// =============================================================================

import { API_BASE_URL } from "./api";
import { authFetch } from "./auth";
import { readErrorMessage } from "./fetchUtils";

export type HumanFactDto = {
  id: string;
  text: string;
  /** The date as it should READ, with its qualifier. `null` when undated. */
  date_label: string | null;
  /** People this fact names, as a human typed them. */
  person_refs: string[];
  /** Whether those names are resolved entities. `false` until task B0. */
  person_refs_are_linked: boolean;
  /** "Added by Roman" — composed server-side. */
  authored_tag: string;
  edited: boolean;
};

export type TalkingPointDto = {
  text: string;
  /**
   * Its place in the list, server-owned — AND its address.
   *
   * 1-based since task 2.11 C: it is the number printed in the pill beside the
   * point and the segment `PUT …/talking-points/:position` matches, and those
   * must be one number or editing point 2 lands on point 1.
   */
  position: number;
  authored_tag: string | null;
};

/**
 * The words the two authoring sections speak on THIS page (task 2.11 C).
 *
 * The components hold no literals: they became shared with the rehearsal page,
 * whose standing law is that every visible word is a stored row, so each surface
 * serves its own set into the same component. Mirrors the backend
 * `AuthoringWordingDto` field for field.
 */
export type AuthoringWordingDto = {
  points_section_heading: string;
  /** Carries `{cap}` — filled by {@link fillCap}. */
  points_section_meta_template: string;
  points_empty_notice: string;
  points_no_exhibit_notice: string;
  points_add_label: string;
  points_edit_label: string;
  points_save_label: string;
  points_saving_label: string;
  points_cancel_label: string;
  /** Carries `{cap}`. */
  points_cap_reached_notice: string;
  /** Carries `{n}`. */
  points_field_label_template: string;
  points_authoring_note: string;
  points_save_failed_notice: string;
  watch_section_heading: string;
  watch_section_meta: string;
  watch_field_label: string;
  watch_add_label: string;
  watch_save_label: string;
  watch_edit_label: string;
  watch_cancel_label: string;
  watch_remove_label: string;
  watch_edited_suffix: string;
  watch_save_failed_notice: string;
};

/**
 * The words the identity modal speaks about a scenario's TARGET (2026-08-07).
 *
 * Mirrors the backend `ScenarioIdentityWordingDto` field for field. It rides
 * this payload because the modal already reads it on open — a second request for
 * four strings, fired at the same instant, would buy nothing.
 */
export type ScenarioIdentityWording = {
  target_label: string;
  target_helper: string;
  target_unset_option: string;
  /** Shown when the party vocabulary cannot be read. Tells the human not to
   *  save — saving then would clear the target the scenario already has. */
  target_options_failed_notice: string;
  /** Refuses a save that names a target while "what they say" is blank — the
   *  one combination a stored definition has no valid shape for. */
  target_needs_attack_text: string;
  /** The same refusal for a typed "meant to imply" gloss. A separate sentence
   *  on purpose: it names a different field and a different remedy. */
  meaning_needs_attack_text: string;
  /** Why the header's "Rehearsal view" control is inert on a scenario that is
   *  not Ready. Carries `{status}`, filled from the status already on screen. */
  rehearsal_link_blocked_reason: string;
  /** The SHORT tooltip on the strip's disabled Rehearsal control (T5). */
  rehearsal_disabled_tooltip: string;
  edit_subsets_section_title: string;
  edit_subsets_section_hint: string;
  edit_subsets_attached_state: string;
  edit_subsets_not_attached_state: string;
  edit_subsets_attach_button: string;
  edit_subsets_detach_button: string;
  edit_subsets_preview_link: string;
  edit_subsets_create_link: string;
  edit_subsets_create_hint: string;
  /**
   * The label on the control that opens Marie's practice drill (PRACTICE v0).
   *
   * Rides this block rather than the drill's own because the SCENARIO page is
   * what speaks it, and this page already fetches this block — the alternative
   * was a whole practice-deck request to learn one word.
   */
  practice_link_label: string;

  // The unified identity vocabulary (task R2). ONE row per idea, rendered by the
  // read-only block AND the editor — so the two surfaces cannot call one column
  // two names, which is what they did until .391.
  attack_label: string;
  attack_absent: string;
  theme_label: string;
  theme_absent: string;
  theme_helper: string;
  motivation_label: string;
  motivation_absent: string;
  bears_on_label: string;
  bears_on_absent: string;
};

export type ScenarioIdentityDto = {
  code: string;
  name: string;
  direction: string;
  /**
   * `"draft"` / `"ready"` — what the Draft/Ready control both SHOWS and WRITES.
   *
   * Added for the header strip (T5). It is here rather than passed in because
   * only two of the four surfaces the strip renders on carry a status at all:
   * `PracticeDeck` and `RehearsalScenario` have none. A strip taking status as a
   * prop would have needed three payloads widened and four pages taught about
   * it — the shape `ScenarioTimelineDock` already rejected for the same reason.
   */
  status: string;
  /** Our one-sentence answer. `null` until framed. */
  theme_statement: string | null;
  /** What they want the jury to believe. `null` until written. */
  motivation: string | null;
  /** The attack as the OTHER side frames it. Never the same field as the theme. */
  attack_text: string | null;
};

export type AugmentationPanelDto = {
  identity: ScenarioIdentityDto;
  human_facts: HumanFactDto[];
  /**
   * C6 — the human-flagged watch-list notes (task 1.5).
   *
   * Split from `human_facts` by the BACKEND, not filtered here: the two render
   * in different places, and a client that forgot the filter would show a
   * watch-list note as a fact.
   */
  watch_list: HumanFactDto[];
  talking_points: TalkingPointDto[];
  /** Served, not hardcoded — it is a tunable that task 1.6 will move. */
  talking_points_cap: number;
  wording: AuthoringWordingDto;
  identity_wording: ScenarioIdentityWording;
};

/**
 * Put the served cap into a stored template.
 *
 * The one substitution these sections perform, and it is not composition: the
 * sentence is the human's and `{cap}` is the slot it leaves for a number the
 * server sent alongside it. A template edited to drop the placeholder is refused
 * by the settings write path, so an unfilled `{cap}` on screen means the store
 * was edited around the API — and it then shows verbatim, visibly wrong, which
 * is how a reader finds out. Mirrors `rehearsalSections.fillCode`.
 */
export function fillCap(template: string, cap: number): string {
  return template.replace("{cap}", String(cap));
}

/**
 * Put a row number into a stored template. Same rule as {@link fillCap}.
 */
export function fillN(template: string, n: number): string {
  return template.replace("{n}", String(n));
}

function scenarioUrl(slug: string, scenarioId: string): string {
  return (
    `${API_BASE_URL}/api/cases/${encodeURIComponent(slug)}` +
    `/scenarios/${encodeURIComponent(scenarioId)}`
  );
}

/** Load the whole panel: identity, human facts, talking points, the cap. */
export async function fetchAugmentationPanel(
  slug: string,
  scenarioId: string,
): Promise<AugmentationPanelDto> {
  const response = await authFetch(`${scenarioUrl(slug, scenarioId)}/augmentation`);

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to load the augmentation panel for scenario "${scenarioId}" ` +
        `(HTTP ${response.status}${detail}). Try again.`,
    );
  }

  const data: unknown = await response.json();
  const parsed = data as Partial<AugmentationPanelDto>;

  // Validate the load-bearing shapes so a contract mismatch throws HERE with
  // context rather than as an `undefined.map` inside the panel.
  if (
    parsed.identity == null ||
    !Array.isArray(parsed.human_facts) ||
    !Array.isArray(parsed.watch_list) ||
    !Array.isArray(parsed.talking_points)
  ) {
    throw new Error(
      `Augmentation response for scenario "${scenarioId}" is missing ` +
        `identity/human_facts/watch_list/talking_points — backend/frontend ` +
        `contract ` +
        `mismatch. If this persists, report it to the site administrator.`,
    );
  }
  return parsed as AugmentationPanelDto;
}

/** What the add-fact form sends. */
export type NewHumanFact = {
  text: string;
  /**
   * `fact` (the default) | `watch_list`.
   *
   * Omitted means a fact, matching the backend default and what every write
   * before task 1.5 meant.
   */
  kind?: string;
  /** ISO `YYYY-MM-DD`, or omitted. */
  occurred_on?: string;
  /** `exact` | `around` | `range` | `ordered`; only meaningful with a date. */
  date_type?: string;
  person_refs?: string[];
};

/** Add one human fact. */
export async function addHumanFact(
  slug: string,
  scenarioId: string,
  fact: NewHumanFact,
): Promise<void> {
  const response = await authFetch(`${scenarioUrl(slug, scenarioId)}/human-facts`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(fact),
  });

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to add the human fact to scenario "${scenarioId}" ` +
        `(HTTP ${response.status}${detail}).`,
    );
  }
}

/** Remove one human fact. */
export async function deleteHumanFact(
  slug: string,
  scenarioId: string,
  factId: string,
): Promise<void> {
  const response = await authFetch(
    `${scenarioUrl(slug, scenarioId)}/human-facts/${encodeURIComponent(factId)}`,
    { method: "DELETE" },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to remove the human fact from scenario "${scenarioId}" ` +
        `(HTTP ${response.status}${detail}).`,
    );
  }
}

/**
 * Replace the scenario's talking points.
 *
 * The whole list, not one point: C5 is a short ordered list curated as a whole,
 * and sending it keeps the ordering server-owned.
 */
export async function setTalkingPoints(
  slug: string,
  scenarioId: string,
  points: string[],
): Promise<void> {
  const response = await authFetch(`${scenarioUrl(slug, scenarioId)}/talking-points`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ points }),
  });

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to save the talking points for scenario "${scenarioId}" ` +
        `(HTTP ${response.status}${detail}).`,
    );
  }
}

/**
 * Rewrite ONE talking point, addressed by its printed position.
 *
 * ## Why this exists beside {@link setTalkingPoints}
 *
 * The list write deletes and re-inserts every row, which re-stamps each one's
 * author and its written-on date with the editor and today. That is right for a
 * reorder and wrong for a typo fix. Ruling C4b, 2026-08-06: update = update.
 */
export async function editTalkingPoint(
  slug: string,
  scenarioId: string,
  position: number,
  text: string,
): Promise<void> {
  const response = await authFetch(
    `${scenarioUrl(slug, scenarioId)}/talking-points/${encodeURIComponent(String(position))}`,
    {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ text }),
    },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to save talking point ${position} on scenario "${scenarioId}" ` +
        `(HTTP ${response.status}${detail}). Your words are still on screen.`,
    );
  }
}

/**
 * Rewrite ONE watch item, in place.
 *
 * The row keeps who wrote it and when; only its updated stamp moves, which is
 * what the "edited since written" tag reads. Until task 2.11 C a wrong word
 * could only be fixed by removing the item and writing it again, which threw
 * that provenance away.
 */
export async function editWatchItem(
  slug: string,
  scenarioId: string,
  factId: string,
  text: string,
): Promise<void> {
  const response = await authFetch(
    `${scenarioUrl(slug, scenarioId)}/human-facts/${encodeURIComponent(factId)}`,
    {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ text }),
    },
  );

  if (!response.ok) {
    const detail = await readErrorMessage(response);
    throw new Error(
      `Failed to save that watch item on scenario "${scenarioId}" ` +
        `(HTTP ${response.status}${detail}). Your words are still on screen.`,
    );
  }
}
