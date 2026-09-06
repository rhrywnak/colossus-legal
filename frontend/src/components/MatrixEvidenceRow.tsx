// =============================================================================
// MatrixEvidenceRow.tsx — one piece of evidence under one accusation
// -----------------------------------------------------------------------------
// PROOF_MATRIX_v2 §3. Split out of ElementAllegationList when the row grew from
// "a quote and a locator" into the reader-facing thing the instruction
// describes: the quote (in RFA form where the card is a Q&A), a grey line under
// it carrying the reason, the confidence, who says it belongs here and whether
// it conflicts — and, on the right, the two buttons that let Roman overrule any
// of it.
//
// Presentational. It fetches nothing and owns no state: the parent owns the
// optimistic update, because the parent owns the list the row has to move
// within.
//
// ## Every visible word is served
//
// There is not one user-facing literal in this file. The labels, the marks and
// the two button captions all come from `MatrixWording`; the layout constants
// are CSS tokens. That is the standing law for reader-facing pages, and this is
// the page it was written for.
// =============================================================================

import React from "react";
import type { AllegationEvidence } from "../services/elementDetailService";
import type { MatrixWording } from "../services/causesOfAction";
import type { MatrixRulingToken } from "../services/matrixRulings";
import { locatorLabel, pdfHref } from "./evidenceLocator";
import { duplicateMarker } from "./matrixStrength";
import { authorMark, confidenceLabel, quoteText, reasonLine } from "./matrixRow";

export interface MatrixEvidenceRowProps {
  item: AllegationEvidence;
  /** Left rule colour: green for support, red for a rebuttal. */
  accent: string;
  wording: MatrixWording;
  /** Fired by Keep / Remove. The parent writes and moves the row. */
  onRule: (evidenceId: string, ruling: MatrixRulingToken) => void;
  /** Fired by Undo, beside a hidden item. */
  onUndo: (evidenceId: string) => void;
  /** Whether this row is being written right now — the buttons go inert. */
  pending: boolean;
}

/** The source locator: a link when the document id is known, plain text if not. */
const EvidenceLocator: React.FC<{ ev: AllegationEvidence }> = ({ ev }) => {
  const label = locatorLabel(ev);
  if (ev.source_document_id === null) {
    return <span style={LOCATOR_TEXT_STYLE}>{label}</span>;
  }
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
};

/**
 * The grey line under a quote: the reason, the confidence, who says it belongs,
 * the conflict mark, and the "×N".
 *
 * All of it on ONE line, in that order, because a reader scanning a paragraph of
 * five items should be able to tell a confirmed admission from an unread
 * context note without reading either quote.
 */
const MetaLine: React.FC<{ item: AllegationEvidence; wording: MatrixWording }> = ({
  item,
  wording,
}) => {
  const reason = reasonLine(item);
  const confidence = confidenceLabel(item.confidence, wording);
  const author = authorMark(item, wording);
  const marker = duplicateMarker(item.occurrences, wording);
  return (
    <div style={META_LINE_STYLE}>
      {reason && <span>{reason}</span>}
      {confidence && <span style={META_MARK_STYLE}>{confidence}</span>}
      <span style={author.confirmed ? CONFIRMED_MARK_STYLE : META_MARK_STYLE}>
        {author.label}
      </span>
      {item.conflict && (
        <span style={CONFLICT_MARK_STYLE}>{wording.conflict_label}</span>
      )}
      {marker && <span style={META_MARK_STYLE}>{marker}</span>}
      <EvidenceLocator ev={item} />
    </div>
  );
};

/**
 * The controls on the right of a row: Keep and Remove on a visible item, Undo on
 * a hidden one.
 *
 * ## Why a hidden item offers Undo and not Keep
 *
 * Undo is the reversal of the act that hid it, and it is the only word that
 * means that. A hidden item ALSO accepts Keep — pressing Keep on something the
 * machine retracted is how a human overrules `does_not_belong` — so both are
 * offered, with Undo first because it is what a reader who just pressed Remove
 * is looking for.
 */
