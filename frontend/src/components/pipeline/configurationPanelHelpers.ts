/**
 * configurationPanelHelpers — pure logic the Configuration Panel uses.
 *
 * Factored out of `ConfigurationPanel.tsx` so the diff / build-PATCH /
 * resolve logic can be unit-tested with Vitest without dragging in a
 * DOM testing library. The panel imports these and supplies React-side
 * state.
 *
 * ## Why a separate file
 *
 * The repo's only existing frontend test pattern is service-layer
 * fetch tests (`frontend/src/services/__tests__/claims.test.ts`) — no
 * `@testing-library/react` is set up. Per Instruction D's Step 1 +
 * Roman's decision (Option B), we test only the pure functions and
 * skip rendered-DOM tests; that's what this module exists to support.
 */
import type {
  PatchConfigInput,
  ProcessingProfile,
} from "../../services/configApi";

// ── Types ───────────────────────────────────────────────────────────

/**
 * Panel-local override state. Each scalar field, when set, means the
 * operator has changed it from the profile default. Map fields
 * (`chunking_config` / `context_config`) carry **only the sub-keys
 * the operator overrode** — non-overridden sub-keys come from the
 * profile at resolve time.
 *
 * ## Rust Learning analogue
 *
 * Mirrors the backend's `PipelineConfigOverrides`:
 *   - scalar `Option<T>` ↔ TS optional field
 *   - map `Option<HashMap<K, V>>` ↔ TS optional `Record<K, V>`
 *
 * `schema_file` is intentionally absent: per Gap 8 the schema dropdown
 * is disabled (cannot be overridden per-document); the panel no longer
 * tracks a schema override. The earlier "schema_file dropped silently"
 * bug is gone because the field isn't in the type.
 */
export interface Overrides {
  profile_name?: string;
  extraction_model?: string;
  /** Pass-2 relationship-extraction model. `undefined` means "unchanged". */
  pass2_extraction_model?: string;
  template_file?: string;
  chunking_mode?: string;
  chunk_size?: number | null;
  chunk_overlap?: number | null;
  max_tokens?: number;
  temperature?: number;
  run_pass2?: boolean;
  /**
   * Per-key chunking_config override map. Only keys whose value
   * differs from the profile's value are present here. Use
   * [`isMapKeyModified`] to test "is this sub-key overridden?"
   */
  chunking_config?: Record<string, unknown>;
  /** Per-key context_config override map. Same shape as above. */
  context_config?: Record<string, unknown>;
}

/**
 * The resolved view of `profile + overrides`, as if Pass-1 were about
 * to run with these settings.
 *
 * ## Tech debt — deliberate duplication
 *
 * This is a TypeScript reimplementation of the backend's
 * `resolve_config` (`backend/src/pipeline/config.rs`). The panel needs
 * the resolved view (audit-trail section, "modified" badges) before
 * any extraction has run, so server-side resolution would require an
 * extra round-trip per state change. We duplicate the logic
 * client-side with these caveats:
 *
 * 1. **Silent drift risk.** If the backend resolver ever gains new
 *    fields or new merge semantics and this code isn't updated,
 *    the panel will display a different "resolved view" than what
 *    actually runs. The audit-trail section visibly compares against
 *    `processing_config` in `extraction_runs` — a divergence will
 *    surface there, not silently.
 * 2. **TODO: replace with `GET /config/resolved` endpoint when
 *    implemented.** That endpoint would have the backend compute the
 *    resolved view and return it; the panel would render the response
 *    directly. Tracked as tech debt; out of scope for Instruction D.
 *
 * Field set deliberately mirrors the audit-trail data the panel
 * displays: scalar values plus the merged maps. Pass-2 model fallback
 * matches the runtime: `pass2_model = override → profile → null`,
 * with the LLM call at runtime falling back to the Pass-1 model when
 * `pass2_model` is null. The audit-trail section formats null as
 * "(reuses Pass-1 model)" for the operator.
 */
