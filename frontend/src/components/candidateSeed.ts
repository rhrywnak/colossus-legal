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
// The panel calls `seedFiltersFromDefinition` and consumes the primitive fields
// of the result; it never reaches back into the definition object. See the panel
// for how the result drives the actual `BiasQueryFilters`.

import type { ScenarioDefinition } from "../pages/trialPrepData";
import type { ActorOption } from "../services/bias";

/**
 * The seed inputs the panel should apply. Any `*Id`/`patternTag` left `undefined`
 * means "don't constrain this dimension" — the panel falls back for that field.
 */
export interface SeedResult {
  /** Resolved `subject_id` from `definition.target`, or undefined (unresolved/absent). */
  subjectId?: string;
  /** Resolved `actor_id` from `definition.wielders[0]`, or undefined. */
  actorId?: string;
  /** A KNOWN pattern tag (vocab casing) matched from `seed_phrases[0]`, or undefined —
   *  never a raw seed phrase. */
  patternTag?: string;
  /** Names the definition asked for but the vocab could not resolve — surfaced to
   *  the user, never silently dropped. Empty when everything resolved (or the
   *  scenario is un-authored, in which case `defined` is false). */
  unresolved: Array<{ field: "target" | "wielder"; name: string }>;
  /** false = the scenario has no authored definition (no `attack_text`) → the
   *  panel uses its FALLBACK path entirely (default subject, no actor, user tag). */
  defined: boolean;
}

/**
 * Normalize a name for matching: trim + lowercase. Deliberately LOCAL — we do
 * NOT reuse `pdfHighlight.normalizeText` (a PDF-quote util; wrong coupling). A
 * definition's `target`/`wielders` are authored free-text; the vocab `name`
 * comes from the graph, so both sides are normalized before an equality match.
 */
const norm = (s: string): string => s.trim().toLowerCase();

/**
 * Resolve a free-text `name` to an id against a vocab list by normalized equality.
 * First match wins (the `AvailableFilters` contract does not guarantee `name`
 * uniqueness, so a deterministic first-match tie-break is applied). Returns
 * undefined when nothing matches.
 */
function resolveId(name: string, options: ActorOption[]): string | undefined {
  const wanted = norm(name);
  return options.find((o) => norm(o.name) === wanted)?.id;
}

/**
 * Derive the candidate-panel seed from a scenario's authored definition.
 *
 * Never throws. Missing/empty arrays are normal (the backend emits `wielders` /
 * `seed_phrases` / `anti_seed_phrases` as `[]` by default), so each is guarded.
 *
 * - `!definition?.attack_text` → `{ defined: false, unresolved: [] }`. The wire
 *   delivers `{}` (not `undefined`) for an un-authored scenario, so the "not yet
 *   defined" test keys on `attack_text`, not on `undefined`.
 * - `target` (a name) → resolve against `subjects` → `subjectId`; unresolved →
 *   recorded in `unresolved`.
 * - `wielders[0]` ONLY (the bias request takes a single `actor_id`, so only the
 *   FIRST wielder seeds it; any others are ignored) → resolve against `actors` →
 *   `actorId`; unresolved → recorded in `unresolved`.
 * - `seed_phrases[0]` → `patternTag` ONLY on an exact normalized match to a known
 *   `patternTags` entry (returning the vocab's casing, not the seed's); otherwise
 *   left undefined. No fuzzy matching.
 */
export function seedFiltersFromDefinition(
  definition: ScenarioDefinition | undefined,
  subjects: ActorOption[],
  actors: ActorOption[],
  patternTags: string[],
): SeedResult {
  // Un-authored scenario: full fallback, nothing to surface.
  if (!definition?.attack_text) {
    return { defined: false, unresolved: [] };
  }

  const unresolved: SeedResult["unresolved"] = [];
  let subjectId: string | undefined;
  let actorId: string | undefined;
  let patternTag: string | undefined;

  // target → subject_id
  const target = definition.target?.trim();
  if (target) {
    const id = resolveId(target, subjects);
    if (id) {
      subjectId = id;
    } else {
      unresolved.push({ field: "target", name: target });
    }
  }

  // wielders[0] → actor_id (only the first wielder; the request is single-actor).
  const firstWielder = (definition.wielders ?? [])[0]?.trim();
  if (firstWielder) {
    const id = resolveId(firstWielder, actors);
    if (id) {
      actorId = id;
    } else {
      unresolved.push({ field: "wielder", name: firstWielder });
    }
  }

  // seed_phrases[0] → pattern_tag (exact normalized match to a known tag only).
  const firstSeed = (definition.seed_phrases ?? [])[0]?.trim();
  if (firstSeed) {
    const wanted = norm(firstSeed);
    patternTag = patternTags.find((t) => norm(t) === wanted);
  }

  return { subjectId, actorId, patternTag, unresolved, defined: true };
}
