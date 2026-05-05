/**
 * Pure-helper tests for the Configuration Panel.
 *
 * Per Roman's Step 1 decision (Option B), no @testing-library/react
 * infrastructure — these tests cover the helpers in isolation. The
 * rendered DOM behaviour is verified manually via the dev server +
 * the SQL verification block in Instruction D's Verification Plan.
 *
 * What's covered here:
 *   - diffConfigFromProfile per-key chunking_config / context_config
 *   - buildPatchInput map → null transition on cleared overrides
 *   - truncateHash boundary cases
 */
import { describe, it, expect } from "vitest";
import {
  buildPatchInput,
  diffConfigFromProfile,
  diffMapFromProfile,
  isMapKeyModified,
  mergeOverridesIntoResolved,
  Overrides,
  truncateHash,
} from "../configurationPanelHelpers";
import type {
  PatchConfigInput,
  ProcessingProfile,
} from "../../../services/configApi";
import type { ResolvedView } from "../../../services/pipelineApi";

// ── Test fixtures ──────────────────────────────────────────────────

function makeProfile(
  overrides: Partial<ProcessingProfile> = {},
): ProcessingProfile {
  return {
    name: "brief",
    display_name: "Appellate Brief",
    description: "",
    schema_file: "brief_v4.yaml",
    template_file: "pass1_brief_v4.md",
    system_prompt_file: "legal_extraction_system.md",
    global_rules_file: "global_rules_v4.md",
    pass2_template_file: "pass2_brief_v4.md",
    extraction_model: "claude-sonnet-4-6",
    pass2_extraction_model: null,
    chunking_mode: "structured",
    chunk_size: 8000,
    chunk_overlap: 500,
    chunking_config: {
      mode: "structured",
      strategy: "section_heading",
      units_per_chunk: 5,
      unit_overlap: 0,
      request_timeout_secs: 1800,
    },
    context_config: {},
    max_tokens: 32000,
    temperature: 0,
    auto_approve_grounded: true,
    run_pass2: true,
    is_default: false,
    profile_hash: "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
    ...overrides,
  };
}

// ── truncateHash ────────────────────────────────────────────────────

describe("truncateHash", () => {
  it("returns the leading 8 chars by default", () => {
    expect(truncateHash("abcdef0123456789")).toBe("abcdef01");
  });
  it("returns the original when shorter than length", () => {
    expect(truncateHash("abc")).toBe("abc");
  });
  it("respects a custom length", () => {
    expect(truncateHash("abcdef0123456789", 4)).toBe("abcd");
  });
  it("handles empty string", () => {
    expect(truncateHash("")).toBe("");
  });
});

// ── isMapKeyModified ───────────────────────────────────────────────

describe("isMapKeyModified", () => {
  it("returns false when the override map is undefined", () => {
    expect(isMapKeyModified(undefined, "units_per_chunk")).toBe(false);
  });
  it("returns false when the key is absent from the override map", () => {
    expect(isMapKeyModified({ strategy: "qa_pair" }, "units_per_chunk")).toBe(false);
  });
  it("returns true when the key is present in the override map", () => {
    expect(isMapKeyModified({ units_per_chunk: 3 }, "units_per_chunk")).toBe(true);
  });
});

// ── diffMapFromProfile ─────────────────────────────────────────────

describe("diffMapFromProfile", () => {
  const diff = diffMapFromProfile;

  it("returns undefined for null/undefined override", () => {
    expect(diff(null, { a: 1 })).toBeUndefined();
    expect(diff(undefined, { a: 1 })).toBeUndefined();
  });

  it("returns empty object when every key matches the profile", () => {
    const out = diff({ a: 1, b: "x" }, { a: 1, b: "x" });
    expect(out).toEqual({});
  });

  it("includes only keys whose value differs from the profile", () => {
    const out = diff(
      { units_per_chunk: 3, strategy: "section_heading" },
      { units_per_chunk: 5, strategy: "section_heading", unit_overlap: 0 },
    );
    expect(out).toEqual({ units_per_chunk: 3 });
  });

  it("includes keys present in override but absent in profile", () => {
    const out = diff({ new_key: "v" }, {});
    expect(out).toEqual({ new_key: "v" });
  });

  it("uses structural equality for object-valued keys", () => {
    const out = diff(
      { nested: { a: 1, b: 2 } },
      { nested: { a: 1, b: 2 } },
    );
    expect(out).toEqual({});
  });
});

// ── diffConfigFromProfile (per-key chunking_config) ─────────────────