export interface ResolvedView {
  profile_name: string;
  profile_hash: string;
  model: string;
  pass2_model: string | null;
  template_file: string;
  pass2_template_file: string | null;
  system_prompt_file: string | null;
  global_rules_file: string | null;
  schema_file: string;
  chunking_mode: string;
  chunk_size: number | null;
  chunk_overlap: number | null;
  chunking_config: Record<string, unknown>;
  context_config: Record<string, unknown>;
  max_tokens: number;
  temperature: number;
  run_pass2: boolean;
}

// ── Tooltip constants ──────────────────────────────────────────────
//
// Centralised so the JSX doesn't carry literal strings. Enables i18n
// later without grepping the component for prose.

export const TOOLTIPS = {
  schemaFileDisabled:
    "Schema is profile-level. To use a different schema, change the profile or edit the profile YAML. Changing schema mid-pipeline can produce items that don't match the verification path.",
  pass2TemplateReadOnly:
    "Pass-2 template is set by the profile and cannot be overridden per-document. To change it, edit the profile YAML or use a different profile.",
  resetSubKey:
    "Reset this sub-key to the profile default.",
  resolvedSection:
    "The fully merged configuration that will run, including profile defaults and per-document overrides. Hashes prove which file content was loaded.",
} as const;

// ── Pure helpers ────────────────────────────────────────────────────

/**
 * Truncate a hex hash string for display (default 8 chars). Returns
 * the original when shorter than `length`.
 */
export function truncateHash(hash: string, length = 8): string {
  return hash.length > length ? hash.slice(0, length) : hash;
}

/**
 * Structural equality for two `unknown` JSON-shaped values via
 * `JSON.stringify` round-trip.
 *
 * ## Why JSON.stringify and not deep-equal
 *
 * Our values come from `serde_json::Value` on the backend (numbers,
 * strings, bools, arrays, nested objects of the same). They round-trip
 * through JSON faithfully and don't carry circular refs or special
 * objects. `JSON.stringify` with a sorted-keys replacer would be more
 * robust against object key ordering, but our maps come straight from
 * the same JSON the backend wrote, so order is preserved between the
 * GET response and the next PATCH. Sticking with the simple
 * `JSON.stringify` keeps the helper one line and zero dependencies.
 */
function jsonEqual(a: unknown, b: unknown): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

/**
 * Compute the per-key override-map diff: which keys in `override`
 * differ from the corresponding keys in `profileMap`.
 *
 * Returns:
 *   - `undefined` when `override` is null/undefined (no override sent).
 *   - A new `Record` containing only the keys whose value differs from
 *     the profile's value. An empty object means "the operator sent
 *     an override map but every key matches the profile" — same
 *     semantic as `undefined` for the audit/UI, but kept distinct so
 *     the panel's `Overrides.chunking_config` can mirror what the
 *     backend has stored exactly.
 *
 * Keys present in `override` but absent in `profileMap` also count as
 * differences — they are operator-added new keys.
 */
export function diffMapFromProfile(
  override: Record<string, unknown> | null | undefined,
  profileMap: Record<string, unknown>,
): Record<string, unknown> | undefined {
  if (override == null) return undefined;
  const out: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(override)) {
    if (!jsonEqual(v, profileMap[k])) {
      out[k] = v;
    }
  }
  return out;
}

/**
 * Is a particular sub-key of an override map currently overridden?
 *
 * The panel's "modified" badge for a sub-key surfaces this. `undefined`
 * map (no override at all) → never modified. A map missing the key →
 * not modified. A map carrying the key → modified.
 */
export function isMapKeyModified(
  overrideMap: Record<string, unknown> | undefined,
  key: string,
): boolean {
  return (
    overrideMap !== undefined &&
    Object.prototype.hasOwnProperty.call(overrideMap, key)
  );
}

/**
 * Seed the panel's `overrides` state from the per-document
 * pipeline_config row (the GET /config response), skipping any value
 * that already matches the profile.
 *
 * At upload time the backend auto-populates pipeline_config from the
 * matched profile, so a freshly-uploaded doc's DB values equal the
 * profile's values. Marking every matching field as "modified" would be
 * wrong — only fields whose DB value genuinely differs from the
 * profile belong here.
 *
 * For chunking_config / context_config the diff is per-key (Gap 1's UI
 * half + Instruction C's audit-trail granularity).
 *
 * `system_prompt_file` is in the DB payload but not tracked by this
 * panel today (no UI surface — out of scope for Instruction D).
 * `schema_file` is no longer in `Overrides` because the schema
 * dropdown is disabled (Gap 8); any value the GET response carries
 * for it is ignored.
 */
