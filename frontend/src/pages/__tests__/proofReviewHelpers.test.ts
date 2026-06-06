/**
 * Pure-helper tests for the Proof Review page.
 *
 * Locks the view-shaping contracts the page depends on: the document-filter
 * option list, the sub-tab badge counts, the Summary view-model, the empty-state
 * flags, and client-side edge filtering. No DOM / RTL — pure functions only
 * (CLAUDE.md §30), mirroring countCardHelpers.test.ts.
 *
 * The fixtures reproduce the live data SHAPE at the real magnitudes
 * (43 corroborations / 79 excluded / 19 borderline; excluded split
 * 45/21/11/2) so the badge and summary assertions are the numbers a reviewer
 * will see on DEV.
 */
import { describe, expect, it } from "vitest";
import {
  buildSummaryView,
  distinctSourceDocuments,
  distinctStatementTypes,
  filterEdges,
  sectionEmptyStates,
  subTabBadgeCounts,
  EDGE_FILTER_ALL,
} from "../proofReviewHelpers";
import type {
  ExcludedEvidence,
  ProofEdge,
  ProofReviewResponse,
  ProofReviewSummary,
} from "../../services/proofReview";

// ─── Factories ───────────────────────────────────────────────────────────────

const makeEdge = (overrides: Partial<ProofEdge> = {}): ProofEdge => ({
  answer: "Yes.",
  question: "Did you sign it?",
  evidence_verbatim_quote: "Yes, I signed it.",
  statement_type: "admission",
  evidence_strength: "sworn_party_admission",
  paragraph: "Q4",
  page_number: 12,
  source_document: "doc-george",
  allegation_summary: "The agreement was signed.",
  allegation_title: "Signature",
  allegation_paragraph_number: "54",
  allegation_id: "alleg-1",
  ...overrides,
});

const makeExcluded = (
  overrides: Partial<ExcludedEvidence> = {},
): ExcludedEvidence => ({
  answer: "Objection.",
  question: "State your assets.",
  evidence_verbatim_quote: "Objection, overbroad.",
  statement_type: "evasive",
  paragraph: "Q9",
  page_number: 3,
  source_document: "doc-george",
  ...overrides,
});

const fill = <T>(n: number, f: (i: number) => T): T[] =>
  Array.from({ length: n }, (_, i) => f(i));

// A summary matching the live numbers: corroborating 43 (admission 24 +
// partial_admission 19); excluded 79 (evasive 45 / referral 21 / denial 11 /
// objection 2).
const liveSummary: ProofReviewSummary = {
  corroborating: {
    total: 43,
    by_statement_type: [
      { statement_type: "admission", count: 24 },
      { statement_type: "partial_admission", count: 19 },
    ],
    by_category: [
      { statement_type: "admission", evidence_strength: "sworn_party_admission", count: 24 },
      { statement_type: "partial_admission", evidence_strength: "sworn_party_admission", count: 19 },
    ],
  },
  excluded: {
    total: 79,
    by_statement_type: [
      { statement_type: "evasive", count: 45 },
      { statement_type: "referral", count: 21 },
      { statement_type: "denial", count: 11 },
      { statement_type: "objection", count: 2 },
    ],
  },
};

const livePayload: ProofReviewResponse = {
  case_slug: "awad_v_catholic_family_service",
  document_id: null,
  summary: liveSummary,
  proof_edges: fill(43, (i) =>
    makeEdge({ statement_type: i < 24 ? "admission" : "partial_admission", allegation_id: `a-${i}` }),
  ),
  excluded: fill(79, (i) => makeExcluded({ statement_type: "evasive", page_number: i })),
  borderline: fill(19, (i) =>
    makeEdge({ statement_type: "partial_admission", allegation_id: `b-${i}` }),
  ),
};

const emptyPayload: ProofReviewResponse = {
  case_slug: "c",
  document_id: "doc-x",
  summary: {
    corroborating: { total: 0, by_statement_type: [], by_category: [] },
    excluded: { total: 0, by_statement_type: [] },
  },
  proof_edges: [],
  excluded: [],
  borderline: [],
};

// ─── Tests ───────────────────────────────────────────────────────────────────

describe("subTabBadgeCounts", () => {
  it("reads the three list lengths verbatim (43 / 79 / 19)", () => {
    expect(subTabBadgeCounts(livePayload)).toEqual({
      proofEdges: 43,
      excluded: 79,
      borderline: 19,
    });
  });

  it("is zero for an empty payload", () => {
    expect(subTabBadgeCounts(emptyPayload)).toEqual({
      proofEdges: 0,
      excluded: 0,
      borderline: 0,
    });
  });
});

describe("buildSummaryView", () => {
  it("surfaces the two corroborating totals and the four excluded categories", () => {
    const view = buildSummaryView(liveSummary);
    expect(view.corroboratingTotal).toBe(43);
    expect(view.excludedTotal).toBe(79);
    // The four excluded categories, verbatim.
    expect(view.excludedByStatementType.map((r) => [r.statement_type, r.count])).toEqual([
      ["evasive", 45],
      ["referral", 21],
      ["denial", 11],
      ["objection", 2],
    ]);
    // The two corroborating statement types.
    expect(view.corroboratingByStatementType.map((r) => r.statement_type)).toEqual([
      "admission",
      "partial_admission",
    ]);
  });

  it("does not invent or sum — totals come straight from the payload", () => {
    // A deliberately inconsistent fixture (total ≠ sum of buckets) must pass
    // through unchanged: the helper reports the backend's total, never recomputes.
    const view = buildSummaryView({
      corroborating: { total: 999, by_statement_type: [], by_category: [] },
      excluded: { total: 1, by_statement_type: [{ statement_type: "denial", count: 1 }] },
    });
    expect(view.corroboratingTotal).toBe(999);
    expect(view.excludedTotal).toBe(1);
  });
});

