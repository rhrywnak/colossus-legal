/**
 * The Include picker, as it actually renders (CC_TASK_INCLUDE_PICKER_v1).
 *
 * `renderToStaticMarkup` is a pure function from React elements to an HTML
 * string — no DOM, no RTL, nothing CLAUDE.md rule 30 says is not set up here. So
 * "the row appears on the card the mode names, and carries that card's own
 * accusations" is checkable against real output rather than against a promise.
 *
 * What these cannot reach is interaction — whether clicking Save fires. That is
 * `includePickerModel.test.ts`'s job, and it covers the DECISION behind every
 * control here.
 */
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { CandidateCard } from "../CandidateCard";
import IncludePickerRow from "../IncludePickerRow";
import { includeOptions } from "../includePickerModel";
import { cardFixture, optionsFixture } from "./cardFixtures";
import type { CardBearsOn } from "../../services/scenarioCards";

const options = optionsFixture();
const grammar = options.card_grammar;

const BEARS_ON: CardBearsOn[] = [
  {
    allegation_id: "alleg-71",
    accusation: "A-71 — Defendants intend to abandon social security funds",
    elements: ["Breach of the fiduciary duty"],
    count: "Count 1 — Breach of Fiduciary Duty",
  },
  {
    allegation_id: "alleg-74",
    accusation: "A-74 — CFS owed a fiduciary duty to named heirs",
    elements: ["Existence of fiduciary duty"],
    count: "Count 1 — Breach of Fiduciary Duty",
  },
];

/** The row on its own, answered as the mockup draws it. */
function row(over: Partial<React.ComponentProps<typeof IncludePickerRow>> = {}) {
  return renderToStaticMarkup(
    <IncludePickerRow
      options={includeOptions(BEARS_ON, [])}
      allegationId="alleg-71"
      stance="supports"
      wording={grammar}
      onAllegation={() => {}}
      onStance={() => {}}
      onSave={() => {}}
      onCancel={() => {}}
      {...over}
    />,
  );
}

/** One card in the list, with the picker open on it or not. */
function card(including: { allegationId: string | null; stance: "supports" | "rebuts" } | null) {
  return renderToStaticMarkup(
    <CandidateCard
      card={cardFixture({ graphNodeId: "ev-1", bearsOn: BEARS_ON })}
      selected
      compact={false}
      onSelect={() => {}}
      onRule={() => {}}
      linkOptions={options}
      onSaveLinks={async () => {}}
      onUnlink={() => {}}
      includePicker={
        including
          ? {
              options: includeOptions(BEARS_ON, []),
              allegationId: including.allegationId,
              stance: including.stance,
              onAllegation: () => {},
              onStance: () => {},
              onSave: () => {},
              onCancel: () => {},
            }
          : null
      }
    />,
  );
}

describe("the row renders the card's own accusations", () => {
  it("offers every bears-on entry, with the first one selected", () => {
    const html = row();

    expect(html).toContain("A-71 — Defendants intend to abandon social security funds");
    expect(html).toContain("A-74 — CFS owed a fiduciary duty to named heirs");
    // The default is the card's FIRST bears-on — what the human is looking at.
    expect(html).toMatch(/<option selected="" value="alleg-71"|value="alleg-71" selected/);
  });

  it("shows the CHOOSE prompt only on a card that bears on nothing", () => {
    expect(row({ options: [], allegationId: null })).toContain(
      grammar.include_picker_choose_prompt,
    );
    // A card WITH an accusation opens on it, so there is nothing to choose
    // toward — an empty option there would be a way to un-choose into a state
    // Save refuses.
    expect(row()).not.toContain(grammar.include_picker_choose_prompt);
  });

  it("disables Save until an accusation is chosen", () => {
    expect(row({ options: [], allegationId: null })).toContain("disabled");
    expect(row()).not.toContain("disabled");
  });

  it("says which stance is chosen, in the store's words", () => {
    const helpsUs = row({ stance: "supports" });
    expect(helpsUs).toContain(grammar.include_picker_helps_us_label);
    expect(helpsUs).toContain(grammar.include_picker_helps_them_label);
    expect(helpsUs).toMatch(/aria-pressed="true"[^>]*>Helps us|>Helps us</);

    // The OTHER state is reachable and says so.
    expect(row({ stance: "rebuts" })).toContain('aria-pressed="true"');
  });

  it("never ships a WIRE token to the screen", () => {
    // `supports` / `rebuts` are the graph's vocabulary. A human reads "Helps us".
    const html = row();
    expect(html).not.toContain(">supports<");
    expect(html).not.toContain(">rebuts<");
  });
});

