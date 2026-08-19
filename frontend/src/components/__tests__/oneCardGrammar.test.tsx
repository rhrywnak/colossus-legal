/**
 * The one-card law, asserted (ONE_CARD_GRAMMAR, approved 2026-08-09).
 *
 * > A candidate and an included fact are the same item at two lifecycle stages,
 * > and they render as the SAME card. Only the wrapper varies.
 *
 * ## What these tests can and cannot reach
 *
 * `renderToStaticMarkup` is a pure function from React elements to an HTML
 * string — no DOM, no RTL, nothing Rule 30 says is not set up here. So the claim
 * "both wrappers show the same fields" is checkable against real output rather
 * than against two lists somebody promised to keep in step.
 *
 * What they cannot reach is interaction: whether the "+N more" control actually
 * expands, whether the sticky bar stays put while scrolling. Those are Roman's
 * DEV verification, and the model tests below cover the DECISIONS behind them.
 */
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { CandidateCard } from "../CandidateCard";
import FactRow from "../FactRow";
import { evidenceCardView, matchesChip } from "../evidenceCardModel";
import { matchAllegations } from "../AllegationTypeahead";
import type { WorkingRow } from "../factsTable";
import { includedRows } from "../factsTable";
import { cardFixture, optionsFixture } from "./cardFixtures";

const options = optionsFixture();
const grammar = options.card_grammar;
const limits = {
  questionChars: options.card_question_truncate_chars,
  elementK: options.card_element_chips_visible_k,
};

/** One fully-dressed card — everything a §7 payload can carry. */
const dressed = () =>
  cardFixture({
    code: "C-73",
    quote: "I would have provided an assessment regarding any delay.",
    question:
      "Did you have any conversations or contact with any of the interested parties in the " +
      "Estate other than the heirs regarding conduct you alleged to attribute to Marie Awad?",
    speaker: "George Phillips",
    statementKind: "sworn discovery answer",
    scanReason: "Phillips admits he told interested parties her actions caused delay.",
    contextBefore: "…the preceding paragraph of the response…",
    contextAfter: "…the following paragraph…",
    bearsOn: [
      {
        accusation: "A-45 — misrepresented the cause of the $30,000 costs",
        elements: [
          "Breach of the fiduciary duty",
          "Improper act in the use of process",
          "Defendant intended plaintiff to act on it",
          "Plaintiff was damaged by the reliance",
        ],
        count: "Count 1 — Breach of Fiduciary Duty",
      },
    ],
  });

const candidateMarkup = (card = dressed()) =>
  renderToStaticMarkup(
    <CandidateCard
      card={card}
      selected
      compact={false}
      onSelect={() => {}}
      onRule={() => {}}
      linkOptions={options}
      onSaveLinks={async () => {}}
      onUnlink={() => {}}
    />,
  );

const factMarkup = (card = dressed()) => {
  // Built through the PRODUCTION projection, not a hand-written row: the claim
  // is that the facts list serves the same payload, and a fixture assembled by
  // hand would prove only that this file can copy fields.
  const [row] = includedRows([{ ...card, status: "included" }]);
  return renderToStaticMarkup(
    <FactRow row={row as WorkingRow} wording={options.wording} options={options} />,
  );
};

describe("a candidate and a fact serve the same card payload fields", () => {
  it("renders every field of the card under BOTH wrappers", () => {
    const card = dressed();
    const view = evidenceCardView(card, grammar, limits);
    const candidate = candidateMarkup(card);
    const fact = factMarkup(card);

    // The fields the design names, one by one. Each is checked on both surfaces
    // in the same assertion, so a regression on either side names itself.
    const fields: [string, string][] = [
      ["the speaker", "George Phillips"],
      ["the extracted tag", grammar.speaker_extracted_label],
      ["the kind", "sworn discovery answer"],
      ["the confidence band", "Model confidence: high"],
      ["the question", view.question?.short ?? ""],
      ["the anchor quote", card.quote.text],
      ["the pinpoint", card.pinpoint.label],
      ["the A- code", "A-45"],
      ["the count chip", "Count 1 — Breach of Fiduciary Duty"],
      ["the first element chip", "Breach of the fiduciary duty"],
      ["the compress control", "+2 more"],
      ["the scan's reason", card.scan_reason ?? ""],
      ["the context control", grammar.context_show_label],
    ];

    for (const [name, text] of fields) {
      expect(candidate, `${name} is missing from the CANDIDATE card`).toContain(text);
      expect(fact, `${name} is missing from the FACT card`).toContain(text);
    }
  });

  it("the anchor quote is highlighted on both wrappers", () => {
    // The C-91 defect: the fact card rendered raw text with no mark, so the one
    // phrase a human had chosen to rest the argument on was indistinguishable
    // from the sentence around it.
    const marked = `<mark style="background:var(--highlight-quote-soft)`;
    expect(candidateMarkup()).toContain(marked);
    expect(factMarkup()).toContain(marked);
  });

  it("the scan's reason survives include", () => {
    // Until ruling R3 the reason lived on the proposal, and precedence law R-a
    // drops every ruled node from the projection — so pressing Include deleted
    // the one sentence saying why the fact mattered. The FACT markup carrying it
    // is the whole of the fix.
    const card = dressed();
    expect(factMarkup(card)).toContain(card.scan_reason);
    expect(factMarkup(card)).toContain(grammar.scan_reason_label);
  });

  it("a card with no reason renders no reason label, rather than an empty one", () => {
    // Absent is a real state — nothing judged this, or the ruling predates the
    // provenance column. A bare "Scan:" with nothing after it would be the card
    // asserting a sentence it does not have.
    const bare = cardFixture({ code: "C-9" });
    expect(bare.scan_reason).toBeUndefined();
    expect(factMarkup(bare)).not.toContain(grammar.scan_reason_label);
  });
});

