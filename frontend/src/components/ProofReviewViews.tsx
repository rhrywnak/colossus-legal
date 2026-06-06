// =============================================================================
// ProofReviewViews.tsx — presentational sub-views for the Proof Review page
// -----------------------------------------------------------------------------
// Pure presentational components (props in → JSX out): no fetch, no state, no
// business logic. They render the labeled rows the PR1 payload returns. Kept in
// their own file so ProofReviewPage.tsx stays a thin orchestrator (fetch + tabs
// + filters) over these + the tested helpers, and neither file exceeds the
// module-size limit.
//
// Nullable fields render an explicit em-dash fallback rather than a blank, so a
// missing value is visible (Charter §8 honesty rule), never silently empty.
// =============================================================================

import React from "react";
import type {
  CategoryCount,
  ExcludedEvidence,
  ProofEdge,
  ProofReviewSummary,
  StatementTypeCount,
} from "../services/proofReview";

const EMDASH = "—";

// ─── Styles (design tokens only) ─────────────────────────────────────────────

const cardStyle: React.CSSProperties = {
  border: "1px solid var(--border-default)",
  backgroundColor: "var(--bg-surface)",
  borderRadius: "8px",
  padding: "14px 16px",
  marginBottom: "12px",
};

const labelStyle: React.CSSProperties = {
  fontSize: "0.72rem",
  fontWeight: 600,
  letterSpacing: "0.04em",
  textTransform: "uppercase",
  color: "var(--text-muted)",
  marginBottom: "2px",
};

const answerTextStyle: React.CSSProperties = {
  fontSize: "0.9rem",
  color: "var(--text-primary)",
  lineHeight: 1.5,
};

const allegationBoxStyle: React.CSSProperties = {
  marginTop: "10px",
  paddingTop: "10px",
  borderTop: "1px dashed var(--border-default)",
};

const chipRowStyle: React.CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  gap: "0.4rem",
  alignItems: "center",
  marginTop: "8px",
};

const chipStyle: React.CSSProperties = {
  display: "inline-block",
  padding: "0.15rem 0.5rem",
  borderRadius: "9999px",
  fontSize: "0.72rem",
  fontWeight: 600,
  backgroundColor: "var(--bg-page)",
  color: "var(--text-secondary)",
  border: "1px solid var(--border-default)",
};

const locatorStyle: React.CSSProperties = {
  fontSize: "0.76rem",
  color: "var(--text-muted)",
  fontFamily: "var(--font-mono, monospace)",
};

const emptyStyle: React.CSSProperties = {
  padding: "2rem",
  textAlign: "center",
  color: "var(--text-muted)",
  fontSize: "0.9rem",
  border: "1px dashed var(--border-default)",
  borderRadius: "8px",
};

const summaryGroupStyle: React.CSSProperties = {
  ...cardStyle,
  padding: "16px 18px",
};

const summaryTotalStyle: React.CSSProperties = {
  fontSize: "1.6rem",
  fontWeight: 700,
  color: "var(--text-primary)",
};

const countRowStyle: React.CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  padding: "0.3rem 0",
  borderBottom: "1px solid var(--bg-page)",
  fontSize: "0.84rem",
  color: "var(--text-secondary)",
};

// ─── Small shared pieces ─────────────────────────────────────────────────────

/** An explicit empty-state panel — never a blank region (Charter §8). */
export const EmptyState: React.FC<{ message: string }> = ({ message }) => (
  <div style={emptyStyle}>{message}</div>
);

/** A rounded category chip (statement_type / evidence_strength). */
const Chip: React.FC<{ text: string }> = ({ text }) => (
  <span style={chipStyle}>{text}</span>
);

/**
 * The source locator as TEXT only (NOT a deep link in v1): source document ·
 * Q-number (`paragraph`) · page. `source_document` is not confirmed to be a
 * usable document id, so no `/api/documents/:id/file` link is wired this round.
 */
const Locator: React.FC<{
  sourceDocument: string | null;
  paragraph: string | null;
  pageNumber: number | null;
}> = ({ sourceDocument, paragraph, pageNumber }) => {
  const parts: string[] = [];
  if (sourceDocument) parts.push(sourceDocument);
  if (paragraph) parts.push(paragraph);
  if (pageNumber !== null) parts.push(`p.${pageNumber}`);
  return <div style={locatorStyle}>{parts.length ? parts.join("  ·  ") : EMDASH}</div>;
};