export function diffConfigFromProfile(
  docConfig: PatchConfigInput | null,
  profile: ProcessingProfile,
): Overrides {
  if (!docConfig) return {};
  const out: Overrides = {};

  if (docConfig.profile_name != null && docConfig.profile_name !== profile.name) {
    out.profile_name = docConfig.profile_name;
  }
  if (
    docConfig.extraction_model != null &&
    docConfig.extraction_model !== profile.extraction_model
  ) {
    out.extraction_model = docConfig.extraction_model;
  }
  if (
    docConfig.pass2_extraction_model != null &&
    docConfig.pass2_extraction_model !== profile.pass2_extraction_model
  ) {
    out.pass2_extraction_model = docConfig.pass2_extraction_model;
  }
  if (
    docConfig.template_file != null &&
    docConfig.template_file !== profile.template_file
  ) {
    out.template_file = docConfig.template_file;
  }
  if (
    docConfig.chunking_mode != null &&
    docConfig.chunking_mode !== profile.chunking_mode
  ) {
    out.chunking_mode = docConfig.chunking_mode;
  }
  if (docConfig.chunk_size != null && docConfig.chunk_size !== profile.chunk_size) {
    out.chunk_size = docConfig.chunk_size;
  }
  if (
    docConfig.chunk_overlap != null &&
    docConfig.chunk_overlap !== profile.chunk_overlap
  ) {
    out.chunk_overlap = docConfig.chunk_overlap;
  }
  if (docConfig.max_tokens != null && docConfig.max_tokens !== profile.max_tokens) {
    out.max_tokens = docConfig.max_tokens;
  }
  if (
    docConfig.temperature != null &&
    docConfig.temperature !== profile.temperature
  ) {
    out.temperature = docConfig.temperature;
  }
  if (docConfig.run_pass2 != null && docConfig.run_pass2 !== profile.run_pass2) {
    out.run_pass2 = docConfig.run_pass2;
  }

  // Per-key map diffs. The override map on the DB stores whatever the
  // operator last sent; we extract only the keys that genuinely differ
  // from the profile so the panel's "modified" badge per sub-key is
  // accurate. A non-empty diff goes into `out`; an empty diff (every
  // override-map key matches the profile) does not — see jsonEqual /
  // diffMapFromProfile rationale.
  const chunkingDiff = diffMapFromProfile(
    docConfig.chunking_config,
    profile.chunking_config,
  );
  if (chunkingDiff !== undefined && Object.keys(chunkingDiff).length > 0) {
    out.chunking_config = chunkingDiff;
  }
  const contextDiff = diffMapFromProfile(
    docConfig.context_config,
    profile.context_config,
  );
  if (contextDiff !== undefined && Object.keys(contextDiff).length > 0) {
    out.context_config = contextDiff;
  }

  return out;
}

/**
 * Build the PATCH body the Configuration Panel sends on Save.
 *
 * Maps semantics match the backend (Instruction C):
 *   - omit the field entirely when the operator made no change
 *   - send the override map verbatim when the operator changed at
 *     least one sub-key
 *   - send `null` to fully clear the override (the document then
 *     re-inherits the profile's map at resolve time). This is the
 *     "cleared the last sub-key" path.
 *
 * The panel is responsible for distinguishing "user cleared the last
 * sub-key" from "user didn't touch the map" — represented in
 * `Overrides` as an empty `{}` vs `undefined` respectively.
 */
