/**
 * Tests for the working view's pure core (task 1.4).
 *
 * The row anatomy and the client-side search. Per CLAUDE.md rule 30 the frontend
 * has no DOM test infrastructure by deliberate convention, so the logic lives in
 * pure functions and is tested as data.
 */
import { describe, expect, it } from "vitest";

import { filterRows, humanFactRows, includedRows, type WorkingRow,
  arrivedIds,
  CARD_ANATOMY,
  neighboursForDrop,
  orderedRows,
  sectionsFor,
  splitBackground,
} from "../factsTable";
import type { ScenarioCard } from "../../services/scenarioCards";

function card(overrides: Partial<ScenarioCard> = {}): ScenarioCard {
  return {
    code: "C-14",
    graph_node_id: "ev-1",
    quote: {
      text: "I do not recall that meeting.",
      context_before: "",
      context_after: "",
      // Task 1.7C (D6). An empty flank did not stop mid-sentence, so it counts as
      // complete and carries no page-edge notice — the backend's own rule for the
      // no-context case (`ContextFlank::absent`).
      context_before_complete: true,
      context_after_complete: true,
      context_before_notice: null,
      context_after_notice: null,
      question: null,
    },
    pinpoint: {
      document_id: "doc-7",
      document_title: "CFS responses",
      label: "CFS responses at 26",
      page: 26,
      viewer_href: "/documents/doc-7?page=26&tab=document",
    },
    speaker: { name: "R. Phillips", attribution: "extracted" },
    statement_kind: "admission",
    stance: null,
    bears_on: [{ accusation: "¶54 — CFS knew of the meeting", elements: [], count: null }],
    grounding: null,
    confidence: { band: "unscored", label: "Not scored by a scan" },
    status: "included",
    status_label: "In the scenario",
    defer_required: false,
    defer_required_reason: null,
    defer_reason: null,
    // Task 2.10: no human has linked this one — the ordinary state.
    human_links: [],
    human_link_summary: null,
    ...overrides,
  };
}

describe("includedRows", () => {
  it("keeps only the items a human put in the scenario", () => {
    const rows = includedRows([
      card({ graph_node_id: "in", status: "included" }),
      card({ graph_node_id: "undecided", status: "undecided" }),
      card({ graph_node_id: "aside", status: "dropped" }),
    ]);
    expect(rows.map((r) => r.graphNodeId)).toEqual(["in"]);
  });

  it("filters on the state token, never on the display label", () => {
    // The regression guard for the field pair added in 1.4: a card whose label
    // was reworded must still be included, because the STATE is what decides.
    const rows = includedRows([
      card({ status: "included", status_label: "Some future wording" }),
    ]);
    expect(rows).toHaveLength(1);
  });

  it("carries the payload's own strings into the row", () => {
    const source = card();
    const [row] = includedRows([source]);
    expect(row.text).toBe(source.quote.text);
    expect(row.pinpointLabel).toBe(source.pinpoint.label);
    expect(row.pinpointHref).toBe(source.pinpoint.viewer_href);
    expect(row.statusLabel).toBe(source.status_label);
    expect(row.bearsOn).toEqual(["¶54 — CFS knew of the meeting"]);
  });

  it("returns no rows when nothing has been included yet", () => {
    // A real state — the scenario is curated but nothing kept — not an error.
    expect(includedRows([card({ status: "undecided" })])).toEqual([]);
  });
});

describe("filterRows", () => {
  const rows: WorkingRow[] = [
    {
      code: "C-1",
      graphNodeId: "ev-1",
      text: "I do not recall that meeting.",
      bearsOn: ["¶54 — CFS knew of the meeting"],
      pinpointLabel: "CFS responses at 26",
      pinpointHref: "#",
      statusLabel: "In the scenario",
      isHuman: false,
      question: null,
      statementKind: null,
      tier: "backup",
      sortOrdinal: null,
      speaker: null,
      displayOrdinal: null,
    },
    {
      code: "C-2",
      graphNodeId: "ev-2",
      text: "That would be correct.",
      bearsOn: ["¶12 — the contract was undisclosed"],
      pinpointLabel: "Phillips deposition at 88",
      pinpointHref: "#",
      statusLabel: "In the scenario",
      isHuman: false,
      question: null,
      statementKind: null,
      tier: "backup",
      sortOrdinal: null,
      speaker: null,
      displayOrdinal: null,
    },
  ];

  it("an empty term shows everything", () => {
    // "No filter" and "a filter matching nothing" are different states.
    expect(filterRows(rows, "")).toHaveLength(2);
    expect(filterRows(rows, "   ")).toHaveLength(2);
  });

  it("matches the quote text, case-insensitively", () => {
    expect(filterRows(rows, "RECALL")).toHaveLength(1);
  });

  it("matches an accusation", () => {
    const found = filterRows(rows, "undisclosed");
    expect(found.map((r) => r.code)).toEqual(["C-2"]);
  });

  it("matches the pinpoint and the code", () => {
    expect(filterRows(rows, "deposition").map((r) => r.code)).toEqual(["C-2"]);
    expect(filterRows(rows, "c-1").map((r) => r.code)).toEqual(["C-1"]);
  });

  it("returns nothing when nothing matches — an honest empty result", () => {
    expect(filterRows(rows, "zzz")).toEqual([]);
  });

  it("never matches on a field the row does not display", () => {
    // Searching a hidden field would give a hit whose reason is invisible, which
    // reads as a broken filter.
    expect(filterRows(rows, "ev-1")).toEqual([]);
  });
});