/** The answer side (Evidence) — question + answer text + category chips + locator. */
const AnswerBlock: React.FC<{
  question: string | null;
  answer: string | null;
  verbatim: string | null;
  statementType: string;
  evidenceStrength?: string | null;
  sourceDocument: string | null;
  paragraph: string | null;
  pageNumber: number | null;
}> = ({
  question,
  answer,
  verbatim,
  statementType,
  evidenceStrength,
  sourceDocument,
  paragraph,
  pageNumber,
}) => (
  <div>
    {question && (
      <>
        <div style={labelStyle}>Question</div>
        <div style={{ ...answerTextStyle, color: "var(--text-secondary)", marginBottom: "6px" }}>
          {question}
        </div>
      </>
    )}
    <div style={labelStyle}>Answer</div>
    <div style={answerTextStyle}>{answer ?? verbatim ?? EMDASH}</div>
    <div style={chipRowStyle}>
      <Chip text={statementType} />
      {evidenceStrength ? <Chip text={evidenceStrength} /> : null}
      <Locator sourceDocument={sourceDocument} paragraph={paragraph} pageNumber={pageNumber} />
    </div>
  </div>
);

// ─── Proof edge / borderline card (shared shape) ─────────────────────────────

/**
 * One proof edge: the answer side and the complaint allegation it corroborates.
 * Used by both the Proof-edges and Borderline tabs (borderline is the
 * `partial_admission` subset — identical row shape).
 */
export const ProofEdgeCard: React.FC<{ edge: ProofEdge }> = ({ edge }) => (
  <div style={cardStyle}>
    <AnswerBlock
      question={edge.question}
      answer={edge.answer}
      verbatim={edge.evidence_verbatim_quote}
      statementType={edge.statement_type}
      evidenceStrength={edge.evidence_strength}
      sourceDocument={edge.source_document}
      paragraph={edge.paragraph}
      pageNumber={edge.page_number}
    />
    <div style={allegationBoxStyle}>
      <div style={labelStyle}>Corroborates allegation</div>
      <div style={{ ...answerTextStyle, fontWeight: 600 }}>
        {edge.allegation_title ?? EMDASH}
      </div>
      <div style={{ ...answerTextStyle, color: "var(--text-secondary)" }}>
        {edge.allegation_summary ?? EMDASH}
      </div>
      <div style={locatorStyle}>
        {edge.allegation_paragraph_number ? `¶ ${edge.allegation_paragraph_number}` : EMDASH}
      </div>
    </div>
  </div>
);

// ─── Excluded card (answer side only — no allegation) ────────────────────────

/** One preserved non-answer that produced no corroboration edge. */
export const ExcludedCard: React.FC<{ row: ExcludedEvidence }> = ({ row }) => (
  <div style={cardStyle}>
    <AnswerBlock
      question={row.question}
      answer={row.answer}
      verbatim={row.evidence_verbatim_quote}
      statementType={row.statement_type}
      sourceDocument={row.source_document}
      paragraph={row.paragraph}
      pageNumber={row.page_number}
    />
  </div>
);

// ─── Summary section ─────────────────────────────────────────────────────────

const CountList: React.FC<{ rows: StatementTypeCount[] }> = ({ rows }) => (
  <div style={{ marginTop: "10px" }}>
    {rows.map((r) => (
      <div key={r.statement_type} style={countRowStyle}>
        <span>{r.statement_type}</span>
        <span style={{ fontWeight: 600 }}>{r.count}</span>
      </div>
    ))}
  </div>
);

const CategoryList: React.FC<{ rows: CategoryCount[] }> = ({ rows }) => (
  <div style={{ marginTop: "10px" }}>
    {rows.map((r) => (
      <div key={`${r.statement_type}/${r.evidence_strength}`} style={countRowStyle}>
        <span>
          {r.statement_type}
          <span style={{ color: "var(--text-muted)" }}> · {r.evidence_strength}</span>
        </span>
        <span style={{ fontWeight: 600 }}>{r.count}</span>
      </div>
    ))}
  </div>
);

/**
 * The Summary sub-view: the corroboration and exclusion breakdowns the backend
 * derived. Plain labeled counts — the "is my count honest" view. Renders exactly
 * what PR1 returns; no recomputation here.
 */
export const SummarySection: React.FC<{ summary: ProofReviewSummary }> = ({ summary }) => (
  <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "16px" }}>
    <div style={summaryGroupStyle}>
      <div style={labelStyle}>Corroborating</div>
      <div style={summaryTotalStyle}>{summary.corroborating.total}</div>
      <div style={{ fontSize: "0.78rem", color: "var(--text-muted)", marginTop: "8px" }}>
        By statement type
      </div>
      <CountList rows={summary.corroborating.by_statement_type} />
      <div style={{ fontSize: "0.78rem", color: "var(--text-muted)", marginTop: "12px" }}>
        By category (statement type · evidence strength)
      </div>
      <CategoryList rows={summary.corroborating.by_category} />
    </div>
    <div style={summaryGroupStyle}>
      <div style={labelStyle}>Excluded (non-answers, preserved)</div>
      <div style={summaryTotalStyle}>{summary.excluded.total}</div>
      <div style={{ fontSize: "0.78rem", color: "var(--text-muted)", marginTop: "8px" }}>
        By statement type
      </div>
      <CountList rows={summary.excluded.by_statement_type} />
    </div>
  </div>
);
