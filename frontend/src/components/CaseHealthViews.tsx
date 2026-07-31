// =============================================================================
// CaseHealthViews.tsx — presentational pieces for the Case Health dashboard
// -----------------------------------------------------------------------------
// Pure presentational components (props in → JSX out): no fetch, no state, no
// arithmetic, no sorting. Every figure rendered here arrives already computed
// and already ordered from `GET /api/cases/:slug/case-health/inventory`; these
// components choose typography and layout and nothing else. Kept in their own
// file so the page stays a thin orchestrator and no module exceeds the size
// limit (the split TrialPrepViews / TrialPrepDashboardPage established).
//
// The hard visual rule of this pane: the PROBATIVE rate is the headline, and the
// TOPICAL rate sits beside it under its own label with its own explanation. They
// are never adjacent bare numbers a reader could mistake for one metric, and
// nothing here ever combines them.
// =============================================================================

import React from "react";

import {
  edgeClassCount,
  formatRate,
  type CorpusConnection,
  type DocumentConnection,
  type EdgeTripleCount,
  type LabelCount,
} from "../services/caseHealth";

import {
  EMDASH,
  headlineCard,
  headlineHelp,
  headlineLabel,
  headlineRow,
  headlineValue,
  noticeStyle,
  primaryCard,
  scrollWrap,
  secondaryValue,
  sectionHeading,
  sectionNote,
  sectionStyle,
  tableStyle,
  tdNumeric,
  tdStrong,
  tdStyle,
  thNumeric,
  thStyle,
} from "./caseHealthStyles";

// ─── Headline ────────────────────────────────────────────────────────────────

/**
 * The corpus rollup: probative rate as the headline, topical rate beside it.
 *
 * The two cards are deliberately unequal — different sizes, different emphasis,
 * each with its own one-line explanation of what it counts — because a reader
 * glancing at two identically-styled percentages would reasonably assume they
 * were variations on one number. They are not: one asserts that evidence bears
 * on whether an allegation is true, the other only that it is on the subject.
 */
export function CorpusHeadline({ corpus }: { corpus: CorpusConnection }) {
  return (
    <div style={sectionStyle}>
      <div style={headlineRow}>
        <div style={primaryCard}>
          <div style={headlineValue}>{formatRate(corpus.probative_rate_pct)}</div>
          <div style={headlineLabel}>Probative connection rate</div>
          <div style={headlineHelp}>
            {corpus.probative_connected.toLocaleString()} of{" "}
            {corpus.evidence_total.toLocaleString()} Evidence items carry at least
            one CORROBORATES, REBUTS or CHARACTERIZES edge to an Allegation —
            edges that bear on whether the allegation is true.
          </div>
        </div>

        <div style={headlineCard}>
          <div style={secondaryValue}>{formatRate(corpus.topical_rate_pct)}</div>
          <div style={headlineLabel}>Topical connection rate</div>
          <div style={headlineHelp}>
            {corpus.topical_connected.toLocaleString()} items, adding ABOUT edges.
            ABOUT says only that an item is on the subject of an allegation, so it
            is reported separately and never counted in the headline.
          </div>
        </div>

        <div style={headlineCard}>
          <div style={secondaryValue}>{formatRate(corpus.inert_rate_pct)}</div>
          <div style={headlineLabel}>Inert</div>
          <div style={headlineHelp}>
            {corpus.inert.toLocaleString()} Evidence items reach no Allegation at
            all, by any of the four edge classes.
          </div>
        </div>
      </div>

      {corpus.evidence_without_document > 0 && (
        <div style={noticeStyle}>
          {corpus.evidence_without_document.toLocaleString()} Evidence{" "}
          {corpus.evidence_without_document === 1 ? "item is" : "items are"} not
          linked to any Document. The corpus figures above count every Evidence
          node; the per-document table below can only see those that reach a
          Document, so the two will differ by this amount.
        </div>
      )}
    </div>
  );
}

// ─── Per-document connection table ───────────────────────────────────────────

/**
 * One row per Document: what it yielded and how much of that is wired in.
 *
 * `edgeClasses` drives the per-class columns, so the frontend holds no edge
 * vocabulary of its own — a class added to the backend's tier definition appears
 * here as a new column with no change to this file.
 */
