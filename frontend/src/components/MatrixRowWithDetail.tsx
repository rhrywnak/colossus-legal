// =============================================================================
// MatrixRowWithDetail.tsx — one PM4 matrix row + its expandable detail
// -----------------------------------------------------------------------------
// Pairs a matrix-variant `ElementRow` with the live mapped-allegation detail
// that drops in below it when the row is expanded. Extracted from
// `ProofMatrixPage` so that page stays within the 300-line module limit and so
// the row+detail unit reads as one thing.
//
// The evidence columns are honest empties (Charter §8): `supportingEvidence` and
// `opposingEvidence` are `[]` and `proofStatus` is `'pending'`, because no
// evidence data exists yet. `ElementDetailContent` self-fetches the real
// allegation detail and is mounted unchanged.
// =============================================================================

import React from "react";
import { ElementDetail } from "../services/causesOfAction";
import ElementRow from "./ElementRow";
import ElementDetailContent from "./ElementDetailContent";

/**
 * Matrix rows interact via `onToggleExpand`, not `onSelect` (PM4 has no separate
 * "select" concept). ElementRow still requires `onSelect`, so we pass this shared
 * no-op — a single stable reference, never invoked while `onToggleExpand` is
 * supplied. It exists only to satisfy the prop contract, not to swallow an
 * action (Rule 1).
 */
const NOOP_SELECT = (): void => {};

export interface MatrixRowWithDetailProps {
  element: ElementDetail;
  /** The parent Count's ordinal, for the "{count}.{order}" number label. */
  countNumber: number;
  /** Row position, used only as the `order_in_count` fallback ordinal. */
  index: number;
  /** Case slug for the expanded detail's self-fetch. */
  caseSlug: string;
  /** Whether this row is currently expanded. */
  expanded: boolean;
  /** Toggle this row's expansion. */
  onToggleExpand: (elementId: string) => void;
}

const MatrixRowWithDetail: React.FC<MatrixRowWithDetailProps> = ({
  element,
  countNumber,
  index,
  caseSlug,
  expanded,
  onToggleExpand,
}) => (
  <>
    <ElementRow
      element={element}
      countNumber={countNumber}
      index={index}
      selected={false}
      onSelect={NOOP_SELECT}
      variant="matrix"
      supportingEvidence={[]}
      opposingEvidence={[]}
      proofStatus="pending"
      expanded={expanded}
      onToggleExpand={onToggleExpand}
    />
    {expanded && (
      <div style={EXPAND_STYLE}>
        <ElementDetailContent
          caseSlug={caseSlug}
          elementId={element.element_id}
          elementName={element.element_name}
        />
      </div>
    )}
  </>
);

// Expanded-row panel: the live mapped-allegation detail, inset under its row with
// an accent left rule so it reads as belonging to the row above it.
const EXPAND_STYLE: React.CSSProperties = {
  margin: "0 4px 12px",
  padding: "12px 16px",
  borderLeft: "3px solid var(--accent-primary)",
  backgroundColor: "var(--bg-page)",
  borderRadius: "0 6px 6px 0",
};

export default MatrixRowWithDetail;
