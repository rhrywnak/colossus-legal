// =============================================================================
// ElementRow.tsx — one Element row, in two variants
// -----------------------------------------------------------------------------
// Originally extracted from PM3's Count-detail list (Part 1). Part 3 makes it
// serve BOTH pages from one definition, switched by an explicit `variant` prop:
//
//   - variant="legacy" (the DEFAULT) → PM3's row, byte-identical to Part 1:
//     a flex row of number + name + single allegation-count badge. PM3 passes no
//     variant, so it always gets this and is provably unchanged.
//   - variant="matrix" → PM4's five-column proof grid:
//     Element | Mapped Allegations | Supporting | Opposing | Status, sharing the
//     one column template in `proofMatrixColumns`, plus an expand caret.
//
// Branching on an EXPLICIT variant (not on the presence of optional data) keeps
// the mode obvious and the PM3 path trivially safe. The matrix-only props
// (evidence, status, expand) are optional and ignored by the legacy variant.
//
// Presentational only: it fetches nothing and owns no state. The parent decides
// selection/expansion and passes callbacks down.
// =============================================================================

import React from "react";
import { ElementDetail } from "../services/causesOfAction";
import { EvidenceRef, ElementProofStatus } from "../services/proofMatrix";
import { formatElementNumber } from "./CountCard";
import { PROOF_MATRIX_GRID_TEMPLATE } from "./proofMatrixColumns";
import EvidenceCell from "./EvidenceCell";
import StatusPill from "./StatusPill";

export interface ElementRowProps {
  /** The Element this row renders. Same shape the page row consumed inline. */
  element: ElementDetail;
  /**
   * The parent Count's ordinal, used to build the "{count}.{order}" number
   * label. Lives on the page (not on the Element), so it is passed in (Option A
   * of the Part-1 analysis) rather than derived here.
   */
  countNumber: number;
  /**
   * The row's position in the rendered list. Used ONLY as the fallback ordinal
   * when `element.order_in_count` is null — mirroring the page's original
   * `order_in_count ?? i + 1` expression exactly, so the displayed number does
   * not change.
   */
  index: number;
  /** Whether this row is the currently-selected Element (legacy highlight). */
  selected: boolean;
  /** Fired on click or Enter/Space with this Element's stable id. */
  onSelect: (elementId: string) => void;

  // ── Matrix-variant props (optional; the legacy variant ignores them) ────────
  /** 'legacy' (default) = PM3's single-badge row; 'matrix' = PM4's 5 columns. */
  variant?: "legacy" | "matrix";
  /**
   * Supporting column magnitude — the backend's `supporting_evidence_count`
   * (DISTINCT corroborating Evidence). The column shows this count; the
   * per-evidence chips live in the expanded detail, not the column.
   */
  supportingCount?: number;
  /** Opposing evidence refs; empty today (no CONTRADICTS/REBUTS edges yet). */
  opposingEvidence?: EvidenceRef[];
  /** Backend-derived proof status; rendered as-is, never re-computed (Rule 19). */
  proofStatus?: ElementProofStatus;
  /** Whether this matrix row is expanded (drives the caret + highlight). */
  expanded?: boolean;
  /** When provided, a matrix-row click toggles expansion instead of selecting. */
  onToggleExpand?: (elementId: string) => void;
}

/**
 * Legacy (PM3) row — the Part-1 markup, unchanged. Highlight and click both key
 * off `selected` / `onSelect` exactly as before.
 */
const LegacyRow: React.FC<ElementRowProps> = ({
  element,
  countNumber,
  index,
  selected,
  onSelect,
}) => (
  <div
    role="tab"
    tabIndex={0}
    aria-selected={selected}
    onClick={() => onSelect(element.element_id)}
    onKeyDown={(e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        onSelect(element.element_id);
      }
    }}
    style={{
      ...ELEMENT_ROW_STYLE,
      backgroundColor: selected ? "var(--accent-bg-soft)" : "transparent",
      borderLeft: selected
        ? "3px solid var(--accent-primary)"
        : "3px solid transparent",
    }}
  >
    <span style={ELEMENT_NUMBER_STYLE}>
      {formatElementNumber(countNumber, element.order_in_count ?? index + 1)}
    </span>
    <span style={ELEMENT_NAME_STYLE}>{element.element_name}</span>
    <span style={element.allegation_count > 0 ? BADGE_STYLE : ZERO_BADGE_STYLE}>
      {element.allegation_count}
    </span>
  </div>
);

/**
 * The matrix row's lead ("Element") cell: expand caret + number + name. Extracted
 * so `MatrixRow` stays within the 50-line limit. Reuses the legacy number/name
 * styles so the typography matches PM3's row exactly.
 */
const ElementLeadCell: React.FC<{
  element: ElementDetail;
  countNumber: number;
  index: number;
  expanded?: boolean;
}> = ({ element, countNumber, index, expanded }) => (
  <span style={ELEMENT_LEAD_STYLE}>
    <span style={CARET_STYLE}>{expanded ? "▾" : "▸"}</span>
    <span style={ELEMENT_NUMBER_STYLE}>
      {formatElementNumber(countNumber, element.order_in_count ?? index + 1)}
    </span>
    <span style={ELEMENT_NAME_STYLE}>{element.element_name}</span>
  </span>
);

