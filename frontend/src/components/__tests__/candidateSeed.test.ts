/**
 * Unit tests for the pure candidate-panel seed logic (B2b, D1 id-based rebuild).
 *
 * The panel that consumes this has no component-test infra (Rule 30), so all the
 * risk-bearing behavior — id resolution, the schema-v2 guard, single-actor
 * seeding, stale-id surfacing, and the un-authored fallback — is pinned here.
 */
import { describe, expect, it } from "vitest";

import { seedFiltersFromDefinition } from "../candidateSeed";
import type { ScenarioDefinition, Wielder } from "../../pages/trialPrepData";
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

const wielder = (party_id: string, actor_role: Wielder["actor_role"] = "originated"): Wielder => ({
  party_id,
  actor_role,
});

/** A fully-authored v2 definition, overridable per test. */
const def = (over: Partial<ScenarioDefinition> = {}): ScenarioDefinition => ({
  attack_text: "She refused to divide the property amicably",
  wielders: [],
  schema_v: 2,
  ...over,
});

// ── Un-authored / stale fallback ─────────────────────────────────────────────

describe("seedFiltersFromDefinition — fallback cases", () => {
  it("returns defined:false when definition is undefined", () => {
    const seed = seedFiltersFromDefinition(undefined, SUBJECTS, ACTORS);
    expect(seed.defined).toBe(false);
    expect(seed.unresolved).toEqual([]);
    expect(seed.subjectId).toBeUndefined();
    expect(seed.actorId).toBeUndefined();
  });

  it("treats the wire's `{}` (no attack_text) as un-authored", () => {
    const seed = seedFiltersFromDefinition({} as ScenarioDefinition, SUBJECTS, ACTORS);
    expect(seed.defined).toBe(false);
    expect(seed.unresolved).toEqual([]);
  });

  it("treats a retired v1 body (schema_v 1) as un-authored", () => {
    // A v1 body would otherwise carry attack_text + string wielders; the schema_v
    // guard rejects it so a stale row is authored afresh, not mis-seeded.
    const v1 = { attack_text: "old", schema_v: 1, wielders: ["CFS"] } as unknown as ScenarioDefinition;
    const seed = seedFiltersFromDefinition(v1, SUBJECTS, ACTORS);
    expect(seed.defined).toBe(false);
  });

  it("treats an empty attack_text as un-authored", () => {
    const seed = seedFiltersFromDefinition(def({ attack_text: "" }), SUBJECTS, ACTORS);
    expect(seed.defined).toBe(false);
  });
});

// ── target → subject_id (id-based) ───────────────────────────────────────────

describe("seedFiltersFromDefinition — target resolution", () => {
  it("passes a known target id through as subjectId", () => {
    const seed = seedFiltersFromDefinition(
      def({ target: "person-marie" }),
      SUBJECTS,
      ACTORS,
    );
    expect(seed.defined).toBe(true);
    expect(seed.subjectId).toBe("person-marie");
    expect(seed.unresolved).toEqual([]);
  });

  it("records a stale target id (not in vocab) as unresolved, not applied", () => {
    const seed = seedFiltersFromDefinition(
      def({ target: "person-deleted" }),
      SUBJECTS,
      ACTORS,
    );
    expect(seed.subjectId).toBeUndefined();
    expect(seed.unresolved).toEqual([{ field: "target", id: "person-deleted" }]);
  });
});

// ── wielders[0].party_id → actor_id ──────────────────────────────────────────

describe("seedFiltersFromDefinition — wielder resolution", () => {
  it("resolves wielders[0] to an actorId and ignores later wielders", () => {
    const seed = seedFiltersFromDefinition(
      def({ wielders: [wielder("person-george"), wielder("org-cfs", "repeated")] }),
      SUBJECTS,
      ACTORS,
    );
    expect(seed.actorId).toBe("person-george"); // resolved from [0]
    expect(seed.unresolved).toEqual([]); // the second wielder is simply ignored
  });

  it("records a stale wielder id as unresolved", () => {
    const seed = seedFiltersFromDefinition(
      def({ wielders: [wielder("person-ghost")] }),
      SUBJECTS,
      ACTORS,
    );
    expect(seed.actorId).toBeUndefined();
    expect(seed.unresolved).toEqual([{ field: "wielder", id: "person-ghost" }]);
  });
});

// ── Partial seeding ──────────────────────────────────────────────────────────

describe("seedFiltersFromDefinition — partial seeding", () => {
  it("seeds what resolves and surfaces what does not (target ok, wielder stale)", () => {
    const seed = seedFiltersFromDefinition(
      def({ target: "person-marie", wielders: [wielder("person-ghost")] }),
      SUBJECTS,
      ACTORS,
    );
    expect(seed.subjectId).toBe("person-marie");
    expect(seed.actorId).toBeUndefined();
    expect(seed.unresolved).toEqual([{ field: "wielder", id: "person-ghost" }]);
    expect(seed.defined).toBe(true);
  });
});