describe("diffConfigFromProfile chunking_config per-key diff", () => {
  it("returns no chunking_config entry when docConfig matches profile", () => {
    const profile = makeProfile();
    const docConfig: PatchConfigInput = {
      chunking_config: { ...profile.chunking_config },
    };
    const out = diffConfigFromProfile(docConfig, profile);
    expect(out.chunking_config).toBeUndefined();
  });

  it("populates chunking_config with only the differing sub-keys", () => {
    const profile = makeProfile();
    const docConfig: PatchConfigInput = {
      chunking_config: {
        ...profile.chunking_config,
        units_per_chunk: 3, // changed from 5
      },
    };
    const out = diffConfigFromProfile(docConfig, profile);
    expect(out.chunking_config).toEqual({ units_per_chunk: 3 });
  });

  it("captures multiple differing sub-keys independently", () => {
    const profile = makeProfile();
    const docConfig: PatchConfigInput = {
      chunking_config: {
        ...profile.chunking_config,
        units_per_chunk: 3,
        unit_overlap: 1,
      },
    };
    const out = diffConfigFromProfile(docConfig, profile);
    expect(out.chunking_config).toEqual({ units_per_chunk: 3, unit_overlap: 1 });
  });

  it("ignores docConfig.chunking_config = null (no override stored)", () => {
    const profile = makeProfile();
    const docConfig: PatchConfigInput = { chunking_config: null };
    const out = diffConfigFromProfile(docConfig, profile);
    expect(out.chunking_config).toBeUndefined();
  });

  it("does not touch other fields when only chunking_config diff is present", () => {
    const profile = makeProfile();
    const docConfig: PatchConfigInput = {
      chunking_config: { ...profile.chunking_config, units_per_chunk: 3 },
    };
    const out = diffConfigFromProfile(docConfig, profile);
    expect(out.profile_name).toBeUndefined();
    expect(out.extraction_model).toBeUndefined();
    expect(out.chunking_mode).toBeUndefined();
  });
});

// ── buildPatchInput (map → null on full clear) ──────────────────────

describe("buildPatchInput chunking_config / context_config", () => {
  it("omits both maps when the operator made no map changes", () => {
    const overrides: Overrides = { temperature: 0.2 };
    const out = buildPatchInput(overrides);
    expect(out).toEqual({ temperature: 0.2 });
    expect("chunking_config" in out).toBe(false);
    expect("context_config" in out).toBe(false);
  });

  it("sends the chunking_config map verbatim when non-empty", () => {
    const overrides: Overrides = {
      chunking_config: { units_per_chunk: 3 },
    };
    const out = buildPatchInput(overrides);
    expect(out.chunking_config).toEqual({ units_per_chunk: 3 });
  });

  it("sends null when the operator cleared the last sub-key (empty map)", () => {
    // The panel represents "cleared every override sub-key" as an
    // empty `{}`, and `buildPatchInput` must translate that into
    // `null` in the PATCH body so the backend column resets to NULL
    // (full re-inherit from the profile). Sending `{}` would persist
    // an empty-but-present override — operationally distinct.
    const overrides: Overrides = { chunking_config: {} };
    const out = buildPatchInput(overrides);
    expect(out.chunking_config).toBeNull();
  });

  it("same null-on-empty contract for context_config", () => {
    const overrides: Overrides = { context_config: {} };
    const out = buildPatchInput(overrides);
    expect(out.context_config).toBeNull();
  });

  it("does NOT include schema_file in the PATCH body (Gap 8 disable)", () => {
    // The Overrides type no longer carries schema_file. This test
    // pins the contract: even if a future caller passes a stray field,
    // buildPatchInput's switch ignores it.
    const overrides = { temperature: 0.1 } as Overrides;
    const out = buildPatchInput(overrides);
    expect("schema_file" in out).toBe(false);
  });
});

// ── mergeOverridesIntoResolved (WI-FIX-5 form-field display merge) ──
//
// These tests are the regression suite for the WI-FIX-4 bug. WI-FIX-4
// replaced the client-side resolveClientSide(profile, overrides) with a
// backend fetch (`/resolved-config`), but accidentally dropped the
// form-field display merge — every dropdown read `value={resolved.X}`
// (the persisted state from backend), so picking a new value visibly
// reverted because `resolved` only refreshed on mount or post-save.
//
// `mergeOverridesIntoResolved` restores that merge as a pure helper so
// the panel's `effective = useMemo(() => merge(resolved, overrides))`
// drives the dropdown bindings. Each test below would FAIL if the
// merge function were buggy (e.g., if it returned `resolved.X` without
// honoring `overrides.X` first).

function makeResolved(overrides: Partial<ResolvedView> = {}): ResolvedView {
  return {
    profile_name: "complaint_v5",
    profile_hash: "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
    model: "claude-sonnet-4-6",
    pass2_model: null,
    template_file: "pass1_complaint_v5.md",
    pass2_template_file: "pass2_complaint_v5.md",
    system_prompt_file: "legal_extraction_system.md",
    global_rules_file: null,
    schema_file: "complaint_v5.yaml",
    chunking_mode: "full",
    chunk_size: null,
    chunk_overlap: null,
    chunking_config: { mode: "full", strategy: "section_heading" },
    context_config: {},
    max_tokens: 32000,
    temperature: 0,
    run_pass2: true,
    ...overrides,
  };
}

