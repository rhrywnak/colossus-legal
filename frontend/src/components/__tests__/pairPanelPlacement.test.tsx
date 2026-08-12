/**
 * WHERE a panel renders — the .394 defect family, as a test (task 394, P8).
 *
 * ## The bug family this file exists to fail the build over
 *
 * Three separate controls on the scenario page had the same defect, and it was
 * never "the control does not work". Measured in CC_REPORT_PAIRING_PICKER_DEAD_v1:
 * the answer picker rendered every single time it was asked for, at one fixed
 * insertion point at the bottom of the accusation panel — 777, 592, 407 and 92
 * pixels below the four buttons that could open it. A human clicked "Pair an
 * answer", saw nothing change, and reported the feature dead. The "+ Add human
 * fact" form had the identical shape: it rendered after a scroll region holding
 * forty-six rows.
 *
 * A test that asserted "the picker renders" would have PASSED throughout. What
 * has to be asserted is CONTAINMENT and ORDER — the panel is inside, or
 * immediately after, the control that opened it — and that is a claim about
 * rendered markup, not about source text.
 *
 * ## Why `renderToStaticMarkup` and not RTL
 *
 * CLAUDE.md rule 30 records that RTL and jsdom are not configured here, and that
 * is still true — nothing below mounts a component, fires an event or reads a
 * layout. `renderToStaticMarkup` is a pure function from React elements to an
 * HTML string and needs no DOM at all, which is the same instrument
 * `factCardAnatomy.test.tsx` already uses to pin where a C-code lands.
 *
 * The limit, stated plainly rather than discovered later: this proves the panel
 * is in the right PLACE IN THE MARKUP. It cannot prove the result is on screen,
 * legible, or unobscured — Roman's walk is what knows that. But every one of the
 * three defects above was a markup-order defect, so this is the instrument that
 * would have caught all three.
 */
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import AccusationFactPicker from "../AccusationFactPicker";
import AddHumanFactForm from "../AddHumanFactForm";
import PairCard from "../PairCard";
import SectionFold from "../SectionFold";
import WorkingView from "../WorkingView";
import { pairCardFromScenarioCard } from "../pairCardModel";
import { cardFixture, optionsFixture } from "./cardFixtures";
import type { ScenarioCard } from "../../services/scenarioCards";

const options = optionsFixture();

/**
 * The facts view's own footer template, which the shared fixture does not carry.
 *
 * Added HERE rather than to `cardFixtures`: that file exists so the one-card
 * tests cannot drift apart, and widening it for one test's needs would put a
 * value in three other suites that none of them are about.
 */
const workingWording = {
  ...options.wording,
  fact_footer_template: "{shown} shown · {background} in the background",
} as typeof options.wording;

/** The picker, with one offerable fact — enough to render its shape. */
const picker = (
  <AccusationFactPicker
    facts={[{ graphNodeId: "ev-2", code: "C-2", text: "I wrote to him twice." }]}
    prompt="Which fact answers this?"
    cancelLabel="Cancel"
    noMatchNotice="Nothing matches."
    emptyNotice="Nothing left to choose."
    excluded={[]}
    onCancel={() => {}}
    onChoose={() => {}}
  />
);

/** One pair card, optionally holding an expansion, as markup. */
function cardMarkup(over: { card?: ScenarioCard; expansion?: React.ReactNode } = {}) {
  const card = over.card ?? cardFixture({ code: "C-12" });
  return renderToStaticMarkup(
    <PairCard
      card={{
        ...pairCardFromScenarioCard(card, options.card_grammar.speaker_absent_label),
        answer: null,
      }}
      answerLabel="OUR ANSWER"
      showLabel={options.card_grammar.context_show_label}
      hideLabel={options.card_grammar.context_hide_label}
      gapNotice={null}
      controls={<button type="button">Pair an answer</button>}
      expansion={over.expansion}
    />,
  );
}

