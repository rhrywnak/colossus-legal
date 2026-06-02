// =============================================================================
// StatusPill.tsx — the Status column cell for the Proof Matrix (PM4)
// -----------------------------------------------------------------------------
// Renders an Element's proof status as a colored pill. In v1 every Element is
// 'pending' (a neutral muted pill) because no evidence data exists yet. The
// 'proven' / 'partial' / 'gap' renderings are built now so Stage 2 needs no new
// component code, but they are UNREACHABLE with today's all-'pending' data.
// =============================================================================

import React from "react";
import { ElementProofStatus } from "../services/proofMatrix";

export interface StatusPillProps {
  status: ElementProofStatus;
}

/** Human-readable label per status. */
const STATUS_LABELS: Record<ElementProofStatus, string> = {
  proven: "Proven",
  partial: "Partial",
  gap: "Gap",
  pending: "Pending",
};

/**
 * Token-based colors per status. `pending` is the neutral muted treatment (the
 * only one shown in v1); the others reuse the project's success/warning/danger
 * tokens so they are visually consistent once reachable.
 */
const STATUS_COLORS: Record<ElementProofStatus, { bg: string; fg: string }> = {
  proven: { bg: "var(--state-success-bg-soft)", fg: "var(--state-success-strong)" },
  partial: { bg: "var(--burden-warning-bg)", fg: "var(--burden-warning-text)" },
  gap: { bg: "var(--state-danger-bg-soft)", fg: "var(--state-danger-strong)" },
  pending: { bg: "var(--burden-neutral-bg)", fg: "var(--text-muted)" },
};

const StatusPill: React.FC<StatusPillProps> = ({ status }) => {
  const palette = STATUS_COLORS[status];
  return (
    <span style={{ ...PILL_STYLE, backgroundColor: palette.bg, color: palette.fg }}>
      {STATUS_LABELS[status]}
    </span>
  );
};

const PILL_STYLE: React.CSSProperties = {
  display: "inline-block",
  padding: "2px 10px",
  borderRadius: "12px",
  fontFamily: "var(--font-sans)",
  fontSize: "12px",
  fontWeight: 600,
};

export default StatusPill;