// ── Task 1.7D item 6 / ruling R6: the coloured left-edge cue's data ──────────

describe("the isHuman flag that drives the row stripe", () => {
  it("marks every evidence row as NOT human-authored", () => {
    // A card exists because the graph produced it — it is evidence a human ruled
    // in, never something they wrote.
    const rows = includedRows([card({ status: "included" })]);
    expect(rows).toHaveLength(1);
    expect(rows[0].isHuman).toBe(false);
  });

  it("marks a human fact as human-authored", () => {
    const rows = humanFactRows([
      { id: "hf-1", text: "Marie's certified letter", authored_tag: "Added by Roman", date_label: null },
    ]);
    expect(rows[0].isHuman).toBe(true);
  });

  it("does NOT infer provenance from a missing pinpoint", () => {
    // The correlation holds today (§8 gives human facts no citation) but the
    // correlation is not the fact. An evidence row that legitimately lacks a page
    // must still read as evidence, or the stripe would call it Marie's.
    const rows = includedRows([
      card({
        status: "included",
        pinpoint: {
          document_id: "doc-7",
          document_title: "CFS responses",
          label: "CFS responses",
          page: null,
          viewer_href: "/documents/doc-7?tab=document",
        },
      }),
    ]);
    expect(rows[0].pinpointLabel).toBe("CFS responses");
    expect(rows[0].isHuman).toBe(false);
  });
});

describe("humanFactRows", () => {
  it("carries the composed provenance as the row's authority", () => {
    // "Added by Roman" arrives composed from the backend — the browser does not
    // write provenance. It stands where an evidence row's ruling state stands,
    // because it is what this row's authority IS.
    const rows = humanFactRows([
      { id: "hf-1", text: "A fact no document says", authored_tag: "Added by Roman", date_label: "Around Mar 2010" },
    ]);
    expect(rows[0].statusLabel).toBe("Added by Roman · Around Mar 2010");
  });

  it("omits the date clause when the fact is undated", () => {
    const rows = humanFactRows([
      { id: "hf-1", text: "A fact", authored_tag: "Added by Roman", date_label: null },
    ]);
    expect(rows[0].statusLabel).toBe("Added by Roman");
  });

  it("carries NO pinpoint, because a human fact has none by design (§8)", () => {
    const rows = humanFactRows([
      { id: "hf-1", text: "A fact", authored_tag: "Added by Roman", date_label: null },
    ]);
    expect(rows[0].pinpointHref).toBe("");
    expect(rows[0].pinpointLabel).toBe("");
  });

  it("namespaces the row key so a human fact cannot collide with a graph node", () => {
    // Both kinds share one list now, and React keys off `graphNodeId`. An
    // unprefixed uuid could in principle collide with a Family-A id; the prefix
    // makes that impossible rather than unlikely.
    const rows = humanFactRows([
      { id: "hf-1", text: "A fact", authored_tag: "t", date_label: null },
    ]);
    expect(rows[0].graphNodeId).toBe("human:hf-1");
  });

  it("is searchable alongside evidence rows", () => {
    // One table, one filter. A human fact the search could not reach would be a
    // fact that disappears the moment someone types.
    const rows = [
      ...includedRows([card({ status: "included" })]),
      ...humanFactRows([
        { id: "hf-1", text: "certified letter of March 2010", authored_tag: "Added by Roman", date_label: null },
      ]),
    ];
    const hits = filterRows(rows, "certified");
    expect(hits).toHaveLength(1);
    expect(hits[0].isHuman).toBe(true);
  });
});

// ─── Which rows just arrived (task 1.7F Part A) ──────────────────────────────

