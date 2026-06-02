// =============================================================================
// ElementAllegationList.tsx — one labeled section of mapped Allegations, each
// with its corroborating Evidence (Proof Matrix expand, Part 2)
// -----------------------------------------------------------------------------
// Extracted from ElementDetailContent.tsx: that file was already over the
// 300-line module limit before Part 2, and nesting per-allegation evidence under
// each card would push it further. This sibling owns the allegation-card +
// evidence rendering and its styles; ElementDetailContent keeps the fetch, the
// notes editor, and the Common/Dedicated grouping.
//
// Per allegation we render its `supporting_evidence` (the backend's
// CORROBORATES-derived list): each item's verbatim quote, interrogatory id, and
// a source locator. An EMPTY array renders an explicit, muted "No supporting
// evidence" row — the per-allegation gap made visible (Rule 1). This is a render
// conditional on a backend-provided array, NOT status derivation (Rule 19).
//
// Source-PDF click-through reuses the app's existing document-file pattern
// (`/api/documents/:id/file#page=N`, as in AnswerDisplay / GraphPage). When an
// Evidence node has no source document (`source_document_id` null — a gap the
// backend warn-logs), the locator renders as text with no link.
// =============================================================================

import React from "react";
import {
  AllegationSummary,
  SupportingEvidence,
} from "../services/elementDetailService";
import { API_BASE_URL } from "../services/api";

// ─── Source-PDF locator helpers (pure) ───────────────────────────────────────

/**
 * Build the existing document-file URL with an optional `#page=N` fragment —
 * the same pattern AnswerDisplay/GraphPage use. We do not invent a viewer.
 */
export function pdfHref(documentId: string, page: number | null): string {
  const fragment = page !== null ? `#page=${page}` : "";
  return `${API_BASE_URL}/api/documents/${encodeURIComponent(documentId)}/file${fragment}`;
}

/** Human locator text: "{title} · p. {n}" (title alone when no page). */
export function locatorLabel(ev: SupportingEvidence): string {
  const title = ev.source_document_title ?? "Source document";
  return ev.page_number !== null ? `${title} · p. ${ev.page_number}` : title;
}

// ─── Evidence rendering ──────────────────────────────────────────────────────

/**
 * The source locator for one Evidence item: a click-through link to the source
 * PDF page when the document id is known, or plain text when it is null (the
 * data-gap state — no dead link).
 */
const EvidenceLocator: React.FC<{ ev: SupportingEvidence }> = ({ ev }) => {
  const label = locatorLabel(ev);
  if (ev.source_document_id) {
    return (
      <a
        href={pdfHref(ev.source_document_id, ev.page_number)}
        target="_blank"
        rel="noopener noreferrer"
        style={LOCATOR_LINK_STYLE}
      >
        {label}
      </a>
    );
  }
  return (
    <span style={LOCATOR_TEXT_STYLE} title="Source document unavailable">
      {label}
    </span>
  );
};

/**
 * An allegation's supporting evidence, or the explicit empty-gap row. An empty
 * array is the visible gap and must be obvious, not blank.
 */
const SupportingEvidenceList: React.FC<{ evidence: SupportingEvidence[] }> = ({
  evidence,
}) => {
  if (evidence.length === 0) {
    return <div style={NO_EVIDENCE_STYLE}>No supporting evidence</div>;
  }
  return (
    <div style={EVIDENCE_LIST_STYLE}>
      {evidence.map((ev) => (
        <div key={ev.id} style={EVIDENCE_ITEM_STYLE}>
          {ev.verbatim_quote && (
            <div className="proof-text" style={EVIDENCE_QUOTE_STYLE}>
              “{ev.verbatim_quote}”
            </div>
          )}
          <div style={EVIDENCE_META_STYLE}>
            {ev.paragraph && (
              <span style={EVIDENCE_PARA_STYLE}>{ev.paragraph}</span>
            )}
            <EvidenceLocator ev={ev} />
          </div>
        </div>
      ))}
    </div>
  );
};

// ─── Allegation section ──────────────────────────────────────────────────────

export interface AllegationSectionProps {
  label: string;
  labelColor: string;
  labelBg: string;
  accentColor: string;
  allegations: AllegationSummary[];
}