/**
 * Matrix (PM4) row — five columns on the shared grid template. The row is
 * interactive: a click toggles expansion via `onToggleExpand` (falling back to
 * `onSelect` if none is given). Highlight reflects `selected || expanded`.
 */
const MatrixRow: React.FC<ElementRowProps> = (props) => {
  const { element, selected, expanded, onSelect, onToggleExpand } = props;
  const highlighted = selected || !!expanded;
  const activate = () =>
    onToggleExpand ? onToggleExpand(element.element_id) : onSelect(element.element_id);
  return (
    <div
      role="tab"
      tabIndex={0}
      aria-selected={highlighted}
      aria-expanded={!!expanded}
      onClick={activate}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          activate();
        }
      }}
      style={{
        ...MATRIX_ROW_STYLE,
        backgroundColor: highlighted ? "var(--accent-bg-soft)" : "transparent",
        borderLeft: highlighted
          ? "3px solid var(--accent-primary)"
          : "3px solid transparent",
      }}
    >
      <ElementLeadCell
        element={element}
        countNumber={props.countNumber}
        index={props.index}
        expanded={expanded}
      />
      <span style={element.allegation_count > 0 ? BADGE_STYLE : ZERO_BADGE_STYLE}>
        {element.allegation_count}
      </span>
      <SupportingCountCell count={props.supportingCount ?? 0} />
      <EvidenceCell items={props.opposingEvidence ?? []} />
      <StatusPill status={props.proofStatus ?? "no_allegations"} />
    </div>
  );
};

/**
 * Supporting column cell: the corroborating-evidence magnitude. Renders the
 * number when > 0, or the muted "—" empty treatment (matching `EvidenceCell`'s
 * empty state) when 0. It only DISPLAYS the backend count — no derivation.
 */
const SupportingCountCell: React.FC<{ count: number }> = ({ count }) =>
  count > 0 ? (
    <span style={SUPPORTING_COUNT_STYLE}>{count}</span>
  ) : (
    <span style={SUPPORTING_EMPTY_STYLE} title="No supporting evidence">
      —
    </span>
  );

/**
 * ElementRow — switch on the explicit `variant` (default 'legacy'). PM3 passes
 * no variant → LegacyRow → identical to today; PM4 passes 'matrix' → MatrixRow.
 */
const ElementRow: React.FC<ElementRowProps> = (props) =>
  props.variant === "matrix" ? <MatrixRow {...props} /> : <LegacyRow {...props} />;

// ─── Legacy row styles (moved verbatim from CountDetailPage; row-only) ───────

const ELEMENT_ROW_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "12px",
  padding: "10px 12px",
  cursor: "pointer",
  borderRadius: "6px",
};

const ELEMENT_NUMBER_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: "13px",
  fontWeight: 700,
  color: "var(--text-secondary)",
  minWidth: "32px",
};

const ELEMENT_NAME_STYLE: React.CSSProperties = {
  flex: 1,
  fontFamily: "var(--font-sans)",
  fontSize: "14px",
  fontWeight: 500,
  color: "var(--text-primary)",
};

const BADGE_STYLE: React.CSSProperties = {
  display: "inline-block",
  padding: "2px 10px",
  borderRadius: "12px",
  backgroundColor: "var(--accent-bg-soft)",
  color: "var(--accent-primary)",
  fontSize: "13px",
  fontWeight: 600,
};

const ZERO_BADGE_STYLE: React.CSSProperties = {
  color: "var(--text-muted)",
  fontSize: "13px",
};

// Supporting-count cell: a plain numeric magnitude (text, not an accent pill, so
// it reads as a count and not as the allegation badge), and the muted "—" empty
// treatment matching EvidenceCell.
const SUPPORTING_COUNT_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "13px",
  fontWeight: 600,
  color: "var(--text-primary)",
};

const SUPPORTING_EMPTY_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "13px",
  color: "var(--text-muted)",
};

// ─── Matrix row styles (PM4 only) ────────────────────────────────────────────

// Same vertical rhythm as the legacy row (padding/gap/radius), but a CSS grid
// on the shared template so the cells line up with the PM4 column header.
const MATRIX_ROW_STYLE: React.CSSProperties = {
  display: "grid",
  gridTemplateColumns: PROOF_MATRIX_GRID_TEMPLATE,
  alignItems: "center",
  gap: "12px",
  padding: "10px 12px",
  cursor: "pointer",
  borderRadius: "6px",
};

// The "Element" lead cell: caret + number + name in a flex group. `minWidth: 0`
// lets the name shrink/wrap within the grid column instead of overflowing.
const ELEMENT_LEAD_STYLE: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "8px",
  minWidth: 0,
};

const CARET_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: "11px",
  color: "var(--text-muted)",
};

export default ElementRow;