describe("P1 · the pair picker renders inside the card that opened it", () => {
  it("the picker is INSIDE the card's element, not after it", () => {
    // The whole defect, as one assertion. `data-pair-card` opens the card's
    // element and the markup ends when it closes; a picker rendered as a sibling
    // would land after that closing tag and this offset test would fail.
    const html = cardMarkup({ expansion: picker });

    const cardOpens = html.indexOf("data-pair-card");
    const pickerAt = html.indexOf("data-fact-picker");

    expect(cardOpens, "the card must carry its own marker").toBeGreaterThan(-1);
    expect(pickerAt, "the picker did not render at all").toBeGreaterThan(-1);
    expect(pickerAt).toBeGreaterThan(cardOpens);

    // …and it really is contained, not merely later in the document: everything
    // from the card's opening tag onward is this one card, because nothing else
    // is rendered here.
    expect(html.slice(cardOpens)).toContain("data-fact-picker");
  });

  it("the picker sits BELOW the controls, under the button that opened it", () => {
    // Above the controls it would push the quote off the top of the card and put
    // the list of choices between the evidence and the button — the panel has to
    // read as belonging to the control, which means following it.
    const html = cardMarkup({ expansion: picker });

    expect(html.indexOf("Pair an answer")).toBeLessThan(html.indexOf("data-fact-picker"));
  });

  it("a card that opened nothing renders no picker", () => {
    // The corollary, and the half that keeps ONE picker on screen: every other
    // card in the list passes no expansion. Without this, the fix for "invisible
    // picker" would be "a picker under every card".
    expect(cardMarkup()).not.toContain("data-fact-picker");
  });

  it("the picker's search box is the first control in it", () => {
    // P1 asks for the search box to take focus on open. The focus itself is an
    // effect, which static rendering does not run — what IS checkable, and what
    // the effect depends on, is that the box exists and is reachable first.
    const html = renderToStaticMarkup(picker);
    expect(html).toContain('type="search"');
    expect(html.indexOf('type="search"')).toBeLessThan(html.indexOf("Cancel"));
  });
});

describe("P2 · a Q/A statement renders its question above its answer", () => {
  const asked = "Did George Phillips make the argument that Marie ran up the bill?";

  it("the question renders ABOVE the quote it belongs to", () => {
    // S-6's own card: the quote is the single word "Yes." Without the question
    // above it, the card is a syllable — the answer with the question that gives
    // it meaning stripped off.
    const html = cardMarkup({ card: cardFixture({ quote: "Yes.", question: asked }) });

    expect(html).toContain(asked);
    expect(html.indexOf(asked)).toBeLessThan(html.indexOf("Yes."));
  });

  it("the question is clamped to one line and stays secondary to the quote", () => {
    // The evidence is what Marie reads out; the question is what makes it mean
    // something. A four-line interrogatory rendered in full above every answer
    // would push the evidence off a card the design keeps compact.
    const html = cardMarkup({ card: cardFixture({ quote: "Yes.", question: asked }) });

    // One line, and only the question is clamped here: this quote is short
    // enough that `FoldedQuote` adds no clamp of its own, so a `1` in the markup
    // can only have come from the question.
    expect(html).toContain("-webkit-line-clamp:1");
    expect(html).toContain("var(--text-muted)");
  });

  it("the whole question is in the markup, never truncated in the string", () => {
    // The clamp is a BOX, not a cut. A quote or a question shortened in the
    // string would be this page editing the record — the thing every rule on
    // this card exists to prevent.
    const long = `${asked} ${"Identify each such conversation. ".repeat(6)}`.trim();
    const html = cardMarkup({ card: cardFixture({ quote: "Yes.", question: long }) });

    expect(html).toContain(long);
    expect(html).not.toContain("…");
  });

  it("documentary evidence renders NO question line", () => {
    // A court finding answers nobody. An empty question line above one asserts a
    // question exists and was lost.
    expect(cardMarkup({ card: cardFixture({ question: null }) })).not.toContain(
      "data-pair-question",
    );
  });

  it("an EMPTY question is the same absence as a missing one", () => {
    // The extraction writes the property on every discovery item it reads, and
    // an item it could not read the question for gets "". Rendering that would
    // put an empty line where the question belongs.
    expect(cardMarkup({ card: cardFixture({ question: "" }) })).not.toContain(
      "data-pair-question",
    );
  });

  it("OUR ANSWER carries its own question too", () => {
    // The asymmetry this closes: five of the nine pairings on DEV point at a
    // discovery response, and four of those quote a bare affirmation. A question
    // that rendered only on the accusation half would leave OUR side the
    // unreadable one.
    const theirs = cardFixture({ code: "C-12", quote: "Yes.", question: asked });
    const ours = cardFixture({
      code: "C-14",
      quote: "It is my understanding that Mr. Zolton represented her.",
      question: "Identify the time period during which Marie Awad was represented.",
    });

    const html = renderToStaticMarkup(
      <PairCard
        card={{
          ...pairCardFromScenarioCard(theirs, options.card_grammar.speaker_absent_label),
          answer: pairCardFromScenarioCard(ours, options.card_grammar.speaker_absent_label),
        }}
        answerLabel="OUR ANSWER"
        showLabel={null}
        hideLabel={null}
        gapNotice={null}
      />,
    );

    expect(html).toContain(asked);
    expect(html).toContain("Identify the time period during which Marie Awad was represented.");
    // Two question lines, one per half — not one shared line.
    expect(html.split("data-pair-question").length - 1).toBe(2);
  });
});

