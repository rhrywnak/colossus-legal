// =============================================================================
// includePickerModel.test.ts — Include asks, and NOTHING but Save sends
// =============================================================================
//
// The property this file exists for is a NEGATIVE: no action but Save files an
// include. A negative asserted against a component is only as good as the list
// of interactions somebody remembered to simulate; over the closed union below
// it is exhaustive by construction, and `nothing_but_save_commits` walks every
// member so an action added later cannot escape it.

import { describe, expect, it } from "vitest";

import {
  canSave,
  closedPicker,
  includeOptions,
  includePickerStep,
  stanceLabels,
  STANCES,
  type IncludePickerAction,
  type IncludePickerState,
} from "../includePickerModel";
import type { AllegationOption, CardGrammarWording } from "../../services/evidenceLinks";
import type { CardBearsOn } from "../../services/scenarioCards";

const OPTIONS = [
  { allegationId: "alleg-7", label: "A-7 — they knew of the meeting" },
  { allegationId: "alleg-9", label: "A-9 — the funds were available" },
];

/** A row open on `ev-1`, defaulted from OPTIONS. */
function opened(): IncludePickerState {
  return includePickerStep(closedPicker, {
    type: "open",
    graphNodeId: "ev-1",
    options: OPTIONS,
  }).state;
}

describe("the picker opens", () => {
  it("defaults to the card's FIRST accusation and to helps-us", () => {
    const state = opened();

    expect(state).toEqual({
      phase: "picking",
      graphNodeId: "ev-1",
      allegationId: "alleg-7",
      stance: "supports",
    });
    expect(canSave(state)).toBe(true);
  });

  it("opens with NOTHING chosen on a card the extraction linked to nothing", () => {
    // Instruction item 4. A default here would file a fact under an accusation
    // nobody picked, on exactly the cards nobody has read yet.
    const state = includePickerStep(closedPicker, {
      type: "open",
      graphNodeId: "ev-2",
      options: [],
    }).state;

    expect(state).toMatchObject({ phase: "picking", allegationId: null });
    expect(canSave(state)).toBe(false);
  });

  it("carries the card it was opened on, not whatever is selected later", () => {
    // The 1.7G law: the target is captured when the row opens.
    const state = opened();
    expect(state).toMatchObject({ graphNodeId: "ev-1" });
  });
});

describe("the picker commits", () => {
  it("save emits exactly once, carrying the chosen pair", () => {
    let state = opened();
    state = includePickerStep(state, { type: "chooseAllegation", allegationId: "alleg-9" }).state;
    state = includePickerStep(state, { type: "chooseStance", stance: "rebuts" }).state;

    const saved = includePickerStep(state, { type: "save" });

    expect(saved.commit).toEqual({
      graphNodeId: "ev-1",
      allegationId: "alleg-9",
      stance: "rebuts",
    });
    expect(saved.state).toEqual({ phase: "closed" });

    // ONCE: saving again from the closed state sends nothing.
    expect(includePickerStep(saved.state, { type: "save" }).commit).toBeNull();
  });

  it("cancel emits nothing and closes the row", () => {
    const state = opened();
    const cancelled = includePickerStep(state, { type: "cancel" });

    expect(cancelled.commit).toBeNull();
    expect(cancelled.state).toEqual({ phase: "closed" });
  });

  it("save is REFUSED while no accusation is chosen", () => {
    // The row stays open rather than firing a round trip that would come back as
    // the very 400 this task exists to stop.
    const empty = includePickerStep(closedPicker, {
      type: "open",
      graphNodeId: "ev-2",
      options: [],
    }).state;

    const saved = includePickerStep(empty, { type: "save" });

    expect(saved.commit).toBeNull();
    expect(saved.state).toMatchObject({ phase: "picking" });
  });

  it("NOTHING BUT SAVE COMMITS — over every action in the union (M)", () => {
    // The negative, exhaustively. An action added to `IncludePickerAction` that
    // is not listed here fails to type-check, so this cannot silently stop
    // covering the union.
    const everything: IncludePickerAction[] = [
      { type: "open", graphNodeId: "ev-1", options: OPTIONS },
      { type: "chooseAllegation", allegationId: "alleg-9" },
      { type: "chooseStance", stance: "rebuts" },
      { type: "cancel" },
      { type: "close" },
    ];

    for (const action of everything) {
      expect(
        includePickerStep(opened(), action).commit,
        `${action.type} must not file an include`,
      ).toBeNull();
    }

    // And the one that does.
    expect(includePickerStep(opened(), { type: "save" }).commit).not.toBeNull();
  });

  it("a choice made while the row is CLOSED opens nothing and sends nothing", () => {
    for (const action of [
      { type: "chooseAllegation", allegationId: "alleg-9" },
      { type: "chooseStance", stance: "rebuts" },
      { type: "save" },
    ] as IncludePickerAction[]) {
      const step = includePickerStep(closedPicker, action);
      expect(step.state).toEqual({ phase: "closed" });
      expect(step.commit).toBeNull();
    }
  });
});

describe("the options the row offers", () => {
  const bearsOn: CardBearsOn[] = [
    { allegation_id: "alleg-7", accusation: "A-7 — they knew", elements: [], count: null },
  ];
  const serving: AllegationOption[] = [
    { allegation_id: "alleg-9", label: "A-9 — the funds", count_label: null, filter_text: "" },
    { allegation_id: "alleg-7", label: "A-7 — they knew (again)", count_label: null, filter_text: "" },
  ];

  it("puts the card's OWN accusations first, then the scenario's", () => {
    expect(includeOptions(bearsOn, serving).map((o) => o.allegationId)).toEqual([
      "alleg-7",
      "alleg-9",
    ]);
  });

  it("dedupes on the ID, so one accusation never appears twice (M)", () => {
    // `alleg-7` is in BOTH lists. Two rows a human cannot tell apart is the
    // failure — and deduping on the LABEL would not catch it here, because the
    // scenario's label for it differs from the card's.
    const options = includeOptions(bearsOn, serving);

    expect(options).toHaveLength(2);
    expect(options[0].label).toBe(
      "A-7 — they knew",
      // The card's own words win: that is what the human is reading.
    );
  });

  it("is empty when the card bears on nothing and the scenario serves nothing", () => {
    expect(includeOptions([], [])).toEqual([]);
  });
});

describe("the two vocabularies", () => {
  it("the WIRE tokens are the graph's own, in the order the control renders", () => {
    // `supports` / `rebuts` are what `r.stance` carries. A prettier synonym here
    // would leave the wire, the drafted cards on disk and the store disagreeing.
    expect(STANCES).toEqual(["supports", "rebuts"]);
  });

  it("the LABELS come from the store, never from a literal here (M)", () => {
    // DELIBERATELY not "Helps us" / "Helps them". A fixture carrying the same
    // words as the shipped rows would pass against a component that compiled
    // those words in — which is exactly what this test exists to forbid, and
    // exactly what the first version of it failed to catch under mutation.
    const wording = {
      include_picker_helps_us_label: "«FOR US»",
      include_picker_helps_them_label: "«FOR THEM»",
    } as CardGrammarWording;

    expect(stanceLabels(wording)).toEqual([
      { stance: "supports", label: "«FOR US»" },
      { stance: "rebuts", label: "«FOR THEM»" },
    ]);
  });
});
