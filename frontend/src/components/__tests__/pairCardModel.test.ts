// =============================================================================
// pairCardModel.test.ts — the shared pair card's rules, as data (task R4, P3)
// =============================================================================
//
// The card renders on two pages from two very different payloads, and every
// decision it makes — which words are highlighted, whether a quote earns a fold,
// what a missing speaker reads as — is made in the model so it can be asserted
// without a DOM (CLAUDE.md rule 30).
//
// The property worth protecting hardest is the LAST describe below: the two
// adapters must produce the same shape, or "one card on both pages" is a claim
// the code does not keep.

import { describe, expect, it } from "vitest";

import {
  FOLD_THRESHOLD_CHARS,
  HIGHLIGHT_ANCHOR_QUOTE,
  foldQuote,
  pairCardFromRehearsalAnswer,
  pairCardFromRehearsalInstance,
  pairCardFromScenarioCard,
} from "../pairCardModel";
import type { RehearsalAnswer, RehearsalInstance } from "../../services/rehearsal";
import type { ScenarioCard } from "../../services/scenarioCards";

const WHO_UNRECORDED = "Speaker not recorded";

/** A working-page card with everything the pair card reads. */
function card(overrides: Partial<ScenarioCard> = {}): ScenarioCard {
  return {
    code: "C-91",
    graph_node_id: "ev-1",
    quote: {
      text: "the parties did not cooperate",
      context_before: "Q. What happened next? A. In my view ",
      context_after: " and that is why the plan failed.",
      context_before_complete: true,
      context_after_complete: true,
      context_before_notice: null,
      context_after_notice: null,
      question: null,
    },
    pinpoint: {
      document_id: "doc-7",
      document_title: "Hearing to approve plan",
      label: "Hearing to approve plan, p. 24",
      page: 24,
      viewer_href: "/viewer/doc-7?page=24",
    },
    speaker: { name: "George Phillips", attribution: "extracted" },
    statement_kind: "Testimony",
    stance: null,
    bears_on: [],
    grounding: null,
    confidence: { band: "unscored", label: "Not scored by a scan" },
    status: "included",
    status_label: "Included",
    defer_required: false,
    defer_required_reason: null,
    defer_reason: null,
    human_links: [],
    human_link_summary: null,
    ...overrides,
  } as ScenarioCard;
}

function instance(overrides: Partial<RehearsalInstance> = {}): RehearsalInstance {
  return {
    position: 1,
    code: "C-91",
    who: "George Phillips",
    when: "December 2009",
    when_gap: null,
    source: {
      label: "Hearing to approve plan, p. 24",
      href: "/viewer/doc-7?page=24",
      open_label: "Open",
    },
    kind_label: "Testimony",
    quote: "the parties did not cooperate",
    question: null,
    quote_first_line: "the parties did not cooperate",
    answer: null,
    answer_tag: "NO ANSWER",
    answer_banner: "No answer prepared",
    phase: "Probate",
    ...overrides,
  };
}

function answer(overrides: Partial<RehearsalAnswer> = {}): RehearsalAnswer {
  return {
    code: "C-14",
    who: "Marie Awad",
    when: null,
    when_gap: "No date yet",
    source: { label: "My certified letter, p. 2", href: "", open_label: "Open" },
    quote: "I am open to dividing the property.",
    question: null,
    ...overrides,
  };
}

describe("the highlight", () => {
  it("falls back to the anchor quote, and SAYS that is what it did", () => {
    // There is no stored marked line in this build — see the module header. The
    // source field is what makes the fallback deliberate rather than invisible,
    // and this asserts today's answer so the day it changes is a visible one.
    expect(foldQuote("the operative words").source).toBe(HIGHLIGHT_ANCHOR_QUOTE);
  });

  it("is the anchor, with the page's context either side of it", () => {
    const built = pairCardFromScenarioCard(card(), WHO_UNRECORDED);

    expect(built.quote.highlight).toBe("the parties did not cooperate");
    expect(built.quote.before).toBe("Q. What happened next? A. In my view ");
    expect(built.quote.after).toBe(" and that is why the plan failed.");
  });

  it("spans the whole quote when there is no context to dim", () => {
    // The prep payload carries the verbatim statement and nothing either side.
    const built = pairCardFromRehearsalInstance(instance());

    expect(built.quote.highlight).toBe("the parties did not cooperate");
    expect(built.quote.before).toBe("");
    expect(built.quote.after).toBe("");
  });
});