describe("the question renders collapsed and never leads the card", () => {
  it("folds a long question and keeps the whole of it for the unfold", () => {
    const card = dressed();
    const view = evidenceCardView(card, grammar, limits);

    expect(view.question?.truncated).toBe(true);
    expect(view.question?.short.length).toBeLessThanOrEqual(limits.questionChars + 1);
    expect(view.question?.full).toBe(card.quote.question);
    // The fold is a display decision and omits nothing — the ellipsis is a real
    // character rather than three dots, which in a legal quotation mean text was
    // dropped from the RECORD.
    expect(view.question?.short.endsWith("…")).toBe(true);
  });

  it("leaves a short question whole and offers no control", () => {
    const view = evidenceCardView(
      cardFixture({ question: "Did you attend?" }),
      grammar,
      limits,
    );
    expect(view.question?.truncated).toBe(false);
    expect(view.question?.short).toBe("Did you attend?");
  });

  it("puts the SPEAKER before the question on the card", () => {
    // Half of ruling R2, and the half layout can fix: the card answers "who said
    // this" before it says anything else. It did not — the question led, with the
    // transcription badge under it, which is what Roman read as the speaker.
    for (const html of [candidateMarkup(), factMarkup()]) {
      const speaker = html.indexOf("George Phillips");
      const question = html.indexOf("Q:");
      expect(speaker).toBeGreaterThan(-1);
      expect(question).toBeGreaterThan(-1);
      expect(speaker, "the speaker chip must lead the question").toBeLessThan(question);
    }
  });
});

describe("system never renders as a speaker", () => {
  it("the authorship badge names the question and never occupies the speaker slot", () => {
    // Re-scoped per ruling R2. Measured on the DEV graph 2026-08-09: nine
    // STATED_BY actor names and none of them "System", so the speaker chip never
    // said it and could not. What Roman read was the badge saying who
    // TRANSCRIBED the question, sitting under seven lines of interrogatory.
    const view = evidenceCardView(dressed(), grammar, limits);

    expect(view.question?.authorship?.label).toBe(
      grammar.question_machine_authorship_label,
    );
    expect(view.question?.authorship?.label).not.toBe("System");
    // The speaker slot is the CHIPS, and the badge is not among them.
    expect(view.chips.map((c) => c.text)).not.toContain(
      grammar.question_machine_authorship_label,
    );
  });

  it("an item with no speaker says so rather than borrowing a name", () => {
    // Documentary evidence genuinely has no speaker. Silence there would be
    // indistinguishable from a chip that failed to render.
    const view = evidenceCardView(cardFixture({ speaker: null }), grammar, limits);
    expect(view.chips[0].text).toBe(grammar.speaker_absent_label);
    expect(view.chips[0].filter, "an absent speaker is not a filter").toBeNull();
  });
});

describe("allegations display as A- codes on touched surfaces", () => {
  it("carries the complaint's own number behind a typeable prefix", () => {
    // A-45 IS complaint ¶45 — renamed, never renumbered. The prefix changed
    // because the type-ahead that replaced the 120-checkbox wall needs one a
    // human can produce, and nobody can type the pilcrow.
    for (const html of [candidateMarkup(), factMarkup()]) {
      expect(html).toContain("A-45");
      expect(html, "the pilcrow has left every surface this task touches").not.toContain(
        "¶",
      );
    }
  });
});

describe("element chips compress beyond K and expand on demand", () => {
  it("stands the first K and folds the rest behind a counted control", () => {
    const view = evidenceCardView(dressed(), grammar, limits);
    const [bears] = view.bearsOn;

    expect(bears.visibleElements).toHaveLength(limits.elementK);
    expect(bears.hiddenElements).toHaveLength(2);
    // The fold says how much is folded. A control reading "+more" is one nobody
    // opens, because it promises nothing.
    expect(bears.moreLabel).toBe("+2 more");
  });

  it("offers no control when nothing folds", () => {
    const view = evidenceCardView(
      cardFixture({
        bearsOn: [{ accusation: "A-41 — a thing", elements: ["Duty"], count: null }],
      }),
      grammar,
      limits,
    );
    expect(view.bearsOn[0].moreLabel).toBeNull();
  });

  it("never DROPS an element, however many there are", () => {
    // Roman's ruling: element chips COMPRESS, never vanish — they are the quick
    // "what harm was done" indicator, and losing them is what left the fact card
    // unable to say anything about damages.
    const elements = Array.from({ length: 9 }, (_, i) => `Element ${i}`);
    const view = evidenceCardView(
      cardFixture({ bearsOn: [{ accusation: "A-1 — x", elements, count: null }] }),
      grammar,
      limits,
    );
    const [bears] = view.bearsOn;
    expect([...bears.visibleElements, ...bears.hiddenElements]).toEqual(elements);
  });
});