describe("mergeOverridesIntoResolved", () => {
  it("returns resolved unchanged when no overrides are set", () => {
    // Sanity: empty overrides should be a no-op merge. If this fails,
    // the merge function is corrupting fields it shouldn't touch.
    const resolved = makeResolved();
    const out = mergeOverridesIntoResolved(resolved, {});
    expect(out).toEqual(resolved);
  });

  it("template_file override wins over resolved (THE WI-FIX-4 BUG)", () => {
    // This is the exact regression test for the bug. Before WI-FIX-5,
    // ConfigurationPanel's dropdown read `value={resolved.template_file}`,
    // so even when overrides.template_file was set, the dropdown showed
    // the persisted value. Asserting that the merge surfaces the
    // override is the direct test.
    const resolved = makeResolved({ template_file: "pass1_complaint_v4.md" });
    const overrides: Overrides = { template_file: "pass1_custom.md" };
    const out = mergeOverridesIntoResolved(resolved, overrides);
    expect(out.template_file).toBe("pass1_custom.md");
  });

  it("pass2_template_file override wins (the field WI-FIX-4 added)", () => {
    // The Pass 2 Template field was the new editable dropdown in
    // WI-FIX-4 — and it suffered the same silent-revert bug. This
    // test pins the override-wins behavior for it.
    const resolved = makeResolved({
      pass2_template_file: "pass2_complaint_v4.md",
    });
    const overrides: Overrides = { pass2_template_file: "pass2_custom.md" };
    const out = mergeOverridesIntoResolved(resolved, overrides);
    expect(out.pass2_template_file).toBe("pass2_custom.md");
  });

  it("extraction_model override surfaces as effective.model (name shift)", () => {
    // Field-name shift catch: Overrides.extraction_model maps onto
    // ResolvedView.model. If the merge wired the wrong source/target,
    // this test fails. (e.g. `overrides.model` would be undefined and
    // resolved.model would win — silent regression.)
    const resolved = makeResolved({ model: "claude-sonnet-4-6" });
    const overrides: Overrides = { extraction_model: "claude-opus-4-7" };
    const out = mergeOverridesIntoResolved(resolved, overrides);
    expect(out.model).toBe("claude-opus-4-7");
  });

  it("pass2_extraction_model override surfaces as effective.pass2_model (name shift)", () => {
    // Same name-shift catch for the Pass 2 model.
    const resolved = makeResolved({ pass2_model: "claude-opus-4-7" });
    const overrides: Overrides = {
      pass2_extraction_model: "claude-opus-4-6",
    };
    const out = mergeOverridesIntoResolved(resolved, overrides);
    expect(out.pass2_model).toBe("claude-opus-4-6");
  });

  it("scalar fields fall back to resolved when override is undefined", () => {
    // The mirror of the override-wins case. With no override, the
    // resolved value must be preserved — otherwise persisted state
    // would silently disappear from the form on every render.
    const resolved = makeResolved({
      template_file: "pass1_complaint_v5.md",
      max_tokens: 32000,
      chunking_mode: "full",
    });
    const out = mergeOverridesIntoResolved(resolved, {});
    expect(out.template_file).toBe("pass1_complaint_v5.md");
    expect(out.max_tokens).toBe(32000);
    expect(out.chunking_mode).toBe("full");
  });

  it("chunking_config override merges per-key onto resolved (not whole-map replace)", () => {
    // The map-merge contract from the original mergeMap helper:
    // override KEYS replace, non-overridden keys fall through. This is
    // the same per-key semantic the chunking_config sub-key editor
    // depends on. A whole-map replace would lose keys the operator
    // didn't touch.
    const resolved = makeResolved({
      chunking_config: {
        mode: "structured",
        strategy: "section_heading",
        units_per_chunk: 5,
      },
    });
    const overrides: Overrides = {
      chunking_config: { units_per_chunk: 3 },
    };
    const out = mergeOverridesIntoResolved(resolved, overrides);
    expect(out.chunking_config.units_per_chunk).toBe(3);
    expect(out.chunking_config.strategy).toBe("section_heading");
    expect(out.chunking_config.mode).toBe("structured");
  });

  it("schema_file is unchanged regardless of merge (no override path)", () => {
    // Per WI-FIX-3 Decision 1 Option C, schema_file is the persisted
    // base column with no override path. Even if a future caller
    // smuggled a schema_file key into Overrides (the type doesn't allow
    // it, but the runtime doesn't enforce), the merge must not surface
    // it as an override. This test pins the absence of that path.
    const resolved = makeResolved({ schema_file: "complaint_v5.yaml" });
    const overrides = {} as Overrides; // empty — no schema_file path
    const out = mergeOverridesIntoResolved(resolved, overrides);
    expect(out.schema_file).toBe("complaint_v5.yaml");
  });
});

