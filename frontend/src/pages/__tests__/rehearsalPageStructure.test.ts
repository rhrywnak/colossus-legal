// =============================================================================
// rehearsalPageStructure.test.ts — the anti-regression net (task R4, P8)
// =============================================================================
//
// ## Why this file exists, in one sentence
//
// P1a shipped: the identity block rendered four unlabelled paragraphs for a
// whole build, nothing in the test suite noticed, and a human reading the page
// found it. This class of test is the answer — the named sections are named, the
// named controls exist, and no surface renders a wording key nothing serves.
//
// ## Why these are SOURCE scans and not DOM tests
//
// The batch asked for DOM smoke tests. This repo has no jsdom, no happy-dom and
// no `@testing-library/*`, by the deliberate convention CLAUDE.md rule 30
// records, and standing up that infrastructure on the evening of a deploy is not
// a thing to do quietly inside a defect fix.
//
// So these follow the established pattern of `scenarioPageStructure.test.ts`:
// read the component source and assert structural facts about it. That is a
// weaker instrument than a render in general — but for THIS bug family it is the
// stronger one. P1a was a wire-contract gap between a Rust DTO and a TypeScript
// type; a DOM test of the frontend in isolation, with a fixture the test author
// wrote, would have rendered four perfect labels and passed. The backend parity
// test in `dto/scenario_authoring_wording_tests.rs` is what actually catches it,
// and these are its frontend half.
//
// The limit is worth stating plainly rather than discovering later: a source
// scan proves a component READS a field and RENDERS a heading. It cannot prove
// the result is legible, positioned, or on screen. Roman's walk is still the
// thing that knows that.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, it } from "vitest";

const COMPONENTS = join(__dirname, "..", "..", "components");
const read = (dir: string, file: string) => readFileSync(join(dir, file), "utf8");

describe("the prep page renders its named blocks", () => {
  it("every section heading comes from a stored row", () => {
    // The four blocks the prep page is made of. A heading that became a literal
    // would be a word Roman cannot change from Settings, on the page whose whole
    // law is that every visible word is a stored row.
    const blocks = read(COMPONENTS, "RehearsalScenarioBlocks.tsx");

    expect(blocks).toContain("wording.block_accusation_heading");
    expect(blocks).toContain("wording.block_points_heading");
    expect(blocks).toContain("wording.block_watch_heading");
  });

  it("an empty block says the STORED sentence, never nothing", () => {
    // Absent is absent, and it says so. A block that rendered an empty list would
    // read as a rendering fault to the person least able to diagnose one.
    const blocks = read(COMPONENTS, "RehearsalScenarioBlocks.tsx");

    expect(blocks).toContain("scenario.accusation.no_instances_notice");
    expect(blocks).toContain("scenario.points_gap");
    expect(blocks).toContain("scenario.watch_for_gap");
  });
});