describe("the fold", () => {
  it("a short quote earns no fold control", () => {
    expect(foldQuote("Three words here").needsFold).toBe(false);
  });

  it("C-91's ramble folds — the whole point of the card", () => {
    // The measured case the design names: a long statement should read as one
    // highlighted phrase at a glance rather than twelve lines to hunt through.
    const ramble = "x".repeat(FOLD_THRESHOLD_CHARS + 1);
    expect(foldQuote(ramble).needsFold).toBe(true);
  });

  it("counts the CONTEXT toward the length, not just the anchor", () => {
    // A two-word anchor inside a page of context is exactly the card that needs
    // folding most, and a threshold that looked only at the highlight would
    // decide it did not.
    const built = foldQuote("no", "x".repeat(FOLD_THRESHOLD_CHARS), "y");
    expect(built.needsFold).toBe(true);
  });

  it("the fold is decided on the data, never on a line count", () => {
    // A line is a property of the rendered box, which this model cannot see —
    // the visual clamp is CSS. Asserted as a boundary rather than a philosophy:
    // exactly at the threshold is not yet long enough.
    expect(foldQuote("x".repeat(FOLD_THRESHOLD_CHARS)).needsFold).toBe(false);
  });
});

describe("provenance", () => {
  it("the working card carries who, kind, source and its code", () => {
    const built = pairCardFromScenarioCard(card(), WHO_UNRECORDED);

    expect(built.provenance.who).toBe("George Phillips");
    expect(built.provenance.kindLabel).toBe("Testimony");
    expect(built.provenance.sourceLabel).toBe("Hearing to approve plan, p. 24");
    expect(built.provenance.code).toBe("C-91");
  });

  it("a statement with no recorded speaker reads as the SERVED sentence", () => {
    // Measured on S-2: one of forty-six. It must not render as a blank, which
    // looks like a card that failed, and must not be composed here.
    const built = pairCardFromScenarioCard(
      card({ speaker: { name: null, attribution: "extracted" } }),
      WHO_UNRECORDED,
    );

    expect(built.provenance.who).toBe(WHO_UNRECORDED);
  });

  it("the source link is the SERVED href — the browser builds no URLs", () => {
    const built = pairCardFromScenarioCard(card(), WHO_UNRECORDED);
    expect(built.provenance.sourceHref).toBe("/viewer/doc-7?page=24");
  });

  it("no link at all when the record cannot say which document", () => {
    // A link to nowhere is worse than no link: a reader clicks it in front of
    // opposing counsel.
    const built = pairCardFromRehearsalAnswer(answer());
    expect(built.provenance.sourceHref).toBeNull();
    expect(built.provenance.sourceLabel).toBe("My certified letter, p. 2");
  });

  it("the working card states no date rather than assembling one", () => {
    // The cards payload carries no composed date, and a date is a claim about
    // precision that the backend owns. Absent is honest here.
    expect(pairCardFromScenarioCard(card(), WHO_UNRECORDED).provenance.when).toBeNull();
  });

  it("the prep card shows the date, or the served gap — exactly one", () => {
    expect(pairCardFromRehearsalInstance(instance()).provenance.when).toBe("December 2009");
    expect(
      pairCardFromRehearsalInstance(instance({ when: null, when_gap: "No date yet" })).provenance
        .when,
    ).toBe("No date yet");
  });
});