describe("arrivedIds", () => {
  it("reports nothing on the first payload", () => {
    // Every row is new on a page that just loaded. Tinting the whole table would
    // say nothing, and would say it loudly.
    expect(arrivedIds(null, ["ev-1", "ev-2"])).toEqual([]);
  });

  it("reports the row a ruling just added", () => {
    // The include case: one id is present now and was not before.
    expect(arrivedIds(new Set(["ev-1"]), ["ev-1", "ev-2"])).toEqual(["ev-2"]);
  });

  it("reports nothing when the payload is unchanged", () => {
    // A re-read that changed nothing must not flash the list.
    expect(arrivedIds(new Set(["ev-1", "ev-2"]), ["ev-1", "ev-2"])).toEqual([]);
  });

  it("reports nothing when a row DISAPPEARS", () => {
    // Undo removes a row. There is nothing to highlight — the thing that changed
    // is no longer on screen — and highlighting a survivor would point at the
    // wrong row.
    expect(arrivedIds(new Set(["ev-1", "ev-2"]), ["ev-1"])).toEqual([]);
  });

  it("reports every arrival when several land at once", () => {
    // A merge can add more than one. All of them are what changed.
    expect(arrivedIds(new Set(["ev-1"]), ["ev-1", "ev-2", "ev-3"])).toEqual(["ev-2", "ev-3"]);
  });

  it("a human's accusation links become chips too (task 2.10)", () => {
    // A human's link is what put some of these rows in the table at all — the
    // card could not be included until somebody linked it — so a facts table
    // showing only the extraction's accusations would leave exactly those rows
    // looking attached to nothing.
    const [row] = includedRows([
      card({
        status: "included",
        bears_on: [{ accusation: "\u00b655 — regularly misrepresented", elements: [], count: null }],
        human_links: [
          {
            allegation_id: "a-41",
            label: "\u00b641 — refused to divide the property",
            cut: "against",
            cut_label: "They'll use it against us",
          },
        ],
      }),
    ]);

    expect(row.bearsOn).toEqual([
      "\u00b655 — regularly misrepresented",
      "\u00b641 — refused to divide the property",
    ]);
  });

  it("a card with only human links still shows them", () => {
    const [row] = includedRows([
      card({
        status: "included",
        bears_on: [],
        human_links: [
          {
            allegation_id: "a-41",
            label: "\u00b641 — refused to divide the property",
            cut: "supports",
            cut_label: "It supports us",
          },
        ],
      }),
    ]);
    expect(row.bearsOn).toEqual(["\u00b641 — refused to divide the property"]);
  });
});

// ── Task 2.13: weight, order, and the question ──────────────────────────────

describe("the question on a fact row", () => {
  it("carries a question through, and folds an empty one into absent", () => {
    // present → present; absent → absent; EMPTY STRING → absent. The third is the
    // one worth pinning: an empty question would render a "Q:" label introducing
    // nothing, which asserts that a question exists and was lost.
    // `question` lives on the quote, so it is overridden there — the same shape
    // the payload actually has.
    const withQuestion = (question: string | null) =>
      includedRows([
        card({ status: "included", quote: { ...card().quote, question } }),
      ])[0].question;

    expect(withQuestion("State the basis for your contention.")).toBe(
      "State the basis for your contention.",
    );
    expect(withQuestion(null)).toBeNull();
    expect(withQuestion("")).toBeNull();
  });
});