describe("the pair card is ONE card on two pages (task R4, P3)", () => {
  it("both pages render the same component", () => {
    // The complaint this closes: the working page showed a code and a line of
    // text while the prep page showed a full card, so the human doing the marking
    // could not see what the human rehearsing would read. Two files importing one
    // component is the structural form of "the rehearsal rendering wins".
    expect(read(COMPONENTS, "AccusationSection.tsx")).toContain('from "./PairCard"');
    expect(read(COMPONENTS, "PrepInstanceCard.tsx")).toContain('from "./PairCard"');
  });

  it("the prep page passes NO controls", () => {
    // Marie reads this page in front of opposing counsel. A control she cannot
    // use costs her a glance; a control she CAN use is a write from a room where
    // nobody is checking. The adapter must not pass one.
    const prep = read(COMPONENTS, "PrepInstanceCard.tsx");

    expect(prep).not.toContain("controls=");
    expect(prep).not.toContain("onClick");
  });

  it("the working page passes controls, and they are the five named ones", () => {
    const working = read(COMPONENTS, "AccusationSection.tsx");

    expect(working).toContain("controls=");
    // Named through their stored labels, so a control that lost its words fails
    // here rather than rendering an empty button.
    for (const label of ["w.pair_label", "w.repair_label", "w.unpair_label", "w.unmark_label"]) {
      expect(working, `the ${label} control is missing`).toContain(label);
    }
    expect(working).toContain("w.mark_label");
  });

  it("the C-code reaches BOTH pages — it is a speakable handle", () => {
    // "no C-code on rehearsal" was a named complaint. The code is what Marie and
    // Chuck call one statement in a room, and withholding it meant the two pages
    // named the same statement differently.
    expect(read(COMPONENTS, "PairCard.tsx")).toContain("of.code");
    expect(read(COMPONENTS, "pairCardModel.ts")).toContain("instance.code");
  });

  it("the pair picker is wired to its ANCHOR, not to the list (task 394, P1)", () => {
    // The markup half of this claim is `pairPanelPlacement.test.tsx`, which
    // proves a `PairCard` renders its expansion inside itself. This is the other
    // half: that the section only ever hands the expansion to the card the
    // picker was opened FROM. Both are needed — the mechanism could be right
    // while the wiring pointed at every card, or at none.
    const working = read(COMPONENTS, "AccusationSection.tsx");

    expect(working).toContain("expansion=");
    expect(working).toMatch(/picker\?\.mode === "pair" && picker\.anchor === instance\./);
  });

  it("the mark picker stays at the Mark control (task 394, P1)", () => {
    // Explicitly ruled: mark mode was never the defect, and moving a working
    // control to match a broken one's fix is change for symmetry's sake.
    const working = read(COMPONENTS, "AccusationSection.tsx");
    expect(working).toContain('picker?.mode === "mark" && pickerFor(picker)');
  });

  it("the eligibility predicate is UNCHANGED — reuse stays permitted", () => {
    // S-5 depends on reuse: C-14 answers two different instances, so an
    // already-paired exclusion would break a READY, demo-facing scenario. And
    // C-74 must stay offerable, so no stance filter either. Pinned as an absence
    // because the tempting "improvement" is to add one.
    const facts = read(COMPONENTS, "accusationFacts.ts");
    const working = read(COMPONENTS, "AccusationSection.tsx");

    expect(facts).toContain('card.status === "included"');
    // Matched as FIELD ACCESSES rather than as bare words: "stance" is a
    // substring of "instances", which this file's own header says four times.
    // A test that fails on prose is a test somebody deletes.
    for (const forbidden of [/card\.stance/, /answers_graph_node_id/, /alreadyPaired/]) {
      expect(
        facts,
        `the picker predicate must not filter on ${forbidden.source}`,
      ).not.toMatch(forbidden);
    }
    // The only exclusion pairing applies is the anchor itself — a statement
    // cannot be its own answer.
    expect(working).toContain("[mode.anchor]");
  });

  it("a discovery answer's question reaches BOTH halves of the card (P2)", () => {
    // A bare "Yes." is a syllable, not evidence. The question renders through
    // one leaf so the answer half cannot quietly lose a field the accusation
    // half has — which is the asymmetry that produced the defect.
    const card = read(COMPONENTS, "PairCard.tsx");

    expect(card).toContain("side.question");
    expect(card.match(/<SideQuote/g) ?? []).toHaveLength(2);
    // …and it arrives from the payload rather than being composed here.
    expect(read(COMPONENTS, "pairCardModel.ts")).toContain("instance.question");
    expect(read(COMPONENTS, "pairCardModel.ts")).toContain("answer.question");
  });

  it("the add-fact form renders inside the view that owns its button (P3)", () => {
    // It used to render after `<WorkingView>` — that is, past a scroll region
    // holding forty-six rows — so the control read as dead. The state stays with
    // the section that owns the write; only the placement moved.
    const facts = read(COMPONENTS, "ScenarioFactsSection.tsx");
    const view = read(COMPONENTS, "WorkingView.tsx");

    expect(facts).toContain("addForm={");
    expect(facts).not.toMatch(/\{open && adding &&/);
    expect(view).toContain("{addForm && ");
  });

  it("the facts fold carries WORDS, not a bare arrow (P4/P6)", () => {
    // A 30-pixel ▸ beside "Reset order" reads as furniture. The visible text is
    // now the accessible name as well, so the two cannot drift.
    const fold = read(COMPONENTS, "SectionFold.tsx");

    expect(fold).toContain("{label}");
    expect(fold, "an aria-label would OVERRIDE the visible words").not.toContain("aria-label=");
  });

  it("the card composes no sentence of its own", () => {
    // Both fold labels arrive from the caller's own store, and the card degrades
    // to a chevron rather than inventing words when a page has none.
    const card = read(COMPONENTS, "PairCard.tsx");

    expect(card).toContain("showLabel");
    expect(card).toContain("hideLabel");
    expect(card).not.toMatch(/>\s*Show all/);
    expect(card).not.toMatch(/>\s*Show more/);
  });
});

describe("the prep page names allegations the way the working page does", () => {
  it("the prep page composes NO label of its own (task R4, P6a)", () => {
    // The defect: chips read `A-<hash>`, because the backend built them from the
    // last segment of an anchor id and that segment is a hash. The paragraph is
    // on the Allegation node, so the ASSEMBLY reads it — not the browser.
    //
    // The first fix put the case-wide allegation catalogue in this page and
    // composed there. It worked and it was wrong twice: this page's standing law
    // is that every visible word arrives composed, and the read it needed was an
    // `authFetch` whose failure Standing Rule 1 does not let degrade quietly. So
    // what is pinned is the ABSENCE of composition here.
    const top = read(COMPONENTS, "PrepTopBlock.tsx");

    expect(top).not.toContain("labelForAllegationId");
    expect(top).toContain("scenario.bears_on.map");
  });

  it("the prep page reads no case-wide catalogue", () => {
    // The §10 line, and the Standing Rule 1 one. A page read in front of
    // opposing counsel does not fetch the complaint.
    const page = read(join(__dirname, ".."), "RehearsalPage.tsx");
    expect(page).not.toContain("getAllegations");
  });

  it("the working page's identity chips still use the browser composer", () => {
    // That surface keeps the documented exception — it already holds the
    // catalogue for its own modal, and it is not the page in the room.
    expect(read(COMPONENTS, "ScenarioIdentityBlock.tsx")).toContain("labelForAllegationId");
  });
});

describe("the prep page's talking points (task R4, P5)", () => {
  it("render the exhibit ONLY when there is one", () => {
    // `exhibit_notice` is a note to the person preparing, and this is the page
    // read in the room: a line under every point saying it has no exhibit is
    // several lines about work that did not happen, at the moment Marie can
    // least act on it. The working page says it, beside the control that fixes
    // it.
    const blocks = read(COMPONENTS, "RehearsalScenarioBlocks.tsx");

    expect(blocks).toContain("point.exhibit &&");
    expect(blocks).not.toContain("point.exhibit ?? point.exhibit_notice");
  });

  it("carry no Edit control at all", () => {
    const blocks = read(COMPONENTS, "RehearsalScenarioBlocks.tsx");
    expect(blocks).not.toContain("AuthoredLineEditor");
  });
});

describe("the scenario page's named controls exist (task R4, P1b and P4)", () => {
  it("the Scenario facts section has a fold", () => {
    // P1b. Asserted through the shared control rather than through an arrow
    // character, so a second hand-rolled fold does not satisfy it.
    const facts = read(COMPONENTS, "ScenarioFactsSection.tsx");

    expect(facts).toContain("SectionFold");
    // The count line must stay OUTSIDE the fold: a collapsed section still says
    // how many facts it holds, or it cannot be told from an empty one.
    expect(facts.indexOf("sectionMetaStyle")).toBeLessThan(facts.indexOf("{open && ("));
  });

  it("the error banner is NOT collapsible out of sight", () => {
    // A message about something that just failed must not be foldable away.
    const facts = read(COMPONENTS, "ScenarioFactsSection.tsx");
    expect(facts).toMatch(/\{error && \(/);
    expect(facts).not.toMatch(/\{open && error/);
  });

  it("the queue heading invents no sentence of its own (P4)", () => {
    // "Candidates awaiting ruling — 145" died twice over: a hardcoded English
    // sentence, and a number counting the whole unruled pool above a list showing
    // one filter's slice.
    const region = read(COMPONENTS, "queueRegion.ts");
    expect(region).not.toContain("Candidates awaiting ruling —");
  });

  it("the filter bar is one line that cannot wrap (P4)", () => {
    const bar = read(COMPONENTS, "CandidateFilterBar.tsx");
    expect(bar).toContain('flexWrap: "nowrap"');
  });
});

describe("no page renders a wording key nothing serves (the P1a class)", () => {
  it("every identity key the block reads is on the wire", () => {
    // THE P1a TEST, from the frontend side. The block destructures nine fields
    // off `identity_wording`; the Rust DTO that serialises that object must
    // declare every one of them. It did not, for a whole build, and the page
    // rendered `undefined` into four labels with nothing failing anywhere.
    //
    // The backend half of this pair walks the domain block's own key list — see
    // `dto/scenario_authoring_wording_tests.rs`. This half pins the frontend's
    // side of the same contract, so a field RENAMED here without being renamed
    // there fails at once.
    const block = read(COMPONENTS, "ScenarioIdentityBlock.tsx");
    const dto = readFileSync(
      join(__dirname, "..", "..", "..", "..", "backend", "src", "dto", "scenario_authoring_wording.rs"),
      "utf8",
    );

    const keys = [...block.matchAll(/wording\.(\w+)/g)].map((m) => m[1]);
    expect(keys.length).toBeGreaterThan(0);

    for (const key of new Set(keys)) {
      expect(dto, `the identity block renders \`${key}\`, which the DTO does not carry`).toContain(
        `pub ${key}:`,
      );
    }
  });

  it("the identity block renders a LABEL for each of its four texts", () => {
    // The specific shape of the defect: the texts rendered, the labels did not,
    // so four paragraphs sat back to back with no way to tell the attack from
    // the answer.
    const block = read(COMPONENTS, "ScenarioIdentityBlock.tsx");

    for (const label of [
      "wording.attack_label",
      "wording.theme_label",
      "wording.motivation_label",
      "wording.bears_on_label",
    ]) {
      expect(block, `${label} is not rendered`).toContain(label);
    }
    // …and a stated absence for each, rather than an empty paragraph.
    for (const absent of [
      "wording.attack_absent",
      "wording.theme_absent",
      "wording.motivation_absent",
      "wording.bears_on_absent",
    ]) {
      expect(block, `${absent} is not rendered`).toContain(absent);
    }
  });
});

// ── 2026-08-25: the rehearsal page's half of the label-weight ruling ─────────

describe("the rehearsal labels carry the same weight as the scenario card's", () => {
  /**
   * Roman ruled the scenario card's four identity labels bold in .411; these two
   * are the same visual family (11px / .08em / uppercase) on the page Marie
   * reads from, and the ruling's second half. Left at 600 they would render
   * "THE ATTACK" semibold on the rehearsal page and bold on the scenario page —
   * the same words, two weights, which is the inconsistency the ruling closes.
   *
   * Source scans, like every fence in this file, and for the same stated reason.
   */
  const cases: [string, string, string][] = [
    ["PrepTopBlock.tsx", "attackLabelStyle", "var(--v3-red-text)"],
    ["PairCard.tsx", "answerLabelStyle", "var(--state-success-strong)"],
  ];

  for (const [file, styleName, color] of cases) {
    it(`${styleName} is 700, and nothing else about it moved`, () => {
      const source = read(COMPONENTS, file);
      const start = source.indexOf(`const ${styleName}`);
      expect(start, `${styleName} must still exist in ${file}`).toBeGreaterThan(-1);
      const block = source.slice(start, source.indexOf("};", start));
      expect(block, "Roman asked for BOLD").toContain("fontWeight: 700");
      // Weight ONLY. Each of these is part of the mockup's `.lbl` family and a
      // change to any of them would be this rider exceeding its scope.
      expect(block).toContain('fontSize: "11px"');
      expect(block).toContain('letterSpacing: "0.08em"');
      expect(block).toContain('textTransform: "uppercase"');
      expect(block).toContain(`color: "${color}"`);
    });
  }

  it("leaves the scenario page's SCENARIO eyebrow at 600", () => {
    // The one member of the family Roman excluded, twice now. Asserted here as
    // well as in scenarioPageStructure.test.ts because "make the small caps
    // bold" is the edit that would sweep it up, and the person making it will be
    // looking at the rehearsal page.
    const header = read(COMPONENTS, "ScenarioHeaderTiers.tsx");
    const eyebrow = header.slice(
      header.indexOf("const eyebrowStyle"),
      header.indexOf("const headerRowStyle"),
    );
    expect(eyebrow).toContain("fontWeight: 600");
  });
});