export function DocumentConnectionTable({
  documents,
  edgeClasses,
}: {
  documents: DocumentConnection[];
  edgeClasses: string[];
}) {
  return (
    <div style={sectionStyle}>
      <div style={sectionHeading}>Per-document connection</div>
      <div style={sectionNote}>
        Ordered by Evidence produced. The per-class columns are ITEM counts and do
        not sum to the connected totals — one item may both corroborate one
        allegation and rebut another.
      </div>

      {documents.length === 0 ? (
        <div style={sectionNote}>No Document nodes in the graph.</div>
      ) : (
        <div style={scrollWrap}>
          <table style={tableStyle}>
            <thead>
              <tr>
                <th style={thStyle}>Document</th>
                <th style={thStyle}>Type</th>
                <th style={thNumeric}>Evidence</th>
                <th style={thNumeric}>Probative</th>
                <th style={thNumeric}>Probative %</th>
                <th style={thNumeric}>Topical</th>
                <th style={thNumeric}>Topical %</th>
                <th style={thNumeric}>Inert</th>
                <th style={thNumeric}>Inert %</th>
                {edgeClasses.map((cls) => (
                  <th key={cls} style={thNumeric}>
                    {cls}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {documents.map((doc) => (
                <tr key={doc.document_id}>
                  <td style={tdStyle} title={doc.document_id}>
                    {doc.title ?? doc.document_id}
                  </td>
                  <td style={tdStyle}>{doc.doc_type ?? EMDASH}</td>
                  <td style={tdNumeric}>{doc.evidence_total.toLocaleString()}</td>
                  <td style={tdNumeric}>
                    {doc.probative_connected.toLocaleString()}
                  </td>
                  <td style={tdStrong}>{formatRate(doc.probative_rate_pct)}</td>
                  <td style={tdNumeric}>
                    {doc.topical_connected.toLocaleString()}
                  </td>
                  <td style={tdNumeric}>{formatRate(doc.topical_rate_pct)}</td>
                  <td style={tdNumeric}>{doc.inert.toLocaleString()}</td>
                  <td style={tdNumeric}>{formatRate(doc.inert_rate_pct)}</td>
                  {edgeClasses.map((cls) => {
                    const count = edgeClassCount(doc, cls);
                    return (
                      <td key={cls} style={tdNumeric}>
                        {count === null ? EMDASH : count.toLocaleString()}
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

// ─── Node labels ─────────────────────────────────────────────────────────────

/** Node counts by populated label, plus the unlabeled tally when non-zero. */
export function NodeLabelTable({
  labels,
  unlabeledNodeCount,
}: {
  labels: LabelCount[];
  unlabeledNodeCount: number;
}) {
  return (
    <div style={sectionStyle}>
      <div style={sectionHeading}>Nodes by label</div>
      <div style={sectionNote}>
        Derived from the nodes themselves, so a label appears only if something
        carries it. Label names retained from earlier schema generations with zero
        nodes are not listed.
      </div>

      {unlabeledNodeCount > 0 && (
        <div style={noticeStyle}>
          {unlabeledNodeCount.toLocaleString()} node
          {unlabeledNodeCount === 1 ? " carries" : "s carry"} no label at all.
        </div>
      )}

      {labels.length === 0 ? (
        <div style={sectionNote}>No nodes in the graph.</div>
      ) : (
        <div style={scrollWrap}>
          <table style={tableStyle}>
            <thead>
              <tr>
                <th style={thStyle}>Label</th>
                <th style={thNumeric}>Nodes</th>
              </tr>
            </thead>
            <tbody>
              {labels.map((row) => (
                <tr key={row.label}>
                  <td style={tdStyle}>{row.label}</td>
                  <td style={tdStrong}>{row.node_count.toLocaleString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

// ─── Edge taxonomy ───────────────────────────────────────────────────────────

/**
 * Edge counts by (from-label, rel-type, to-label).
 *
 * The endpoints are the point: a count by relationship type alone cannot
 * distinguish `Evidence ABOUT Person` from `Evidence ABOUT Allegation`, and only
 * the second is a connection.
 */
export function EdgeTripleTable({ triples }: { triples: EdgeTripleCount[] }) {
  return (
    <div style={sectionStyle}>
      <div style={sectionHeading}>Edges by endpoint</div>
      <div style={sectionNote}>
        Every relationship in the graph, grouped by its source label,
        relationship type and target label.
      </div>

      {triples.length === 0 ? (
        <div style={sectionNote}>No relationships in the graph.</div>
      ) : (
        <div style={scrollWrap}>
          <table style={tableStyle}>
            <thead>
              <tr>
                <th style={thStyle}>From</th>
                <th style={thStyle}>Relationship</th>
                <th style={thStyle}>To</th>
                <th style={thNumeric}>Edges</th>
              </tr>
            </thead>
            <tbody>
              {triples.map((row) => (
                <tr
                  key={`${row.from_label ?? ""}|${row.rel_type}|${row.to_label ?? ""}`}
                >
                  <td style={tdStyle}>{row.from_label ?? "(unlabeled)"}</td>
                  <td style={tdStyle}>{row.rel_type}</td>
                  <td style={tdStyle}>{row.to_label ?? "(unlabeled)"}</td>
                  <td style={tdStrong}>{row.edge_count.toLocaleString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
