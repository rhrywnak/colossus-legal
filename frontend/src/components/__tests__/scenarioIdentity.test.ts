// =============================================================================
// scenarioIdentity.test.ts — the identity modal's rules (task 1.7B)
// =============================================================================
//
// The load-bearing assertion in this file is that the THREE TEXTS stay three.
// They are three sentences with three points of view, stored in three places,
// and the C1 migration says collapsing any two destroys what rehearsal mode
// reads. A mapping bug here would not crash anything — it would quietly put our
// answer where their accusation belongs, and nobody would notice until a witness
// read it back.

import { describe, expect, it } from "vitest";

import {
  canSave,
  draftFrom,
  patchFrom,
  withAllegation,
  withoutAllegation,
  type IdentityDraft,
} from "../scenarioIdentity";
import { CURRENT_SCHEMA_V, type ScenarioDefinition } from "../../pages/trialPrepData";

const definition: ScenarioDefinition = {
  attack_text: "the parties did not cooperate with each other",
  attack_meaning: "Phillips framed the sisters as unable to get along",
  target: "person-marie-awad",
  wielders: [{ party_id: "person-george-phillips", actor_role: "originated" }],
  schema_v: CURRENT_SCHEMA_V,
};

const source = {
  name: "Refused to divide property amicably",
  themeStatement: "Marie proposed a split; the estate refused it",
  motivation: "That Marie is the reason the auction was necessary",
  definition,
  anchorAllegationIds: ["alleg-54"],
};

// ── The three texts ──────────────────────────────────────────────────────────

describe("the three texts stay three", () => {
  it("seeds each text from its own field", () => {
    const draft = draftFrom(source);
    expect(draft.attackText).toBe("the parties did not cooperate with each other");
    expect(draft.themeStatement).toBe("Marie proposed a split; the estate refused it");
    expect(draft.motivation).toBe("That Marie is the reason the auction was necessary");
  });

  it("sends each text back to its own field — never to another's", () => {
    const patch = patchFrom(draftFrom(source), definition);

    expect(patch.definition?.attack_text).toBe(
      "the parties did not cooperate with each other",
    );
    expect(patch.theme_statement).toBe("Marie proposed a split; the estate refused it");
    expect(patch.motivation).toBe("That Marie is the reason the auction was necessary");

    // THEIR framing must never be written into OUR answer, or vice versa.
    expect(patch.theme_statement).not.toBe(patch.definition?.attack_text);
    expect(patch.motivation).not.toBe(patch.definition?.attack_text);
  });

  it("leaves an unwritten text empty rather than inventing a placeholder", () => {
    const draft = draftFrom({
      name: "S-1",
      themeStatement: null,
      motivation: null,
      definition: undefined,
      anchorAllegationIds: [],
    });
    expect(draft.themeStatement).toBe("");
    expect(draft.motivation).toBe("");
    expect(draft.attackText).toBe("");
  });
});

// ── The patch ────────────────────────────────────────────────────────────────

describe("the update body", () => {
  it("never sends direction — the backend refuses it and would 400", () => {
    const patch = patchFrom(draftFrom(source), definition);
    expect(patch).not.toHaveProperty("direction");
  });

  it("never sends status — readiness has its own actor-recording route", () => {
    const patch = patchFrom(draftFrom(source), definition);
    expect(patch).not.toHaveProperty("status");
  });

  it("carries through the definition fields the modal does not edit", () => {
    // The backend REPLACES the definition blob rather than merging it, so a
    // patch that dropped `target`/`wielders` would silently delete them.
    const patch = patchFrom(draftFrom(source), definition);
    expect(patch.definition?.target).toBe("person-marie-awad");
    expect(patch.definition?.wielders).toEqual(definition.wielders);
    expect(patch.definition?.schema_v).toBe(CURRENT_SCHEMA_V);
  });

  it("omits the definition entirely when the attack text is empty", () => {
    // `attack_text` is required by the backend's parse contract. An unframed
    // scenario has none, and storing "" would read later as "someone wrote
    // nothing here" rather than "nobody has written this yet".
    const draft: IdentityDraft = { ...draftFrom(source), attackText: "   " };
    expect(patchFrom(draft, definition).definition).toBeUndefined();
  });

  it("clears an emptied gloss instead of storing a blank string", () => {
    const draft: IdentityDraft = { ...draftFrom(source), attackMeaning: "  " };
    expect(patchFrom(draft, definition).definition?.attack_meaning).toBeUndefined();
  });

  it("trims what it sends", () => {
    const draft: IdentityDraft = {
      ...draftFrom(source),
      name: "  Refused to divide  ",
      themeStatement: " our answer ",
    };
    const patch = patchFrom(draft, definition);
    expect(patch.name).toBe("Refused to divide");
    expect(patch.theme_statement).toBe("our answer");
  });
});

// ── Save gating ──────────────────────────────────────────────────────────────

describe("when the modal can save", () => {
  it("requires a name — it is NOT NULL and every other surface shows it", () => {
    expect(canSave({ ...draftFrom(source), name: "   " })).toBe(false);
    expect(canSave(draftFrom(source))).toBe(true);
  });

  it("does not require the three texts", () => {
    // A human fixing a typo in the name must not be made to invent a theme
    // statement first.
    const bare: IdentityDraft = {
      name: "S-9",
      attackText: "",
      attackMeaning: "",
      themeStatement: "",
      motivation: "",
      anchorAllegationIds: [],
    };
    expect(canSave(bare)).toBe(true);
  });
});

// ── Allegation chips ─────────────────────────────────────────────────────────

describe("allegation chips", () => {
  it("adds one", () => {
    const next = withAllegation(draftFrom(source), "alleg-77");
    expect(next.anchorAllegationIds).toEqual(["alleg-54", "alleg-77"]);
  });

  it("ignores a duplicate rather than stacking it", () => {
    const next = withAllegation(draftFrom(source), "alleg-54");
    expect(next.anchorAllegationIds).toEqual(["alleg-54"]);
  });

  it("removes one", () => {
    const next = withoutAllegation(draftFrom(source), "alleg-54");
    expect(next.anchorAllegationIds).toEqual([]);
  });

  it("does not mutate the draft it was given", () => {
    const draft = draftFrom(source);
    withAllegation(draft, "alleg-77");
    withoutAllegation(draft, "alleg-54");
    expect(draft.anchorAllegationIds).toEqual(["alleg-54"]);
  });
});
