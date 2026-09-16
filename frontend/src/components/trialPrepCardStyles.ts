// =============================================================================
// trialPrepCardStyles.ts — the two style objects the dashboard grid and the
// timeline views both use
// =============================================================================
//
// Extracted on 2026-08-07 when `ScenarioCard` moved to its own file: both it and
// `TrialPrepViews` need `pillStyle`, and a second copy of a shape token is how
// two pills stop looking alike. Design tokens only — no literal colors (Rule 2).

import React from "react";

/** The dashboard grid's card frame. */
export const scenarioCardStyle: React.CSSProperties = {
  border: "1px solid var(--border-default)",
  backgroundColor: "var(--bg-surface)",
  borderRadius: "8px",
  padding: "14px 16px",
  display: "flex",
  flexDirection: "column",
  gap: "8px",
};

/** The rounded chip used for status/pattern flags. Callers supply the colors. */
export const pillStyle: React.CSSProperties = {
  display: "inline-block",
  padding: "0.12rem 0.5rem",
  borderRadius: "9999px",
  fontSize: "0.72rem",
  fontWeight: 600,
};

// ─── The War Room status card (CC_TASK_WAR_ROOM_v1) ──────────────────────────
//
// WAR_ROOM_CARD_MOCKUP_v1_2026-09-16, as drawn. Tokens only — no new colour:
// hairlines `--border-default`, links and the bar `--accent-primary`, the warning
// ink and the amber pill the burden-warning pair (the mockup's orange-brown), the
// green pill the success pair the case status already uses.
//
// STRUCTURAL: geometry transcribed from one approved drawing — pane widths, padding,
// type sizes. Not per-deployment values, so not settings.

/** The card frame: one row of three panes, wrapping to a stack when narrow. */
export const warRoomCardStyle: React.CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  alignItems: "stretch",
  border: "1px solid var(--border-default)",
  backgroundColor: "var(--bg-surface)",
  borderRadius: "var(--radius-card)",
  boxShadow: "var(--shadow-card)",
  overflow: "hidden",
};

/** The left pane — widest, flexible. */
export const warRoomLeftPaneStyle: React.CSSProperties = {
  flex: "1 1 320px",
  minWidth: 0,
  padding: "22px 24px",
  display: "flex",
  flexDirection: "column",
  gap: "10px",
};

/** A fixed side pane; `width` is the mockup's (~250px middle, ~280px right). */
export function warRoomSidePaneStyle(width: string): React.CSSProperties {
  return {
    flex: `1 0 ${width}`,
    maxWidth: "100%",
    boxSizing: "border-box",
    padding: "22px 24px",
    borderLeft: "1px solid var(--border-default)",
    display: "flex",
    flexDirection: "column",
    gap: "6px",
  };
}

/** The code chip before the title. */
export const warRoomCodeChipStyle: React.CSSProperties = {
  display: "inline-block",
  padding: "3px 9px",
  borderRadius: "6px",
  backgroundColor: "var(--bg-page)",
  color: "var(--text-secondary)",
  fontSize: "0.82rem",
  fontWeight: 700,
  fontVariantNumeric: "tabular-nums",
  flexShrink: 0,
};

/** The theme statement, clamped to three lines. */
export const warRoomThemeStyle: React.CSSProperties = {
  margin: 0,
  color: "var(--text-secondary)",
  fontSize: "0.95rem",
  lineHeight: 1.55,
  display: "-webkit-box",
  WebkitLineClamp: 3,
  WebkitBoxOrient: "vertical",
  overflow: "hidden",
};

/** EVIDENCE / PREP & REHEARSAL — black, not muted (Roman's one change). */
export const warRoomPaneHeadingStyle: React.CSSProperties = {
  margin: "0 0 6px",
  color: "var(--text-primary)",
  fontSize: "0.74rem",
  fontWeight: 700,
  letterSpacing: "0.08em",
  textTransform: "uppercase",
};

/** One label/value row. */
export const warRoomRowStyle: React.CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  gap: "12px",
  fontSize: "0.9rem",
  color: "var(--text-secondary)",
};

/** A row's value: bold ink, or the warning ink for owed work. */
export function warRoomValueStyle(warning: boolean): React.CSSProperties {
  return {
    fontWeight: 700,
    color: warning ? "var(--burden-warning-text)" : "var(--text-primary)",
    fontVariantNumeric: "tabular-nums",
    whiteSpace: "nowrap",
  };
}

/** The scan line, under a dashed rule. */
export function warRoomScanLineStyle(warning: boolean): React.CSSProperties {
  return {
    marginTop: "8px",
    paddingTop: "8px",
    borderTop: "1px dashed var(--border-default)",
    fontSize: "0.82rem",
    color: warning ? "var(--burden-warning-text)" : "var(--text-muted)",
  };
}

/** The answered line. */
export const warRoomAnsweredStyle: React.CSSProperties = {
  marginTop: "10px",
  fontSize: "0.86rem",
  color: "var(--text-secondary)",
};

/** The progress bar's track. */
export const warRoomBarTrackStyle: React.CSSProperties = {
  height: "5px",
  borderRadius: "9999px",
  backgroundColor: "var(--bg-page)",
  overflow: "hidden",
};

/** The progress bar's fill, `fraction` of the track. */
export function warRoomBarFillStyle(fraction: number): React.CSSProperties {
  return {
    width: `${Math.round(fraction * 100)}%`,
    height: "100%",
    backgroundColor: "var(--accent-primary)",
  };
}

/** The badge row's pill: amber for owed work, green for nothing owed. */
export function warRoomBadgeStyle(kind: "changed" | "up_to_date"): React.CSSProperties {
  return {
    ...pillStyle,
    alignSelf: "flex-start",
    marginTop: "10px",
    padding: "4px 12px",
    fontSize: "0.82rem",
    backgroundColor:
      kind === "changed" ? "var(--burden-warning-bg)" : "var(--state-success-bg-soft)",
    color: kind === "changed" ? "var(--burden-warning-text)" : "var(--status-active-text)",
  };
}

/** The action row under the theme. */
export const warRoomActionsStyle: React.CSSProperties = {
  display: "flex",
  flexWrap: "wrap",
  alignItems: "center",
  gap: "6px 22px",
  marginTop: "auto",
  paddingTop: "6px",
};

/** An action link; `bold` for Open scenario. */
export function warRoomActionStyle(bold: boolean): React.CSSProperties {
  return {
    color: "var(--accent-primary)",
    fontSize: "0.95rem",
    fontWeight: bold ? 700 : 400,
    textDecoration: "none",
    background: "none",
    border: "none",
    padding: 0,
    fontFamily: "inherit",
    cursor: "pointer",
  };
}

/** Delete: last, and quieter than the other three (ruling Q5) — muted, not accent. */
export const warRoomDeleteStyle: React.CSSProperties = {
  ...warRoomActionStyle(false),
  color: "var(--text-muted)",
  fontSize: "0.88rem",
};
