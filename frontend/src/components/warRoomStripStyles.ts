// =============================================================================
// warRoomStripStyles.ts — the queue strip's style objects
// =============================================================================
//
// Its own file rather than more of `trialPrepCardStyles.ts`, which holds the
// card. Tokens only — no literal colour (Rule 2).
//
// STRUCTURAL: geometry transcribed from the ruled mockup v3 — not settings.

import type React from "react";

import type { StripTile } from "./warRoomStripView";

/** One white band of four equal tiles; wraps to two-by-two, then a stack. */
export const warRoomStripStyle: React.CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  border: "1px solid var(--border-default)",
  backgroundColor: "var(--bg-surface)",
  borderRadius: "var(--radius-card)",
  boxShadow: "var(--shadow-card)",
  padding: "14px 0",
  marginBottom: "1.5rem",
};

/** One tile; every tile after the first carries the hairline divider. */
export function warRoomTileStyle(first: boolean): React.CSSProperties {
  return {
    flex: "1 1 180px",
    textAlign: "center",
    padding: "6px 12px",
    borderLeft: first ? "none" : "1px solid var(--border-default)",
    color: "var(--text-secondary)",
    fontSize: "0.95rem",
  };
}

/** The big number: amber for owed work, link blue for Roman's queue, else ink. */
export function warRoomTileValueStyle(tile: StripTile): React.CSSProperties {
  const color = tile.warning
    ? "var(--burden-warning-text)"
    : tile.key === "candidates"
      ? "var(--accent-primary)"
      : "var(--text-primary)";
  return {
    fontSize: "1.7rem",
    fontWeight: 700,
    color,
    fontVariantNumeric: "tabular-nums",
  };
}
