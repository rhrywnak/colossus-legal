// =============================================================================
// warRoomSummaryStyles.ts — the War Room summary card's style objects
// =============================================================================
//
// Transcribed from WAR_ROOM_SUMMARY_CARD_RULED_2026-09-17. Tokens only — no
// literal colour (Rule 2). The top row's pale ground is `--practice-quiet-bg`
// (GO ruling 3: no new token for one shade).
//
// STRUCTURAL: geometry from one approved drawing — not settings.

import type React from "react";

import type { QueueOwner } from "./warRoomSummaryView";

/** Each owner's stripe, chip ground and chip ink — existing tokens only. */
const OWNER_TOKENS: Record<QueueOwner, { stripe: string; chipBg: string; chipInk: string }> = {
  marie: {
    stripe: "var(--accent-primary)",
    chipBg: "var(--state-info-bg-soft)",
    chipInk: "var(--accent-primary)",
  },
  reviewer: {
    stripe: "var(--state-warning-strong)",
    chipBg: "var(--burden-warning-bg)",
    chipInk: "var(--burden-warning-text)",
  },
  roman: {
    stripe: "var(--state-success-strong)",
    chipBg: "var(--state-success-bg-soft)",
    chipInk: "var(--status-active-text)",
  },
};

/** The card frame. */
export const summaryCardStyle: React.CSSProperties = {
  border: "1px solid var(--border-default)",
  backgroundColor: "var(--bg-surface)",
  borderRadius: "var(--radius-card)",
  boxShadow: "var(--shadow-card)",
  overflow: "hidden",
  marginBottom: "1.5rem",
};

/** The quiet top row. */
export const summaryTopStyle: React.CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  alignItems: "center",
  justifyContent: "space-between",
  gap: "12px 24px",
  padding: "16px 24px",
  backgroundColor: "var(--practice-quiet-bg)",
  borderBottom: "1px solid var(--border-default)",
};

/** The three small metrics, divided by hairlines. */
export const summaryMetricsStyle: React.CSSProperties = { display: "flex", flexWrap: "wrap", alignItems: "center" };

/** One small metric; every one after the first carries the divider. */
export function summaryMetricStyle(first: boolean): React.CSSProperties {
  return {
    padding: first ? "0 24px 0 0" : "0 24px",
    borderLeft: first ? "none" : "1px solid var(--border-default)",
    color: "var(--text-secondary)",
    fontSize: "0.95rem",
  };
}

/** The small metric's number. */
export const summaryMetricValueStyle: React.CSSProperties = {
  color: "var(--text-primary)",
  fontWeight: 700,
  fontSize: "1.35rem",
  marginRight: "8px",
  fontVariantNumeric: "tabular-nums",
};

/** The answered figure: label, bar, count. */
export const summaryAnsweredStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "12px",
  flex: "1 1 320px",
  justifyContent: "flex-end",
  color: "var(--text-secondary)",
  fontSize: "0.95rem",
};

/** The bar's track. */
export const summaryBarTrackStyle: React.CSSProperties = {
  flex: "0 1 220px",
  height: "7px",
  borderRadius: "9999px",
  backgroundColor: "var(--border-default)",
  overflow: "hidden",
};

/** The bar's fill. */
export function summaryBarFillStyle(fraction: number): React.CSSProperties {
  return { width: `${Math.round(fraction * 100)}%`, height: "100%", backgroundColor: "var(--accent-primary)" };
}

/** The bottom row: three equal cells. */
export const summaryCellsStyle: React.CSSProperties = { display: "flex", flexWrap: "wrap" };

/** One cell, with its owner's coloured left stripe. */
export function summaryCellStyle(owner: QueueOwner, first: boolean): React.CSSProperties {
  return {
    flex: "1 1 260px",
    minWidth: 0,
    padding: "18px 24px 20px",
    borderLeft: first ? "none" : "1px solid var(--border-default)",
    boxShadow: `inset 4px 0 0 ${OWNER_TOKENS[owner].stripe}`,
  };
}

/** The label + chip row. */
export const summaryCellHeadStyle: React.CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "center",
  gap: "12px",
  color: "var(--text-secondary)",
  fontWeight: 600,
  fontSize: "0.98rem",
};

/** The owner chip. */
export function summaryChipStyle(owner: QueueOwner): React.CSSProperties {
  return {
    padding: "3px 10px",
    borderRadius: "9999px",
    fontSize: "0.74rem",
    fontWeight: 700,
    letterSpacing: "0.06em",
    textTransform: "uppercase",
    backgroundColor: OWNER_TOKENS[owner].chipBg,
    color: OWNER_TOKENS[owner].chipInk,
    whiteSpace: "nowrap",
  };
}

/** The big count: warning ink for owed work, else ink. */
export function summaryCountStyle(warning: boolean): React.CSSProperties {
  return {
    fontSize: "2.4rem",
    fontWeight: 700,
    lineHeight: 1.2,
    margin: "4px 0 2px",
    color: warning ? "var(--burden-warning-text)" : "var(--text-primary)",
    fontVariantNumeric: "tabular-nums",
  };
}

/** The muted context line. */
export const summaryContextStyle: React.CSSProperties = { color: "var(--text-muted)", fontSize: "0.9rem" };