describe("distinctSourceDocuments", () => {
  it("returns distinct, sorted source documents across all sections", () => {
    const payload: ProofReviewResponse = {
      ...emptyPayload,
      proof_edges: [
        makeEdge({ source_document: "doc-zeta" }),
        makeEdge({ source_document: "doc-alpha" }),
        makeEdge({ source_document: "doc-zeta" }),
      ],
      excluded: [makeExcluded({ source_document: "doc-alpha" })],
      borderline: [makeEdge({ source_document: "doc-mid" })],
    };
    expect(distinctSourceDocuments(payload)).toEqual(["doc-alpha", "doc-mid", "doc-zeta"]);
  });

  it("skips null source documents", () => {
    const payload: ProofReviewResponse = {
      ...emptyPayload,
      proof_edges: [makeEdge({ source_document: null }), makeEdge({ source_document: "doc-a" })],
    };
    expect(distinctSourceDocuments(payload)).toEqual(["doc-a"]);
  });
});

describe("distinctStatementTypes", () => {
  it("returns distinct, sorted statement types present on the edges", () => {
    const edges = [
      makeEdge({ statement_type: "partial_admission" }),
      makeEdge({ statement_type: "admission" }),
      makeEdge({ statement_type: "admission" }),
    ];
    expect(distinctStatementTypes(edges)).toEqual(["admission", "partial_admission"]);
  });

  it("returns an empty array for no edges (the empty-payload case)", () => {
    // ProofReviewPage feeds this select an empty array when proof_edges is
    // empty — it must yield [], never throw.
    expect(distinctStatementTypes([])).toEqual([]);
  });
});

describe("sectionEmptyStates", () => {
  it("flags every section non-empty for populated data", () => {
    expect(sectionEmptyStates(livePayload)).toEqual({
      summaryEmpty: false,
      proofEdgesEmpty: false,
      excludedEmpty: false,
      borderlineEmpty: false,
    });
  });

  it("flags every section empty for empty data", () => {
    expect(sectionEmptyStates(emptyPayload)).toEqual({
      summaryEmpty: true,
      proofEdgesEmpty: true,
      excludedEmpty: true,
      borderlineEmpty: true,
    });
  });

  it("summary is empty only when BOTH totals are zero", () => {
    const onlyExcluded: ProofReviewResponse = {
      ...emptyPayload,
      summary: {
        corroborating: { total: 0, by_statement_type: [], by_category: [] },
        excluded: { total: 5, by_statement_type: [{ statement_type: "denial", count: 5 }] },
      },
    };
    expect(sectionEmptyStates(onlyExcluded).summaryEmpty).toBe(false);
  });
});

describe("filterEdges", () => {
  const edges = [
    makeEdge({ statement_type: "admission", source_document: "doc-a", allegation_id: "1" }),
    makeEdge({ statement_type: "partial_admission", source_document: "doc-a", allegation_id: "2" }),
    makeEdge({ statement_type: "admission", source_document: "doc-b", allegation_id: "3" }),
  ];

  it("returns all edges when both dimensions are 'all'", () => {
    const out = filterEdges(edges, {
      statementType: EDGE_FILTER_ALL,
      sourceDocument: EDGE_FILTER_ALL,
    });
    expect(out).toHaveLength(3);
  });

  it("filters by statement_type", () => {
    const out = filterEdges(edges, {
      statementType: "admission",
      sourceDocument: EDGE_FILTER_ALL,
    });
    expect(out.map((e) => e.allegation_id)).toEqual(["1", "3"]);
  });

  it("filters by source_document", () => {
    const out = filterEdges(edges, {
      statementType: EDGE_FILTER_ALL,
      sourceDocument: "doc-a",
    });
    expect(out.map((e) => e.allegation_id)).toEqual(["1", "2"]);
  });

  it("filters by both dimensions together", () => {
    const out = filterEdges(edges, { statementType: "admission", sourceDocument: "doc-b" });
    expect(out.map((e) => e.allegation_id)).toEqual(["3"]);
  });

  it("does not mutate the input array (purity)", () => {
    const before = edges.map((e) => e.allegation_id);
    filterEdges(edges, { statementType: "admission", sourceDocument: EDGE_FILTER_ALL });
    expect(edges.map((e) => e.allegation_id)).toEqual(before);
  });

  it("excludes an edge with a null source_document when a specific doc is selected", () => {
    // An edge with no source_document cannot match a concrete document filter,
    // so it drops out — but stays visible under the 'all' default.
    const withNull = [
      makeEdge({ source_document: "doc-a", allegation_id: "1" }),
      makeEdge({ source_document: null, allegation_id: "2" }),
    ];
    expect(
      filterEdges(withNull, { statementType: EDGE_FILTER_ALL, sourceDocument: "doc-a" }).map(
        (e) => e.allegation_id,
      ),
    ).toEqual(["1"]);
    // Under 'all', the null-document edge is still shown.
    expect(
      filterEdges(withNull, {
        statementType: EDGE_FILTER_ALL,
        sourceDocument: EDGE_FILTER_ALL,
      }),
    ).toHaveLength(2);
  });
});