describe("orderedRows", () => {
  const row = (
    id: string,
    tier: WorkingRow["tier"],
    sortOrdinal: number | null,
  ): WorkingRow => ({
    code: id,
    graphNodeId: id,
    text: id,
    bearsOn: [],
    pinpointLabel: "",
    pinpointHref: "",
    statusLabel: "In the scenario",
    isHuman: false,
    question: null,
    statementKind: null,
    tier,
    sortOrdinal,
    speaker: null,
    displayOrdinal: null,
  });

  it("leaves an untouched list in exactly the order it arrived", () => {
    // THE regression guard: a scenario nobody has dragged in must render
    // identically to how it did before this task shipped. The server's C-ordinal
    // sort is the incoming order, and a stable sort must not disturb it.
    const rows = [
      row("c1", "backup", null),
      row("c2", "backup", null),
      row("c3", "backup", null),
    ];
    expect(orderedRows(rows).map((r) => r.graphNodeId)).toEqual(["c1", "c2", "c3"]);
  });

  it("sorts on the SERVER's position, not the stored one", () => {
    // Task 2.13c. This used to assert "placed facts first, then unplaced" —
    // the browser's own two-region model, which is exactly what made a newly
    // included fact land fourth instead of last. The server now computes ONE
    // number per card and this sorts on that.
    //
    // The rows below make the distinction impossible to fudge: `sortOrdinal`
    // deliberately disagrees with `displayOrdinal`, and the displayed order must
    // follow the latter.
    const rows = [
      { ...row("third", "backup", 10), displayOrdinal: 3072 },
      { ...row("first", "backup", 9999), displayOrdinal: 1024 },
      { ...row("second", "backup", null), displayOrdinal: 2048 },
    ];
    expect(orderedRows(rows).map((r) => r.graphNodeId)).toEqual([
      "first",
      "second",
      "third",
    ]);
  });

  it("keeps a card with no served position behind the ones that have one", () => {
    // A human fact has no display position. It must not jump the queue, and it
    // must not disappear — it follows, in the order the payload sent.
    const rows = [
      { ...row("no-position", "backup", null), displayOrdinal: null },
      { ...row("placed", "backup", null), displayOrdinal: 1024 },
    ];
    expect(orderedRows(rows).map((r) => r.graphNodeId)).toEqual([
      "placed",
      "no-position",
    ]);
  });

  it("groups by weight ahead of position", () => {
    // Weight decides the block, position decides the seat. A background fact with
    // a tiny ordinal must not outrank a carrying fact with a large one — otherwise
    // starring something would silently reorder the list.
    const rows = [
      row("light", "background", 1),
      row("heavy", "carries", 9000),
      row("middle", "backup", 5),
    ];
    expect(orderedRows(rows).map((r) => r.graphNodeId)).toEqual(["heavy", "middle", "light"]);
  });

  it("sorts a human fact with the middle tier, never above starred evidence", () => {
    // A human fact has NO tier — §8 keeps it uncited, and nobody weighs it. It
    // must not therefore float to the top ahead of evidence somebody deliberately
    // starred, nor sink into the background pile they folded away. `rankOf`
    // places it with `backup`, and this pins that: a future change promoting
    // null-tier rows to rank 0 would reorder Roman's list silently.
    const human = { ...row("human:1", null, null), isHuman: true };
    const rows = [row("light", "background", null), human, row("heavy", "carries", null)];
    expect(orderedRows(rows).map((r) => r.graphNodeId)).toEqual([
      "heavy",
      "human:1",
      "light",
    ]);
  });

  it("does not mutate the array it was given", () => {
    // The rows come from a `useMemo` over props; sorting in place would reorder
    // React's own data and make renders depend on how many times this ran.
    const rows = [row("b", "backup", 2), row("a", "backup", 1)];
    orderedRows(rows);
    expect(rows.map((r) => r.graphNodeId)).toEqual(["b", "a"]);
  });
});

describe("splitBackground", () => {
  const row = (id: string, tier: WorkingRow["tier"]): WorkingRow => ({
    code: id,
    graphNodeId: id,
    text: id,
    bearsOn: [],
    pinpointLabel: "",
    pinpointHref: "",
    statusLabel: "In the scenario",
    isHuman: false,
    question: null,
    statementKind: null,
    tier,
    sortOrdinal: null,
    speaker: null,
    displayOrdinal: null,
  });

  it("folds the background tier out without losing any of it", () => {
    // A split, never a filter: every row must still be reachable, because a
    // curated fact that silently disappears is the failure this tier exists to
    // avoid.
    const rows = [row("a", "carries"), row("b", "background"), row("c", "backup")];
    const { shown, background } = splitBackground(rows);
    expect(shown.map((r) => r.graphNodeId)).toEqual(["a", "c"]);
    expect(background.map((r) => r.graphNodeId)).toEqual(["b"]);
    expect(shown.length + background.length).toBe(rows.length);
  });

  it("keeps a human fact out of the background pile", () => {
    // A human fact has no tier at all, and must never be folded away by a weight
    // judgment that was never made about it.
    const human = { ...row("h", null), isHuman: true };
    expect(splitBackground([human]).background).toEqual([]);
    expect(splitBackground([human]).shown).toEqual([human]);
  });
});