describe("P3 · the add-fact form renders at the control that opens it", () => {
  /** The facts view, with the form open or closed. */
  const workingMarkup = (addForm: React.ReactNode) =>
    renderToStaticMarkup(
      <WorkingView
        cards={[cardFixture({ status: "included" })]}
        humanFacts={[]}
        onAdd={() => {}}
        addForm={addForm}
        onRemoveHumanFact={() => {}}
        onRemoveFact={() => {}}
        wording={workingWording}
        options={options}
        onSetTier={() => Promise.resolve()}
        onMoveFact={() => {}}
      />,
    );

  const form = (
    <AddHumanFactForm slug="case" scenarioId="s-6" onSaved={() => {}} onCancel={() => {}} />
  );

  it("the form renders immediately after its own button", () => {
    // The defect: the form rendered after `<WorkingView>` — that is, after a
    // scroll region holding forty-six rows — so clicking the button appeared to
    // do nothing at all. Identical in shape to P1's picker.
    const html = workingMarkup(form);

    const button = html.indexOf("+ Add human fact");
    const formAt = html.indexOf("data-add-human-fact");

    expect(button, "the create control is missing").toBeGreaterThan(-1);
    expect(formAt, "the form did not render").toBeGreaterThan(-1);
    expect(formAt).toBeGreaterThan(button);
  });

  it("the form renders BEFORE the facts list, never past it", () => {
    // The precise regression guard: "after the button" is satisfied by "at the
    // very bottom of the page" too. The form has to come before the rows.
    const html = workingMarkup(form);

    expect(html.indexOf("data-add-human-fact")).toBeLessThan(html.indexOf("data-fact-code"));
  });

  it("nothing renders when the form is closed", () => {
    expect(workingMarkup(null)).not.toContain("data-add-human-fact");
  });

  it("the form's own text box is what a human types into first", () => {
    const html = renderToStaticMarkup(form);

    expect(html).toContain('id="human-fact-text"');
    expect(html.indexOf('id="human-fact-text"')).toBeLessThan(html.indexOf("Add fact"));
  });
});

describe("P6 · the facts fold says what it folds", () => {
  it("the fold's visible words ARE its accessible name", () => {
    // It was a bare 30-pixel ▸ beside "Reset order" and read as furniture. The
    // words are now in the button itself rather than in an `aria-label`, so a
    // sighted reader and a screen-reader user get one string that cannot drift.
    const html = renderToStaticMarkup(
      <SectionFold open onToggle={() => {}} names="the scenario facts" />,
    );

    expect(html).toContain("Collapse the scenario facts");
    expect(html, "an aria-label would OVERRIDE the visible words").not.toContain("aria-label");
    // The arrow stays, and stays decorative: a screen reader announcing "▾" is
    // announcing a symbol nobody can say out loud.
    expect(html).toContain('aria-hidden="true"');
  });
});
