/**
 * The v2.1 fact card: evidence only, a Q&A proof, and the Backs picker.
 *
 * Split from `factCardAnatomy.test.tsx`, which was over the 300-line limit with
 * these suites in it (CLAUDE.md rule 17). The split is also honest about what is
 * being tested: that file renders `FactRow` WITHOUT a scenario scope, which
 * falls through to the curator's `EvidenceCardBody`. Everything here needs the
 * witness card, and the helper below is what selects it.
 *
 * Same standing as its sibling on Rule 30: nothing mounts a component, fires an
 * event or reads a layout. `renderToStaticMarkup` is a pure function from React
 * elements to an HTML string and needs no DOM.
 */
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import FactRow from "../FactRow";
import type { WorkingRow } from "../factsTable";
import type { ScenarioCard } from "../../services/scenarioCards";
import type { TalkingPointDto } from "../../services/scenarioAugmentation";
import { cardFixture, optionsFixture } from "./cardFixtures";

const options = optionsFixture();
const wording = options.wording;

function row(over: Partial<WorkingRow> = {}): WorkingRow {
  return {
    code: "C-1",
    graphNodeId: "ev-1",
    text: "Yes.",
    bearsOn: [],
    pinpointLabel: "CFS responses at 26",
    pinpointHref: "/documents/doc-7?page=26",
    statusLabel: "In the scenario",
    isHuman: false,
    card: cardFixture({ code: "C-1" }),
    question: null,
    statementKind: null,
    tier: "backup",
    sortOrdinal: null,
    speaker: null,
    displayOrdinal: null,
    ...over,
  };
}

// ── v2.1 (ruling R37): the card is EVIDENCE ONLY ─────────────────────────────
//
// ⚑ These render `FactCardBody`, which the sibling file's `markup` helper does
// NOT: that helper passes no `slug` / `scenarioId`, and `FactRow` falls through
// to the curator's `EvidenceCardBody` without them (there is no scenario to
// write a card against). Every assertion below would have passed vacuously
// against the wrong component — the first draft of this suite did, which is why
// the two helpers are separate files and named for what they mount.

/** The scenario a card edit is written against. Presence is what selects the wrapper. */
const CARD_SCOPE = { slug: "awad-v-cfs", scenarioId: "s-7" };

/** `FactRow` mounting the WITNESS card. Optionally with the Backs picker on. */
const factCardMarkup = (r: WorkingRow, points?: TalkingPointDto[]) =>
  renderToStaticMarkup(
    <FactRow
      row={r}
      wording={wording}
      options={options}
      onSetTier={() => {}}
      points={points}
      {...CARD_SCOPE}
    />,
  );

/**
 * A card block, which `cardFixture` deliberately does not build.
 *
 * The fixture models the §7 PAYLOAD — quote, pinpoint, speaker. The authored
 * block is a different thing (`FactCardBlock`, written by the extraction and
 * edited by a human), and the rows below are the only place its fields would ever
 * reach a screen, so it is built here rather than pushed into a fixture six other
 * suites read.
 */
function withBlock(
  blockOver: Partial<NonNullable<ScenarioCard["card"]>> = {},
  cardOver: Parameters<typeof cardFixture>[0] = {},
): ScenarioCard {
  const base = cardFixture(cardOver);
  return {
    ...base,
    card: {
      title: "They admitted I was the only heir asked to pay",
      backs: "Point 1 — A judge ordered that money put back.",
      backs_position: 1,
      supports: ["Supports A-21 — CFS could have returned it."],
      supports_refs: [],
      watch_out: "They will say the judge approved it.",
      answer: "The money was Dad's.",
      drafts: {
        title: false,
        backs: false,
        supports: false,
        watch_out: false,
        answer: false,
      },
      count_tags: ["Count 1 — Breach of Fiduciary Duty"],
      ...blockOver,
    },
  };
}