describe("neighboursForDrop", () => {
  const row = (id: string): WorkingRow => ({
    code: id,
    graphNodeId: id,
    text: id,
    bearsOn: [],
    pinpointLabel: "",
    pinpointHref: "",
    statusLabel: "In the scenario",
    isHuman: false,
    question: null,
    statementKind: null,
    tier: "backup",
    sortOrdinal: null,
    speaker: null,
    displayOrdinal: null,
  });
  const rows = [row("a"), row("b"), row("c")];

  it("names the pair a row lands between when dropped onto a target", () => {
    // Dropping "c" onto "b" means "put c where b is" — above b, below a.
    expect(neighboursForDrop(rows, "c", "b")).toEqual({ after: "a", before: "b" });
  });

  it("reports no lower neighbour at the top of the list", () => {
    expect(neighboursForDrop(rows, "c", "a")).toEqual({ after: null, before: "a" });
  });

  it("lifts the dragged row out before reading its neighbours", () => {
    // Dropping "a" onto "b" must see "b" as the FIRST remaining row, not the
    // second — otherwise a drop one place down is computed as a no-op.
    expect(neighboursForDrop(rows, "a", "b")).toEqual({ after: null, before: "b" });
  });

  it("refuses a drop that names no position", () => {
    // Onto itself, or onto a row that has gone: nothing is sent, because the
    // human meant nothing by it and the server would only have to refuse it.
    expect(neighboursForDrop(rows, "a", "a")).toBeNull();
    expect(neighboursForDrop(rows, "a", "ghost")).toBeNull();
  });
});

// ── Task 2.13b: the card anatomy is fixed, and testable ─────────────────────

describe("sectionsFor — the fixed card anatomy", () => {
  const base = (over: Partial<WorkingRow> = {}): WorkingRow => ({
    code: "C-1",
    graphNodeId: "ev-1",
    text: "Yes.",
    bearsOn: [],
    pinpointLabel: "",
    pinpointHref: "",
    statusLabel: "In the scenario",
    isHuman: false,
    question: null,
    statementKind: null,
    tier: "backup",
    sortOrdinal: null,
    speaker: null,
    displayOrdinal: null,
    ...over,
  });

  it("returns the sections in the canonical order, always", () => {
    // Roman, 2026-08-05: "the Candidate numbers appear in different places
    // cards." They did, because the code shared a wrapping row with whatever
    // else fitted. The order is now data, and this is the assertion that it
    // cannot vary.
    const full = base({
      question: "State the basis for your contention.",
      bearsOn: ["¶41 — …"],
      statementKind: "admission",
      speaker: "R. Phillips",
    });
    expect(sectionsFor(full)).toEqual([
      "header",
      "question",
      "quote",
      "allegations",
      "source",
    ]);
  });

  it("collapses an absent section without reordering the rest", () => {
    // The ruling: "An absent element's row collapses; nothing reflows into its
    // position." So a card with no question must produce the SAME sequence minus
    // that one entry — never a different arrangement of what is left.
    const noQuestion = sectionsFor(base({ bearsOn: ["¶41 — …"] }));
    expect(noQuestion).toEqual(["header", "quote", "allegations", "source"]);

    const bare = sectionsFor(base());
    expect(bare).toEqual(["header", "quote", "source"]);

    // The fourth corner of the 2x2. `question` and `allegations` are the only
    // conditional sections, so there are exactly four input combinations and
    // three of them were covered — question-without-allegations was not, and it
    // is the one where a naive implementation would be most tempted to slide the
    // question down into the gap the allegations left.
    const questionOnly = sectionsFor(base({ question: "State the basis." }));
    expect(questionOnly).toEqual(["header", "question", "quote", "source"]);

    // Every card's sections are a SUBSEQUENCE of the canonical order — the
    // property that makes "nothing reflows" true for any combination, and now
    // asserted over the COMPLETE combination space rather than a sample.
    for (const row of [noQuestion, bare, questionOnly]) {
      const positions = row.map((s) => CARD_ANATOMY.indexOf(s));
      expect(positions).toEqual([...positions].sort((a, b) => a - b));
    }
  });

  it("gives every card a header and a quote", () => {
    // The landmark line and the words are the two things a fact always has. If
    // either could be absent, the card would have no fixed anchor at all.
    const shapes = [
      base(),
      base({ isHuman: true, code: null, tier: null }),
      base({ question: "Q?", bearsOn: ["¶1"], speaker: "X" }),
    ];
    for (const shape of shapes) {
      expect(sectionsFor(shape)).toContain("header");
      expect(sectionsFor(shape)).toContain("quote");
    }
  });

  it("keeps a human fact on the same anatomy as evidence", () => {
    // "Same anatomy inside the background pile" and for human facts: the card
    // must not become a different KIND of object because a person wrote it.
    const human = base({ isHuman: true, code: null, tier: null, bearsOn: [] });
    expect(sectionsFor(human)).toEqual(["header", "quote", "source"]);
  });
});
