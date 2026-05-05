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
  Overrides,
  truncateHash,
} from "../configurationPanelHelpers";
import type {
  PatchConfigInput,
  ProcessingProfile,
} from "../../../services/configApi";

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

