/**
 * Unit tests for the pure v2 definition guard.
 *
 * This is the single gate deciding whether the define form pre-fills or opens
 * blank, and whether the candidate panel seeds or falls back. Its four inputs —
 * un-authored `{}`, retired v1, malformed, clean v2 — are all pinned here (Rule
 * 30: no component-test infra, so the risk-bearing branch is tested pure).
 */
import { describe, expect, it } from "vitest";

import { parseScenarioDefinition } from "../scenarioDefinitionGuard";

describe("parseScenarioDefinition — rejects non-v2 bodies", () => {
  it("returns undefined for undefined / null / non-object", () => {
    expect(parseScenarioDefinition(undefined)).toBeUndefined();
    expect(parseScenarioDefinition(null)).toBeUndefined();
    expect(parseScenarioDefinition("nope")).toBeUndefined();
  });

  it("returns undefined for the un-authored `{}` sentinel", () => {
    expect(parseScenarioDefinition({})).toBeUndefined();
  });

  it("returns undefined for a retired v1 body (wrong schema_v)", () => {
    expect(
      parseScenarioDefinition({ attack_text: "old", schema_v: 1, wielders: ["CFS"] }),
    ).toBeUndefined();
  });

  it("returns undefined when attack_text is missing or blank", () => {
    expect(parseScenarioDefinition({ schema_v: 2 })).toBeUndefined();
    expect(parseScenarioDefinition({ schema_v: 2, attack_text: "   " })).toBeUndefined();
  });

  it("returns undefined when a wielder has an unknown role", () => {
    expect(
      parseScenarioDefinition({
        attack_text: "x",
        schema_v: 2,
        wielders: [{ party_id: "org-cfs", actor_role: "invented" }],
      }),
    ).toBeUndefined();
  });

  it("returns undefined when a wielder is missing party_id", () => {
    expect(
      parseScenarioDefinition({
        attack_text: "x",
        schema_v: 2,
        wielders: [{ actor_role: "originated" }],
      }),
    ).toBeUndefined();
  });

  it("returns undefined when wielders is present but not an array", () => {
    expect(
      parseScenarioDefinition({ attack_text: "x", schema_v: 2, wielders: "CFS" }),
    ).toBeUndefined();
  });
});

describe("parseScenarioDefinition — accepts a clean v2 body", () => {
  it("parses a minimal v2 body (required pair only)", () => {
    const parsed = parseScenarioDefinition({ attack_text: "The gift", schema_v: 2 });
    expect(parsed).toEqual({ attack_text: "The gift", schema_v: 2, wielders: [] });
  });

  it("parses a full v2 body with target, wielders, and meaning", () => {
    const parsed = parseScenarioDefinition({
      attack_text: "She refused to divide the property",
      attack_meaning: "paints her as obstructive",
      target: "person-marie",
      schema_v: 2,
      wielders: [
        { party_id: "org-cfs", actor_role: "originated" },
        { party_id: "person-tighe", actor_role: "repeated" },
      ],
    });
    expect(parsed).toEqual({
      attack_text: "She refused to divide the property",
      attack_meaning: "paints her as obstructive",
      target: "person-marie",
      schema_v: 2,
      wielders: [
        { party_id: "org-cfs", actor_role: "originated" },
        { party_id: "person-tighe", actor_role: "repeated" },
      ],
    });
  });

  it("omits optional scalars when absent (no null pollution)", () => {
    const parsed = parseScenarioDefinition({ attack_text: "x", schema_v: 2 });
    expect(parsed).not.toHaveProperty("target");
    expect(parsed).not.toHaveProperty("attack_meaning");
  });
});