describe("the machine's prose left the card (v2.1)", () => {
  it("renders PROOF and SUPPORTS, and neither WATCH OUT nor ANSWER nor BACKS prose", () => {
    // All four labels are still stored rows and all four are still served — the
    // ruling removed the ROWS, not the vocabulary — so asserting on the words is
    // asserting on exactly what a reader would see.
    const html = factCardMarkup(row({ card: withBlock() }));

    expect(html).toContain(options.fact_card.proof_label);
    expect(html).toContain(options.fact_card.supports_label);
    expect(html).toContain("Supports A-21 — CFS could have returned it.");

    expect(html).not.toContain(options.fact_card.watch_out_label);
    expect(html).not.toContain(options.fact_card.answer_label);
    // The composed sentences themselves, not just their headings: a row could be
    // re-added tomorrow under a label read from somewhere else.
    expect(html).not.toContain("They will say the judge approved it.");
    expect(html).not.toContain("The money was Dad's.");
    expect(html).not.toContain("Point 1 — A judge ordered that money put back.");
  });

  it("prints no sort ordinal in the title bar", () => {
    // The `20480`-style number. `display_ordinal` is a sparse sort key and this
    // list is not read in sequence, so the number meant nothing to anybody. It
    // still ORDERS the list; it is simply not narrated.
    const html = factCardMarkup(row({ card: withBlock(), displayOrdinal: 20480 }));
    expect(html).not.toContain("20480");
  });
});

describe("PROOF is a Q&A pair when the record has one (v2.1)", () => {
  it("renders Q: then A: on a discovery answer", () => {
    const question = "Admit that Marie was the only heir asked to pay costs.";
    const html = factCardMarkup(
      row({ card: withBlock({}, { question, quote: "Admitted." }) }),
    );

    const q = html.indexOf("Q:");
    const a = html.indexOf("A:");
    expect(q, "the question must render — it is half the evidence").toBeGreaterThan(-1);
    expect(a).toBeGreaterThan(-1);
    expect(q, "the question leads, the answer is the star").toBeLessThan(a);
    expect(html).toContain(question);
    expect(html).toContain("Admitted.");
  });

  it("renders ONE line, with no Q:/A: prefixes, when there is no question", () => {
    // `null` means "this statement is not a Q&A pair at all" — transcript
    // speech, a letter. An `A:` with nothing answering it would be the card
    // asserting a structure the record does not have.
    const html = factCardMarkup(
      row({ card: withBlock({}, { question: null, quote: "Yes." }) }),
    );
    expect(html).not.toContain("Q:");
    expect(html).not.toContain("A:");
    expect(html).toContain("Yes.");
  });

  it("folds a long question at the SERVED threshold, with the stored control", () => {
    // The same three values `EvidenceCardBody` folds by — one piece of evidence,
    // one fold, whichever wrapper is around it.
    const question = "A".repeat(400);
    const html = factCardMarkup(row({ card: withBlock({}, { question }) }));
    expect(html).not.toContain(question);
    expect(html).toContain("…");
    expect(html).toContain(options.card_grammar.question_expand_label);
  });
});

describe("the Backs picker, and where it is NOT offered (v2.1)", () => {
  const points: TalkingPointDto[] = [
    {
      position: 1,
      text: "When my sister's side did the same thing, nothing happened.",
      authored_tag: "Added by roman",
    },
    {
      position: 2,
      text: "They told the court things they later admitted were untrue.",
      authored_tag: null,
    },
  ];

  it("offers one option per LIVE talking point, plus the stored em dash", () => {
    // The defect the picker replaces: `backs_position` was a free text box, so a
    // human could type 7 on a scenario with two points. The write was accepted
    // and the composed sentence came back null — an em dash with nothing saying
    // why. An unoccupied position cannot be chosen at all now.
    const html = factCardMarkup(row({ card: withBlock() }), points);
    expect(html).toContain("<select");
    expect(html).toContain('value="1"');
    expect(html).toContain('value="2"');
    expect(html).not.toContain('value="3"');
    expect(html, "clearing wears the card's own empty vocabulary").toContain(
      options.fact_card.empty_value,
    );
    expect(html, "the picker wears the stored BACKS label").toContain(
      options.fact_card.backs_label,
    );
  });

  it("is ABSENT on a card rendered under a talking point", () => {
    // No `points` prop ⇒ no picker. That is the second surface this card renders
    // on: under the point it already backs, where a control offering to move it
    // out from under that heading would be clicked by accident.
    expect(factCardMarkup(row({ card: withBlock() }))).not.toContain("<select");
  });

  it("is absent when the scenario has no points and the card names none", () => {
    const html = factCardMarkup(row({ card: withBlock({ backs_position: null }) }), []);
    expect(html).not.toContain("<select");
  });

  it("still renders for a STALE pointer, so it can be cleared", () => {
    // A card pointing at position 4 on a scenario whose points were deleted. The
    // pointer is real and stored; withholding the control would leave it
    // unclearable, which is the original defect in a different hat.
    const html = factCardMarkup(row({ card: withBlock({ backs_position: 4 }) }), []);
    expect(html).toContain("<select");
  });
});
