// =============================================================================
// StatusPill.tsx — the Status column cell for the Proof Matrix (PM4)
// -----------------------------------------------------------------------------
// Renders an Element's backend-derived proof status as a colored pill. The four
// values come straight off the wire (`ElementProofStatus`); the frontend only
// renders the label/treatment — it never derives status (Rule 19).
//
// The labels are PRESENCE-OF-EVIDENCE, not legal sufficiency: "Supported" (every
// mapped allegation has corroboration), NOT "Proven". "No allegations" (nothing
// mapped to the Element) is a distinct neutral state from "Gap" (allegations
// mapped, none corroborated).
// =============================================================================

import React from "react";
import { ElementProofStatus } from "../services/proofMatrix";

export interface StatusPillProps {
  status: ElementProofStatus;
}

/** Human-readable label per status. */
const STATUS_LABELS: Record<ElementProofStatus, string> = {
  supported: "Supported",
  partial: "Partial",
  gap: "Gap",
  no_allegations: "No allegations",
};

/**
 * Token-based colors per status — success/warning/danger for the three coverage
 * states, neutral-muted for "no allegations". All from design tokens (Rule 2).
 */
const STATUS_COLORS: Record<ElementProofStatus, { bg: string; fg: string }> = {
  supported: { bg: "var(--state-success-bg-soft)", fg: "var(--state-success-strong)" },
  partial: { bg: "var(--burden-warning-bg)", fg: "var(--burden-warning-text)" },
  gap: { bg: "var(--state-danger-bg-soft)", fg: "var(--state-danger-strong)" },
  no_allegations: { bg: "var(--burden-neutral-bg)", fg: "var(--text-muted)" },
};

/** Neutral fallback if an unexpected status string ever arrives — render a
 *  muted pill rather than crash on an undefined palette (Rule 1). */
const NEUTRAL_PALETTE = { bg: "var(--burden-neutral-bg)", fg: "var(--text-muted)" };

const StatusPill: React.FC<StatusPillProps> = ({ status }) => {
  const palette = STATUS_COLORS[status] ?? NEUTRAL_PALETTE;
  const label = STATUS_LABELS[status] ?? status;
  return (
    <span style={{ ...PILL_STYLE, backgroundColor: palette.bg, color: palette.fg }}>
      {label}
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
