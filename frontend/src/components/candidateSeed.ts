// =============================================================================
// candidateSeed.ts — pure seed logic for CandidateFactsPanel (B2b).
// =============================================================================
//
// Maps a scenario's authored `definition` onto the bias-query filter inputs, so
// the "Find candidate facts" panel auto-filters to THIS scenario's theme instead
// of the case-default subject. Pure (no React, no fetch) so the risk-bearing
// logic is unit-tested without component-test infra (CLAUDE.md Rule 30 — the
// panel itself has none).
//
// D1 rebuild: with the definition now storing `target` and `wielders[].party_id`
// as GRAPH NODE IDS chosen from the live vocabulary (not free text), seeding is
// ID-BASED — a direct id lookup against the vocab, no name normalization. The
// retired `seed_phrases` → pattern-tag branch is gone (the field no longer
// exists); the panel's own pattern dropdown remains the sole tag source.
//
// The panel calls `seedFiltersFromDefinition` and consumes the primitive fields
// of the result; it never reaches back into the definition object.

import type { ScenarioDefinition } from "../pages/trialPrepData";
import type { ActorOption } from "../services/bias";
import { parseScenarioDefinition } from "./scenarioDefinitionGuard";

/**
 * The seed inputs the panel should apply. Any `*Id` left `undefined` means "don't
 * constrain this dimension" — the panel falls back for that field.
 */
export interface SeedResult {
  /** `subject_id` from `definition.target` when it is a known subject, else undefined. */
  subjectId?: string;
  /** `actor_id` from `definition.wielders[0].party_id` when known, else undefined. */
  actorId?: string;
  /** Ids the definition named but the CURRENT vocab does not contain — surfaced to
   *  the user, never silently applied as a filter that would return nothing. Empty
   *  when everything resolved (or the scenario is un-authored → `defined` false). */
  unresolved: Array<{ field: "target" | "wielder"; id: string }>;
  /** false = no clean v2 definition (un-authored / retired v1 / malformed) → the
   *  panel uses its FALLBACK path entirely (default subject, no actor). */
  defined: boolean;
}

/**
 * Derive the candidate-panel seed from a scenario's authored definition.
 *
 * Never throws. The raw definition is routed through the shared v2 guard, so a
 * `{}` sentinel, a retired v1 body, or a malformed body all collapse to the
 * un-authored fallback (`{ defined: false }`) rather than mis-seeding.
 *
 * - No clean v2 definition → `{ defined: false, unresolved: [] }`.
 * - `target` (a subject node id) present in `subjects` → `subjectId`; a stale id
 *   (not in the current vocab) → recorded in `unresolved`, not applied.
 * - `wielders[0].party_id` ONLY (the bias request takes a single `actor_id`, so
 *   only the FIRST wielder seeds it; any others are ignored) → same id lookup
 *   against `actors`.
 */
export function seedFiltersFromDefinition(
  definition: ScenarioDefinition | undefined,
  subjects: ActorOption[],
  actors: ActorOption[],
): SeedResult {
  // Route through the v2 guard: only a clean, current-schema body seeds.
  const parsed = parseScenarioDefinition(definition);
  if (!parsed) {
    return { defined: false, unresolved: [] };
  }

  const unresolved: SeedResult["unresolved"] = [];
  let subjectId: string | undefined;
  let actorId: string | undefined;

  // target → subject_id (direct id membership check against the vocab).
  const targetId = parsed.target?.trim();
  if (targetId) {
    if (subjects.some((s) => s.id === targetId)) {
      subjectId = targetId;
    } else {
      unresolved.push({ field: "target", id: targetId });
    }
  }

  // wielders[0].party_id → actor_id (only the first wielder; single-actor request).
  const firstWielderId = parsed.wielders[0]?.party_id?.trim();
  if (firstWielderId) {
    if (actors.some((a) => a.id === firstWielderId)) {
      actorId = firstWielderId;
    } else {
      unresolved.push({ field: "wielder", id: firstWielderId });
    }
  }

  return { subjectId, actorId, unresolved, defined: true };
}