describe("chips are cross-reference currency (Piece 7)", () => {
  it("filters on the RAW payload value, not on the chip's displayed text", () => {
    // The two differ the moment a chip carries a provenance tag — and matching on
    // prose would break the day the wording changed, which is the same argument
    // `candidateState` makes for reading `status` over `status_label`.
    const card = dressed();
    const view = evidenceCardView(card, grammar, limits);
    const speakerChip = view.chips[0];

    expect(speakerChip.filter).toEqual({ kind: "speaker", value: "George Phillips" });
    expect(matchesChip(card, speakerChip.filter)).toBe(true);
    expect(matchesChip(cardFixture({ speaker: "Marie Awad" }), speakerChip.filter)).toBe(
      false,
    );
  });

  it("treats a null filter as no narrowing at all", () => {
    // "No filter" and "a filter that matches nothing" are different states, and
    // the absence of a chip means the former.
    expect(matchesChip(cardFixture({ speaker: null }), null)).toBe(true);
  });

  it("makes the band and the grounding warning labels, not filters", () => {
    // A band is a reading of one card's score; narrowing a list to "high" would
    // be a facet, and facets are the chips at the top of the queue. Only the two
    // chips that name a THING in the case are cross-references.
    const view = evidenceCardView(dressed(), grammar, limits);
    const filterable = view.chips.filter((c) => c.filter !== null).map((c) => c.text);
    expect(filterable).toEqual(["George Phillips", "sworn discovery answer"]);
  });
});

describe("linking a locked card wakes ruling and acknowledges", () => {
  it("ranks the scenario's own accusations ahead of the rest of the complaint", () => {
    // The short list is "what this scenario is about" and the payload keeps it
    // apart from the rest for that reason. A human typing "divide" should meet
    // their own anchor before the 90th paragraph of the complaint.
    const withOptions = {
      ...options,
      serving: [
        {
          allegation_id: "a-41",
          label: "A-41 — justified the auction",
          count_label: "Count 2 — Fraud",
          filter_text: "a-41 — justified the auction count 2 — fraud",
        },
      ],
      others: [
        {
          allegation_id: "a-90",
          label: "A-90 — an unrelated auction claim",
          count_label: null,
          filter_text: "a-90 — an unrelated auction claim",
        },
      ],
    };

    const hits = matchAllegations(withOptions, "auction", 6);
    expect(hits.map((h) => h.allegation_id)).toEqual(["a-41", "a-90"]);
  });

  it("matches on the haystack the BACKEND composed, never on invented text", () => {
    // `filter_text` is lowercased server-side from the label and the count. A
    // match found any other way could surface a row for a reason the human
    // cannot see on screen.
    const withOptions = {
      ...options,
      others: [
        {
          allegation_id: "a-41",
          label: "A-41 — justified the auction",
          count_label: "Count 2 — Fraud",
          filter_text: "a-41 — justified the auction count 2 — fraud",
        },
      ],
    };

    expect(matchAllegations(withOptions, "fraud", 6)).toHaveLength(1);
    expect(matchAllegations(withOptions, "A-41", 6)).toHaveLength(1);
    expect(matchAllegations(withOptions, "conspiracy", 6)).toHaveLength(0);
  });

  it("offers the whole list before anything is typed", () => {
    // An empty box means "no filter", not "a filter that matches nothing" — the
    // same distinction the facts search box makes.
    const withOptions = {
      ...options,
      serving: [
        {
          allegation_id: "a-41",
          label: "A-41 — a thing",
          count_label: null,
          filter_text: "a-41 — a thing",
        },
      ],
    };
    expect(matchAllegations(withOptions, "   ", 6)).toHaveLength(1);
  });

  it("a locked card renders the type-ahead and its condition on its own face", () => {
    // Piece 4b. The 120-checkbox wall it replaces was a scroll region on a card
    // that could not be ruled until something in it was ticked.
    const locked = {
      ...cardFixture({ code: "C-110" }),
      defer_required: true,
      defer_required_reason: "Nothing links this statement to an accusation.",
      stance: null,
    };
    const html = candidateMarkup(locked);

    expect(html).toContain(grammar.link_typeahead_intro);
    expect(html).toContain(grammar.link_typeahead_placeholder);
    // …and the ruling controls are visibly SHUT rather than absent, with the
    // stored label introducing why.
    expect(html).toContain(options.wording.card_locked_condition_label);
    expect(html).toContain("not-allowed");
  });
});
