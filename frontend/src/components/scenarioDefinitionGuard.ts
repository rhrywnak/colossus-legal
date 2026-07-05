// =============================================================================
// scenarioDefinitionGuard.ts — parse a raw stored definition into a v2 body.
// =============================================================================
//
// The scenario detail payload carries `definition` as opaque jsonb. It may be any
// of four things at read time:
//   - `undefined`           — never authored (older rows / absent key)
//   - `{}`                  — the un-authored sentinel every fresh row starts as
//   - a v1 body             — a now-retired shape (flat-string wielders, seeds…)
//   - a valid v2 body       — the current shape the form authors
//
// Only the last is safe to pre-fill the form from. This guard returns the typed
// v2 definition ONLY when the raw value genuinely is one (right `schema_v`, right
// wielder shape); for the other three it returns `undefined` and the form opens
// blank. There is deliberately NO migration of v1 → v2 (Roman's decision: stored
// rows are `{}` bar disposable dev-test rows; `schema_v` is the guard, not a
// backfill), so a v1 body is simply treated as "author afresh."
//
// Pure (no React, no fetch) so this risk-bearing branch is unit-tested without
// component-test infra (Rule 30). Both the form (pre-fill) and `candidateSeed`
// (B2b seeding) route their raw definition through the same guard, so neither can
// mis-read a stale body.

import {
  CURRENT_SCHEMA_V,
  type ActorRole,
  type ScenarioDefinition,
  type Wielder,
} from "../pages/trialPrepData";
import { ACTOR_ROLE_OPTIONS } from "./scenarioFormLabels";

/** The valid role tokens, derived from the one label-config list (no second copy). */
const VALID_ROLES: ReadonlySet<string> = new Set(
  ACTOR_ROLE_OPTIONS.map((o) => o.code),
);

/** Narrow an unknown to a valid `ActorRole` (a token the backend enum accepts). */
function asActorRole(value: unknown): ActorRole | undefined {
  return typeof value === "string" && VALID_ROLES.has(value)
    ? (value as ActorRole)
    : undefined;
}

/**
 * Narrow one raw array element to a typed `Wielder`. A malformed entry (missing
 * `party_id`, unknown `actor_role`) yields `undefined` — the caller drops the
 * whole definition rather than pre-filling a partial/garbled wielder list, so a
 * corrupt body can never masquerade as authored.
 */
function asWielder(value: unknown): Wielder | undefined {
  if (value === null || typeof value !== "object") return undefined;
  const party_id = (value as { party_id?: unknown }).party_id;
  const role = asActorRole((value as { actor_role?: unknown }).actor_role);
  if (typeof party_id !== "string" || party_id.length === 0 || !role) {
    return undefined;
  }
  return { party_id, actor_role: role };
}

/**
 * Parse a raw stored definition into a typed **v2** `ScenarioDefinition`, or
 * `undefined` if it is un-authored / a retired v1 body / malformed.
 *
 * Rejects (→ `undefined`) when the raw value is not an object, is missing a
 * non-empty `attack_text`, does not carry `schema_v === CURRENT_SCHEMA_V`, or has
 * a `wielders` array any element of which is not a valid `{party_id, actor_role}`.
 */
export function parseScenarioDefinition(
  raw: unknown,
): ScenarioDefinition | undefined {
  if (raw === null || typeof raw !== "object") return undefined;
  const obj = raw as Record<string, unknown>;

  // schema_v gate — a v1/`{}`/newer body is "not this shape," open blank.
  if (obj.schema_v !== CURRENT_SCHEMA_V) return undefined;

  // attack_text is the required, load-bearing field — absent/blank → un-authored.
  const attack_text = obj.attack_text;
  if (typeof attack_text !== "string" || attack_text.trim().length === 0) {
    return undefined;
  }

  // wielders: absent → []; present but not an array, or any bad entry → reject
  // the whole body (do not silently drop entries — a garbled list means the row
  // is not a clean v2 body we should pre-fill from).
  let wielders: Wielder[] = [];
  if (obj.wielders !== undefined) {
    if (!Array.isArray(obj.wielders)) return undefined;
    const mapped = obj.wielders.map(asWielder);
    if (mapped.some((w) => w === undefined)) return undefined;
    wielders = mapped as Wielder[];
  }

  const attack_meaning =
    typeof obj.attack_meaning === "string" ? obj.attack_meaning : undefined;
  const target = typeof obj.target === "string" ? obj.target : undefined;

  return {
    attack_text,
    schema_v: CURRENT_SCHEMA_V,
    wielders,
    ...(attack_meaning !== undefined ? { attack_meaning } : {}),
    ...(target !== undefined ? { target } : {}),
  };
}
