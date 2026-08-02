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
  position: number;
  authored_tag: string | null;
};

export type ScenarioIdentityDto = {
  code: string;
  name: string;
  direction: string;
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
  talking_points: TalkingPointDto[];
  /** Served, not hardcoded — it is a tunable that task 1.6 will move. */
  talking_points_cap: number;
};

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
    !Array.isArray(parsed.talking_points)
  ) {
    throw new Error(
      `Augmentation response for scenario "${scenarioId}" is missing ` +
        `identity/human_facts/talking_points — backend/frontend contract ` +
        `mismatch. If this persists, report it to the site administrator.`,
    );
  }
  return parsed as AugmentationPanelDto;
}

/** What the add-fact form sends. */
export type NewHumanFact = {
  text: string;
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