export function buildPatchInput(overrides: Overrides): PatchConfigInput {
  const out: PatchConfigInput = {};
  if (overrides.profile_name !== undefined) {
    out.profile_name = overrides.profile_name;
  }
  if (overrides.extraction_model !== undefined) {
    out.extraction_model = overrides.extraction_model;
  }
  if (overrides.pass2_extraction_model !== undefined) {
    out.pass2_extraction_model = overrides.pass2_extraction_model;
  }
  if (overrides.template_file !== undefined) {
    out.template_file = overrides.template_file;
  }
  if (overrides.chunking_mode !== undefined) {
    out.chunking_mode = overrides.chunking_mode;
  }
  if (overrides.chunk_size !== undefined) out.chunk_size = overrides.chunk_size;
  if (overrides.chunk_overlap !== undefined) {
    out.chunk_overlap = overrides.chunk_overlap;
  }
  if (overrides.max_tokens !== undefined) out.max_tokens = overrides.max_tokens;
  if (overrides.temperature !== undefined) out.temperature = overrides.temperature;
  if (overrides.run_pass2 !== undefined) out.run_pass2 = overrides.run_pass2;

  if (overrides.chunking_config !== undefined) {
    // Empty map → send null so the column resets to NULL (full
    // re-inherit from profile). Non-empty → send the map.
    out.chunking_config =
      Object.keys(overrides.chunking_config).length === 0
        ? null
        : overrides.chunking_config;
  }
  if (overrides.context_config !== undefined) {
    out.context_config =
      Object.keys(overrides.context_config).length === 0
        ? null
        : overrides.context_config;
  }
  return out;
}

/**
 * Merge two `Record<string, unknown>` maps with override keys winning.
 *
 * Backend parallel: `HashMap::extend()` over `profile.chunking_config`
 * with the override iterator. Same upsert semantics: keys present in
 * the override replace profile values; keys absent in the override
 * fall through from the profile.
 */
function mergeMap(
  profileMap: Record<string, unknown>,
  overrideMap: Record<string, unknown> | undefined,
): Record<string, unknown> {
  if (!overrideMap || Object.keys(overrideMap).length === 0) {
    return { ...profileMap };
  }
  return { ...profileMap, ...overrideMap };
}

/**
 * Compute the resolved-view: what configuration *will* run for this
 * document if the operator clicks Process now.
 *
 * Mirrors backend `resolve_config`. See [`ResolvedView`] for the
 * deliberate-duplication tech debt note.
 *
 * Two callers in the panel:
 *   1. The existing field widgets (Model dropdown, etc.) read scalar
 *      fields off this to render the "effective" value.
 *   2. The new "Resolved Configuration" audit-trail section reads
 *      everything off this to display the full-fidelity view.
 *
 * Pass-2 model resolution mirrors the backend fallback chain:
 *   override → profile.pass2_extraction_model → null
 * The runtime LLM call further falls back to the Pass-1 model when
 * `pass2_model` is null; that further fallback is presentation-layer,
 * shown by the audit section as "(reuses Pass-1 model)".
 */
export function resolveClientSide(
  profile: ProcessingProfile,
  overrides: Overrides,
): ResolvedView {
  const model = overrides.extraction_model ?? profile.extraction_model;
  const pass2_model =
    overrides.pass2_extraction_model ?? profile.pass2_extraction_model ?? null;
  return {
    profile_name: overrides.profile_name ?? profile.name,
    profile_hash: profile.profile_hash,
    model,
    pass2_model,
    template_file: overrides.template_file ?? profile.template_file,
    // No per-document override path for these — profile-level only.
    pass2_template_file: profile.pass2_template_file,
    system_prompt_file: profile.system_prompt_file,
    global_rules_file: profile.global_rules_file,
    // No per-document override path for schema_file (Gap 8 — disabled UI).
    schema_file: profile.schema_file,
    chunking_mode: overrides.chunking_mode ?? profile.chunking_mode,
    chunk_size: overrides.chunk_size ?? profile.chunk_size,
    chunk_overlap: overrides.chunk_overlap ?? profile.chunk_overlap,
    chunking_config: mergeMap(profile.chunking_config, overrides.chunking_config),
    context_config: mergeMap(profile.context_config, overrides.context_config),
    max_tokens: overrides.max_tokens ?? profile.max_tokens,
    temperature: overrides.temperature ?? profile.temperature,
    run_pass2: overrides.run_pass2 ?? profile.run_pass2,
  };
}