describe("the C-code reaches BOTH pages", () => {
  // The complaint this closes: "no C-code on rehearsal". Codes are speakable
  // handles, not internal vocabulary — the two pages must call one statement one
  // thing.
  it("the working card carries it", () => {
    expect(pairCardFromScenarioCard(card(), WHO_UNRECORDED).provenance.code).toBe("C-91");
  });

  it("the prep card carries it", () => {
    expect(pairCardFromRehearsalInstance(instance()).provenance.code).toBe("C-91");
  });

  it("our answer carries its own", () => {
    expect(pairCardFromRehearsalAnswer(answer()).provenance.code).toBe("C-14");
  });

  it("an unnumbered candidate carries none rather than its node id", () => {
    // An id in a slot labelled "code" reads as a handle and gets quoted as one
    // out loud, which is worse than a blank.
    expect(pairCardFromScenarioCard(card({ code: null }), WHO_UNRECORDED).provenance.code).toBeNull();
    expect(pairCardFromRehearsalInstance(instance({ code: null })).provenance.code).toBeNull();
  });
});

describe("one card, two pages", () => {
  it("both adapters fill the same slots", () => {
    // THE SHARED-SHAPE TEST. If an adapter stops filling a field the other one
    // fills, the two pages have quietly diverged again — which is the defect P3
    // exists to end. Compares KEYS, so a new field must be handled by both.
    const fromWorking = pairCardFromScenarioCard(card(), WHO_UNRECORDED);
    const fromPrep = pairCardFromRehearsalInstance(instance());

    expect(Object.keys(fromWorking.provenance).sort()).toEqual(
      Object.keys(fromPrep.provenance).sort(),
    );
    expect(Object.keys(fromWorking.quote).sort()).toEqual(Object.keys(fromPrep.quote).sort());
  });

  it("all THREE adapters fill the same slots — the answer half included", () => {
    // The test above compared the two INSTANCE adapters and let the answer
    // adapter drift. That is exactly where task 394's P2 defect lived: the
    // question reached the accusation half and not the answer, and five of the
    // nine pairings on DEV point at a discovery response.
    const shapes = [
      pairCardFromScenarioCard(card(), WHO_UNRECORDED),
      pairCardFromRehearsalInstance(instance()),
      pairCardFromRehearsalAnswer(answer()),
    ].map((side) => Object.keys(side).sort());

    expect(shapes[1]).toEqual(shapes[0]);
    expect(shapes[2]).toEqual(shapes[0]);
  });
});

/**
 * A working-page card whose QUOTE carries a question.
 *
 * The question hangs off `quote` on this payload and off the row itself on the
 * rehearsal one — which is precisely why the adapters exist, and why the shape
 * test above compares their OUTPUT rather than their inputs.
 */
function asked(question: string | null): ScenarioCard {
  const base = card();
  return { ...base, quote: { ...base.quote, question } };
}

describe("the question a statement answers (task 394, P2)", () => {
  it("reaches the card from all three payloads", () => {
    // The working page has carried the question since 2.13; the rehearsal
    // payload gained it in 394. Both have to arrive at the same slot, or the
    // two pages call the same statement two different things again.
    expect(
      pairCardFromScenarioCard(asked("Did he make the argument?"), WHO_UNRECORDED).question,
    ).toBe("Did he make the argument?");

    expect(
      pairCardFromRehearsalInstance(instance({ question: "Did he make the argument?" })).question,
    ).toBe("Did he make the argument?");

    expect(
      pairCardFromRehearsalAnswer(answer({ question: "Identify the time period." })).question,
    ).toBe("Identify the time period.");
  });

  it("documentary evidence carries none", () => {
    // A court finding answers nobody. `null`, not an empty string — the card
    // branches on presence, and "" would render an empty line where the question
    // belongs.
    expect(pairCardFromScenarioCard(asked(null), WHO_UNRECORDED).question).toBeNull();
    expect(pairCardFromRehearsalInstance(instance({ question: null })).question).toBeNull();
    expect(pairCardFromRehearsalAnswer(answer({ question: null })).question).toBeNull();
  });

  it("an EMPTY question is folded into the same absence", () => {
    // The extraction writes the property on every discovery item it reads, and
    // an item whose question it could not read gets "". Both mean "there is no
    // question to show", and there is no different act a human could take for
    // the two — the same rule `factsTable` applies to this field.
    expect(pairCardFromScenarioCard(asked(""), WHO_UNRECORDED).question).toBeNull();
    expect(pairCardFromRehearsalInstance(instance({ question: "" })).question).toBeNull();
    expect(pairCardFromRehearsalAnswer(answer({ question: "" })).question).toBeNull();
  });
});
