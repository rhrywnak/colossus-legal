// =============================================================================
// MatrixRowWithDetail.tsx — one PM4 matrix row + its expandable detail
// -----------------------------------------------------------------------------
// Pairs a matrix-variant `ElementRow` with the live mapped-allegation detail
// that drops in below it when the row is expanded. Extracted from
// `ProofMatrixPage` so that page stays within the 300-line module limit and so
// the row+detail unit reads as one thing.
//
// Every column shows REAL backend data, rendered as-is and never derived here
// (Rule 19): `strong_evidence_count`, `approved_evidence_count`,
// `disputing_evidence_count` and `proof_status` are all computed by the backend.
// Since task 396 the third column leads with the STRONG count — proof the other
// side cannot dispute — with the raw approved figure as small print beside it.
//
// The Disputes column was previously fed a hardcoded `[]` on the grounds that no
// REBUTS edges existed — a claim that stopped being true without anyone
// noticing, leaving a blank column over 41 real `Evidence -[:REBUTS]->
// Allegation` edges. It is now wired to the backend count, and the items behind
// it appear in the expanded detail via `ElementDetailContent`, which
// self-fetches them.
// =============================================================================

import React from "react";
import { ElementDetail, MatrixWording } from "../services/causesOfAction";
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
  /**
   * The Proof Matrix's served words, from the payload the page gates on.
   *
   * Passed DOWN rather than fetched here: every row on the page speaks the same
   * eight strings from the same snapshot, and thirty rows each reading the store
   * would be thirty chances for two of them to disagree.
   */
  matrixWording: MatrixWording;
}

const MatrixRowWithDetail: React.FC<MatrixRowWithDetailProps> = ({
  element,
  countNumber,
  index,
  caseSlug,
  expanded,
  onToggleExpand,
  matrixWording,
}) => (
  <>
    <ElementRow
      element={element}
      countNumber={countNumber}
      index={index}
      selected={false}
      onSelect={NOOP_SELECT}
      variant="matrix"
      strongCount={element.strong_evidence_count}
      approvedCount={element.approved_evidence_count}
      matrixWording={matrixWording}
      disputingCount={element.disputing_evidence_count}
      proofStatus={element.proof_status}
      expanded={expanded}
      onToggleExpand={onToggleExpand}
    />
    {expanded && (
      <div style={EXPAND_STYLE}>
        <ElementDetailContent
          caseSlug={caseSlug}
          elementId={element.element_id}
          elementName={element.element_name}
          matrixWording={matrixWording}
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