/**
 * One labeled group of allegation cards (Common / Count-specific / Other). Moved
 * verbatim from ElementDetailContent and extended with the nested
 * `SupportingEvidenceList` under each card.
 */
const AllegationSection: React.FC<AllegationSectionProps> = ({
  label,
  labelColor,
  labelBg,
  accentColor,
  allegations,
}) => (
  <div>
    <div style={SECTION_DIVIDER_STYLE_BASE}>
      <span
        style={{
          color: labelColor,
          backgroundColor: labelBg,
          padding: "2px 8px",
          borderRadius: "12px",
        }}
      >
        {label}
      </span>
      <span style={SECTION_DIVIDER_RULE_STYLE} />
    </div>
    {allegations.map((a) => (
      <div key={a.allegation_id} style={ALLEGATION_CARD_STYLE}>
        <div>
          <span style={PARAGRAPH_LABEL_STYLE}>¶{a.paragraph_number}</span>
          {a.summary && <span style={SUMMARY_TEXT_STYLE}>{a.summary}</span>}
        </div>
        {a.verbatim_quote && (
          <div
            className="proof-text"
            style={{ ...QUOTE_TEXT_STYLE_BASE, borderLeft: `3px solid ${accentColor}` }}
          >
            {a.verbatim_quote}
          </div>
        )}
        <SupportingEvidenceList evidence={a.supporting_evidence} />
      </div>
    ))}
  </div>
);

export default AllegationSection;

// ─── Styles (allegation card — moved from ElementDetailContent) ──────────────

const SECTION_DIVIDER_STYLE_BASE: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "8px",
  margin: "12px 0 8px",
  fontFamily: "var(--font-sans)",
  fontSize: "11px",
  fontWeight: 700,
  letterSpacing: "0.05em",
  textTransform: "uppercase",
};

const SECTION_DIVIDER_RULE_STYLE: React.CSSProperties = {
  flex: 1,
  height: "1px",
  backgroundColor: "var(--border-default)",
};

const ALLEGATION_CARD_STYLE: React.CSSProperties = {
  padding: "10px 12px",
  marginBottom: "8px",
  border: "1px solid var(--border-default)",
  borderRadius: "8px",
  backgroundColor: "var(--bg-surface)",
};

const PARAGRAPH_LABEL_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: "15px",
  fontWeight: 700,
  color: "var(--text-primary)",
};

const SUMMARY_TEXT_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "13px",
  color: "var(--text-secondary)",
  marginLeft: "8px",
};

// Layout-only; typography comes from the `.proof-text` utility class in
// tokens.css (the canonical proof/body treatment).
const QUOTE_TEXT_STYLE_BASE: React.CSSProperties = {
  marginTop: "6px",
  paddingLeft: "8px",
  lineHeight: 1.45,
};

// ─── Styles (supporting evidence — new) ──────────────────────────────────────

// Evidence nests under the allegation, set off by a top rule so it reads as
// "what backs this allegation" rather than part of the allegation text.
const EVIDENCE_LIST_STYLE: React.CSSProperties = {
  marginTop: "8px",
  paddingTop: "8px",
  borderTop: "1px dashed var(--border-default)",
  display: "flex",
  flexDirection: "column",
  gap: "8px",
};

const EVIDENCE_ITEM_STYLE: React.CSSProperties = {
  paddingLeft: "8px",
  borderLeft: "3px solid var(--state-success-strong)",
};

const EVIDENCE_QUOTE_STYLE: React.CSSProperties = {
  lineHeight: 1.45,
};

const EVIDENCE_META_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "baseline",
  flexWrap: "wrap",
  gap: "8px",
  marginTop: "4px",
};

const EVIDENCE_PARA_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: "12px",
  fontWeight: 700,
  color: "var(--text-secondary)",
};

const LOCATOR_LINK_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "12px",
  color: "var(--accent-primary)",
  textDecoration: "none",
};

const LOCATOR_TEXT_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "12px",
  color: "var(--text-muted)",
};

// The explicit per-allegation gap — muted but obviously present, never blank.
const NO_EVIDENCE_STYLE: React.CSSProperties = {
  marginTop: "8px",
  paddingTop: "8px",
  borderTop: "1px dashed var(--border-default)",
  fontFamily: "var(--font-sans)",
  fontSize: "12px",
  fontStyle: "italic",
  color: "var(--text-muted)",
};