describe("every sentence on the row comes from the store", () => {
  /**
   * A grammar whose six picker words are DELIBERATELY not the shipped ones.
   *
   * The first version of this test used the ordinary fixture, whose values are
   * the same sentences the migration seeds — so a component that compiled
   * "Include under:" in passed it. The mutation proof caught that and this is
   * the repair: if any word below fails to appear, the component is speaking for
   * itself instead of reading the store.
   */
  const marked = {
    ...grammar,
    include_picker_label: "«UNDER»",
    include_picker_choose_prompt: "«CHOOSE»",
    include_picker_helps_us_label: "«FOR US»",
    include_picker_helps_them_label: "«FOR THEM»",
    include_picker_save_label: "«SAVE»",
    include_picker_cancel_label: "«CANCEL»",
  };

  it("renders no label this build compiled in (M)", () => {
    const stored = Object.values(marked).filter((v) => typeof v === "string" && v.startsWith("«"));
    const html = row({ options: [], allegationId: null, wording: marked });

    // Every one of the six reaches the screen…
    for (const word of stored) expect(html).toContain(word);

    // …and NOTHING else does. Any visible line that is not a stored word is a
    // sentence this build invented.
    const text = html
      .replace(/<[^>]*>/g, "\n")
      .split("\n")
      .map((s) => s.trim())
      .filter(Boolean);
    for (const line of text) {
      expect(stored, `"${line}" is on screen and in no settings row`).toContain(line);
    }
  });
});

describe("Include is wired to the picker, not to the action", () => {
  it("renders the row on the card the mode names", () => {
    const html = card({ allegationId: "alleg-71", stance: "supports" });

    expect(html).toContain(grammar.include_picker_label);
    expect(html).toContain(grammar.include_picker_save_label);
  });

  it("renders NOTHING on a card the mode does not name (M)", () => {
    // The fence. `CandidateList` hands each card the mode only when it names
    // that card, so a row can appear on a card it is not about by no route at
    // all — and this is the assertion that keeps that true.
    const html = card(null);

    expect(html).not.toContain(grammar.include_picker_label);
    expect(html).not.toContain(grammar.include_picker_save_label);
  });
});

describe("the accent is a stripe or a fill, never furniture", () => {
  it("paints --accent-primary as a 3px bar down the left edge", () => {
    // Roman's standing rule (2026-08-31): `--accent-primary` is INK or FILL,
    // never a 1-px hairline. `ScanConfirmBar` carries the same construction, and
    // `accentHairline.test.ts` names "3-px accent bars down the left of a row"
    // as one of the shapes the rule permits.
    expect(row()).toContain("border-left:3px solid var(--accent-primary)");
  });

  it("uses the accent as a 1px border ONLY where it also fills (M)", () => {
    // The other shape that file permits: "filled buttons whose border matches
    // their own fill". Save is one. What the rule forbids is an accent hairline
    // around something that is NOT accent-filled — a soft-ground button, a
    // panel, a divider — so this checks the pairing rather than banning the
    // border outright.
    const html = row();

    for (const style of html.match(/style="[^"]*"/g) ?? []) {
      if (!style.includes("border:1px solid var(--accent-primary)")) continue;
      expect(
        style,
        `an accent hairline with no accent fill behind it: ${style}`,
      ).toContain("background:var(--accent-primary)");
    }

    // And the row's own box takes the neutral hairline, like every other box.
    expect(html).toContain("border:1px solid var(--border-default)");
  });
});
