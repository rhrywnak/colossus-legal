// =============================================================================
// ElementRow.tsx — one selectable Element row (presentational, props-driven)
// -----------------------------------------------------------------------------
// Extracted VERBATIM from CountDetailPage's inline Elements list (PM3, Part 1 of
// the PM4 work) so the Proof Matrix page (PM4, Parts 2-3) can reuse the exact
// same row and extend it with columns, instead of duplicating it and letting the
// two copies drift.
//
// This component is deliberately dumb, matching the repo's generic presentational
// components (Breadcrumb, BurdenBadge, AuthorityPopover): it fetches nothing,
// owns no selection state, and does no Count-level math. The parent owns
// `selectedElementId` and passes `selected` + `onSelect` down — exactly the
// ownership PM3 had before extraction, so behavior is unchanged.
// =============================================================================

import React from "react";
import { ElementDetail } from "../services/causesOfAction";
import { formatElementNumber } from "./CountCard";

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
  /** Whether this row is the currently-selected Element. */
  selected: boolean;
  /** Fired on click or Enter/Space with this Element's stable id. */
  onSelect: (elementId: string) => void;
}

/**
 * Render one Element as a selectable `role="tab"` row: the "{count}.{order}"
 * number, the Element name, and the single allegation-count badge.
 *
 * ## React/TS Learning: a presentational component
 * It receives everything it needs via props and reports user intent back through
 * `onSelect` — it never mutates state itself. That keeps it trivially reusable
 * (PM3 today, PM4 next) and keeps the single source of truth for "which Element
 * is selected" in the parent page.
 */
const ElementRow: React.FC<ElementRowProps> = ({
  element,
  countNumber,
  index,
  selected,
  onSelect,
}) => {
  return (
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
};

// ─── Styles (moved verbatim from CountDetailPage; row-only) ──────────────────

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

export default ElementRow;
