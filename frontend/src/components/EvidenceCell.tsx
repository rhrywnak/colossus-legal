// =============================================================================
// EvidenceCell.tsx — one Supporting/Opposing evidence column cell (PM4)
// -----------------------------------------------------------------------------
// Honest-empty by design (Charter §8): there is NO evidence data yet (discovery
// unprocessed, zero Evidence nodes), so today this always receives an empty array
// and renders a muted "—" ("Discovery pending"). The non-empty rendering
// (evidence chips) is built now so Stage 2 only swaps the data source — never a
// fake count, never a fake document chip in the meantime.
// =============================================================================

import React from "react";
import { EvidenceRef } from "../services/proofMatrix";

export interface EvidenceCellProps {
  /** Evidence refs for this cell. Empty today (honest pending state). */
  items: EvidenceRef[];
}

/**
 * Render a list of evidence refs, or the honest empty state when there are none.
 * The empty branch is the only one reachable in v1.
 */
const EvidenceCell: React.FC<EvidenceCellProps> = ({ items }) => {
  if (items.length === 0) {
    // Honest empty: discovery is not processed yet. Muted `—`, matching the
    // pending-rollup `—` elsewhere — never a fabricated value.
    return (
      <span style={EMPTY_STYLE} title="Discovery pending">
        —
      </span>
    );
  }
  return (
    <div style={LIST_STYLE}>
      {items.map((ref) => (
        <span key={ref.id} style={CHIP_STYLE} title={`${ref.label} · p.${ref.page}`}>
          {ref.label}
        </span>
      ))}
    </div>
  );
};

const EMPTY_STYLE: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: "13px",
  color: "var(--text-muted)",
};

const LIST_STYLE: React.CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  gap: "4px",
};

const CHIP_STYLE: React.CSSProperties = {
  display: "inline-block",
  padding: "2px 8px",
  borderRadius: "10px",
  backgroundColor: "var(--burden-neutral-bg)",
  color: "var(--text-secondary)",
  fontFamily: "var(--font-sans)",
  fontSize: "12px",
  fontWeight: 500,
};

export default EvidenceCell;
