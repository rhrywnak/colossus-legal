/**
 * Unit tests for the pure candidate-panel seed logic (B2b).
 *
 * The panel that consumes this has no component-test infra (Rule 30), so all the
 * risk-bearing behavior — name resolution, normalization, exact-tag matching,
 * partial seeding, and the un-authored fallback — is pinned here.
 */
import { describe, expect, it } from "vitest";

import { seedFiltersFromDefinition } from "../candidateSeed";
import type { ScenarioDefinition } from "../../pages/trialPrepData";
import type { ActorOption } from "../../services/bias";

// ── Fixtures ─────────────────────────────────────────────────────────────────

const actor = (id: string, name: string): ActorOption => ({
  id,
  name,
  actor_type: "Person",
  tagged_statement_count: 1,
});

const SUBJECTS: ActorOption[] = [
  actor("person-marie", "Marie Awad"),
  actor("person-nadia", "Nadia Awad"),
];
const ACTORS: ActorOption[] = [
  actor("person-george", "George Phillips"),
  actor("org-cfs", "CFS"),
];
const PATTERN_TAGS = ["Disparagement", "obstruction", "selective_sanctions"];

/** A fully-authored definition, overridable per test. */
const def = (over: Partial<ScenarioDefinition> = {}): ScenarioDefinition => ({
  attack_text: "Marie is obstructive",
  wielders: [],
  seed_phrases: [],
  anti_seed_phrases: [],
  schema_v: 1,
  ...over,
});

// ── Un-authored fallback ─────────────────────────────────────────────────────

describe("seedFiltersFromDefinition — un-authored fallback", () => {
  it("returns defined:false with no seeds/unresolved when definition is undefined", () => {
    const seed = seedFiltersFromDefinition(undefined, SUBJECTS, ACTORS, PATTERN_TAGS);
    expect(seed.defined).toBe(false);
    expect(seed.unresolved).toEqual([]);
    expect(seed.subjectId).toBeUndefined();
    expect(seed.actorId).toBeUndefined();
    expect(seed.patternTag).toBeUndefined();
  });

  it("treats the wire's `{}` (no attack_text) as un-authored", () => {
    // The backend delivers `{}` for an un-authored scenario — a truthy object
    // lacking attack_text. Cast because `{}` does not satisfy the type, which is
    // exactly the runtime shape the guard must catch.
    const seed = seedFiltersFromDefinition(
      {} as ScenarioDefinition,
      SUBJECTS,
      ACTORS,
      PATTERN_TAGS,
    );
    expect(seed.defined).toBe(false);
    expect(seed.unresolved).toEqual([]);
  });

  it("treats an empty attack_text as un-authored", () => {
    const seed = seedFiltersFromDefinition(
      def({ attack_text: "" }),
      SUBJECTS,
      ACTORS,
      PATTERN_TAGS,
    );
    expect(seed.defined).toBe(false);
  });
});

// ── target → subject_id ──────────────────────────────────────────────────────

describe("seedFiltersFromDefinition — target resolution", () => {
  it("resolves target to a subjectId despite case/whitespace differences", () => {
    const seed = seedFiltersFromDefinition(
      def({ target: "  marie awad  " }), // different case + surrounding space
      SUBJECTS,
      ACTORS,
      PATTERN_TAGS,
    );
    expect(seed.defined).toBe(true);
    expect(seed.subjectId).toBe("person-marie");
    expect(seed.unresolved).toEqual([]);
  });

  it("records an unresolved target and leaves subjectId undefined", () => {
    const seed = seedFiltersFromDefinition(
      def({ target: "Someone Not In The Vocab" }),
      SUBJECTS,
      ACTORS,
      PATTERN_TAGS,
    );
    expect(seed.subjectId).toBeUndefined();
    expect(seed.unresolved).toEqual([
      { field: "target", name: "Someone Not In The Vocab" },
    ]);
  });
});

// ── wielders[0] → actor_id ───────────────────────────────────────────────────

describe("seedFiltersFromDefinition — wielder resolution", () => {
  it("resolves wielders[0] to an actorId and ignores later wielders", () => {
    const seed = seedFiltersFromDefinition(
      def({ wielders: ["george phillips", "CFS"] }), // second is a valid actor too
      SUBJECTS,
      ACTORS,
      PATTERN_TAGS,
    );
    expect(seed.actorId).toBe("person-george"); // resolved from [0]
    // The second wielder ("CFS") is ignored — a single actor_id only.
    expect(seed.unresolved).toEqual([]);
  });

  it("records an unresolved wielder and leaves actorId undefined", () => {
    const seed = seedFiltersFromDefinition(
      def({ wielders: ["Nobody Here"] }),
      SUBJECTS,
      ACTORS,
      PATTERN_TAGS,
    );
    expect(seed.actorId).toBeUndefined();
    expect(seed.unresolved).toEqual([{ field: "wielder", name: "Nobody Here" }]);
  });
});

// ── seed_phrases[0] → pattern_tag ────────────────────────────────────────────

describe("seedFiltersFromDefinition — pattern tag matching", () => {
  it("matches seed_phrases[0] to a known tag and returns the VOCAB casing", () => {
    const seed = seedFiltersFromDefinition(
      def({ seed_phrases: ["disparagement"] }), // lowercase; vocab is "Disparagement"
      SUBJECTS,
      ACTORS,
      PATTERN_TAGS,
    );
    expect(seed.patternTag).toBe("Disparagement");
  });

  it("leaves patternTag undefined when no tag matches (no fuzzy)", () => {
    const seed = seedFiltersFromDefinition(
      def({ seed_phrases: ["not a known tag"] }),
      SUBJECTS,
      ACTORS,
      PATTERN_TAGS,
    );
    expect(seed.patternTag).toBeUndefined();
  });
});

// ── Partial seeding ──────────────────────────────────────────────────────────

describe("seedFiltersFromDefinition — partial seeding", () => {
  it("seeds what resolves and surfaces what does not (target ok, wielder unresolved)", () => {
    const seed = seedFiltersFromDefinition(
      def({ target: "Marie Awad", wielders: ["Ghost Actor"] }),
      SUBJECTS,
      ACTORS,
      PATTERN_TAGS,
    );
    expect(seed.subjectId).toBe("person-marie"); // partial: subject seeded
    expect(seed.actorId).toBeUndefined(); // wielder not seeded
    expect(seed.unresolved).toEqual([{ field: "wielder", name: "Ghost Actor" }]);
    expect(seed.defined).toBe(true);
  });
});
