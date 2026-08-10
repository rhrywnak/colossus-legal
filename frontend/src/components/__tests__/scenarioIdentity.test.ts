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
  definitionWouldBeLost,
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
      target: "",
      anchorAllegationIds: [],
    };
    expect(canSave(bare)).toBe(true);
  });
});

// ── The target (2026-08-07) ──────────────────────────────────────────────────

describe("authoring a scenario's target", () => {
  it("is how a legacy scenario stops gathering nothing", () => {
    // The path Roman takes to complete S-3: a scenario stored with no target,
    // given one here. If the patch did not carry it, the save would appear to
    // succeed and the scenario would stay empty.
    const draft = { ...draftFrom(source), target: "person-george-phillips" };
    const patch = patchFrom(draft, definition);
    expect(patch.definition?.target).toBe("person-george-phillips");
  });

  it("removes the key when the target is cleared, rather than storing a blank", () => {
    // "Cleared" and "never chose one" are one state — the scenario gathers
    // nothing — and they must have one stored form. A `target: ""` would read
    // in the column as a choice somebody made.
    const patch = patchFrom({ ...draftFrom(source), target: "" }, definition);
    expect(patch.definition?.target).toBeUndefined();
  });

  it("keeps the wielders the modal does not edit", () => {
    // The backend REPLACES the definition blob rather than merging it, so a
    // dropped field here is a field deleted from the row.
    const patch = patchFrom({ ...draftFrom(source), target: "person-tighe" }, definition);
    expect(patch.definition?.wielders).toEqual(definition.wielders);
  });

  /** A draft with nothing in it, for the loss cases to vary one field at a time. */
  const blank: IdentityDraft = {
    name: "S-3",
    attackText: "",
    attackMeaning: "",
    themeStatement: "",
    motivation: "",
    target: "",
    anchorAllegationIds: [],
  };

  it("refuses a target with no attack text instead of silently dropping it", () => {
    // `patchFrom` omits the whole definition when the attack text is blank
    // (`attack_text` is required by the parse contract). Without this gate the
    // human would choose a person, save, and find the field empty on reopen
    // with nothing said — the exact silent-loss class this task removes.
    const draft: IdentityDraft = { ...blank, target: "person-marie-awad" };
    expect(definitionWouldBeLost(draft)).toBe("target");
    expect(canSave(draft)).toBe(false);
    // And the omission it is protecting against is real, not theoretical:
    expect(patchFrom(draft, undefined).definition).toBeUndefined();
  });

  // ── Task R1 Piece 5a: the half the target guard did not cover ─────────────

  it("refuses a typed MEANING with no attack text — the .389 silent discard", () => {
    // Audit defect 16, measured. `attack_meaning` lives inside the same
    // definition object as `target`, so it went the same way when the object was
    // omitted — but nothing guarded it. A human typed into "what that is meant to
    // imply", left "what they say" blank, saved, watched the modal close on a
    // successful write, and lost the sentence with nothing said.
    const draft: IdentityDraft = { ...blank, attackMeaning: "paints her as obstructive" };
    expect(definitionWouldBeLost(draft)).toBe("meaning");
    expect(canSave(draft)).toBe(false);
    expect(patchFrom(draft, undefined).definition).toBeUndefined();
  });

  it("names the TARGET when both would be lost", () => {
    // Two answers, one sentence to show. The target wins because it has the wider
    // blast radius — it decides what evidence the scenario can see at all.
    const draft: IdentityDraft = {
      ...blank,
      target: "person-marie-awad",
      attackMeaning: "paints her as obstructive",
    };
    expect(definitionWouldBeLost(draft)).toBe("target");
  });

  it("permits a save that loses nothing — the guard is not a demand for prose", () => {
    // The common edit: fixing a typo in the name on a scenario nobody has framed
    // yet. Refusing that would make the guard cost work rather than save it.
    expect(definitionWouldBeLost(blank)).toBeNull();
    expect(canSave(blank)).toBe(true);
  });

  it("permits a gloss once the attack text it belongs to exists", () => {
    // With an attack text present the definition IS sent, so nothing is dropped
    // and there is nothing to refuse.
    const draft: IdentityDraft = {
      ...blank,
      attackText: "the parties did not cooperate",
      attackMeaning: "paints her as obstructive",
    };
    expect(definitionWouldBeLost(draft)).toBeNull();
    expect(patchFrom(draft, undefined).definition?.attack_meaning).toBe(
      "paints her as obstructive",
    );
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