const RowControls: React.FC<{
  item: AllegationEvidence;
  wording: MatrixWording;
  onRule: (evidenceId: string, ruling: MatrixRulingToken) => void;
  onUndo: (evidenceId: string) => void;
  pending: boolean;
}> = ({ item, wording, onRule, onUndo, pending }) => (
  <div style={CONTROLS_STYLE}>
    {item.hidden_reason !== null && item.ruling === "remove" && (
      <button
        type="button"
        style={BUTTON_STYLE}
        disabled={pending}
        onClick={() => onUndo(item.id)}
      >
        {wording.undo_label}
      </button>
    )}
    {item.ruling !== "keep" && (
      <button
        type="button"
        style={BUTTON_STYLE}
        disabled={pending}
        onClick={() => onRule(item.id, "keep")}
      >
        {wording.keep_label}
      </button>
    )}
    {item.hidden_reason === null && (
      <button
        type="button"
        style={BUTTON_STYLE}
        disabled={pending}
        onClick={() => onRule(item.id, "remove")}
      >
        {wording.remove_label}
      </button>
    )}
  </div>
);

/**
 * One evidence row. A hidden item renders greyed, in the same shape — a reader
 * who reveals the hidden group should recognise what they are looking at.
 */
const MatrixEvidenceRow: React.FC<MatrixEvidenceRowProps> = ({
  item,
  accent,
  wording,
  onRule,
  onUndo,
  pending,
}) => {
  const text = quoteText(item);
  const hidden = item.hidden_reason !== null;
  return (
    <div
      style={{
        ...ROW_STYLE,
        borderLeft: `3px solid ${accent}`,
        opacity: hidden ? 0.55 : 1,
      }}
    >
      <div style={{ minWidth: 0, flex: 1 }}>
        {/* A row with no words at all is a real state — an Evidence node whose
            quote never grounded. It renders as its meta line alone rather than
            as blank space, so the gap is visible and the item is still rulable. */}
        {text && (
          <div className="proof-text" style={QUOTE_STYLE}>
            {text}
          </div>
        )}
        <MetaLine item={item} wording={wording} />
      </div>
      <RowControls
        item={item}
        wording={wording}
        onRule={onRule}
        onUndo={onUndo}
        pending={pending}
      />
    </div>
  );
};

export default MatrixEvidenceRow;

// ─── Styles (tokens only) ────────────────────────────────────────────────────

const ROW_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "flex-start",
  gap: "12px",
  paddingLeft: "8px",
};

const QUOTE_STYLE: React.CSSProperties = {
  lineHeight: 1.45,
};

const META_LINE_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "baseline",
  flexWrap: "wrap",
  gap: "8px",
  marginTop: "4px",
  fontFamily: "var(--font-sans)",
  fontSize: "11.5px",
  color: "var(--text-muted)",
};

const META_MARK_STYLE: React.CSSProperties = {
  color: "var(--text-muted)",
};

// The one green mark on the page: a human has been through this item. It is the
// single most important distinction the row draws, so it gets the colour.
const CONFIRMED_MARK_STYLE: React.CSSProperties = {
  color: "var(--state-success-strong)",
  fontWeight: 600,
};

// Amber, not red: a conflict is something to look at, not something that is
// wrong. Red on this page already means "disputes".
const CONFLICT_MARK_STYLE: React.CSSProperties = {
  color: "var(--burden-warning-text)",
  fontWeight: 600,
};

const LOCATOR_LINK_STYLE: React.CSSProperties = {
  color: "var(--accent-primary)",
  textDecoration: "none",
};

const LOCATOR_TEXT_STYLE: React.CSSProperties = {
  color: "var(--text-muted)",
};

const CONTROLS_STYLE: React.CSSProperties = {
  display: "flex",
  gap: "6px",
  flexShrink: 0,
};

// A control, not a chip: bordered, and sized so the two sit comfortably at the
// end of a line a reader is scanning rather than reading.
const BUTTON_STYLE: React.CSSProperties = {
  padding: "2px 10px",
  border: "1px solid var(--border-default)",
  borderRadius: "6px",
  backgroundColor: "var(--bg-surface)",
  color: "var(--text-secondary)",
  fontFamily: "var(--font-sans)",
  fontSize: "11.5px",
  fontWeight: 600,
  cursor: "pointer",
};
