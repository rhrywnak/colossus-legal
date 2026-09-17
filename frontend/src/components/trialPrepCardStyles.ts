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

/** The prep headline — "42 of 42 answered". */
export const warRoomAnsweredStyle: React.CSSProperties = {
  marginTop: "2px",
  fontSize: "1.05rem",
  color: "var(--text-primary)",
};

/** The one muted line under the headline. */
export const warRoomMetaStyle: React.CSSProperties = {
  fontSize: "0.84rem",
  color: "var(--text-muted)",
  marginBottom: "6px",
};

/** The prep pane's header row: the heading left, "Practice →" right. */
export const warRoomPrepHeaderStyle: React.CSSProperties = {
  display: "flex",
  justifyContent: "space-between",
  alignItems: "baseline",
  gap: "12px",
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

/** A pill: amber for owed work, gray for not started, green for nothing owed. */
export function warRoomBadgeStyle(
  kind: "viewer" | "marie" | "not_started" | "up_to_date",
): React.CSSProperties {
  const [bg, ink] =
    kind === "viewer" || kind === "marie"
      ? ["var(--burden-warning-bg)", "var(--burden-warning-text)"]
      : kind === "not_started"
        ? ["var(--burden-neutral-bg)", "var(--burden-neutral-text)"]
        : ["var(--state-success-bg-soft)", "var(--status-active-text)"];
  return {
    ...pillStyle,
    alignSelf: "flex-start",
    marginTop: "8px",
    padding: "4px 12px",
    fontSize: "0.82rem",
    backgroundColor: bg,
    color: ink,
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

/** An action link; `bold` for the prep pane's "Practice →". */
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

/** Delete: RED, the existing danger token (CC_TASK_REVIEW_LOOP_v1 §5). */
export const warRoomDeleteStyle: React.CSSProperties = {
  ...warRoomActionStyle(false),
  color: "var(--state-danger-strong)",
};

/** The card title, which is now the scenario link. */
export const warRoomTitleStyle: React.CSSProperties = {
  margin: 0,
  fontSize: "1.3rem",
};

/**
 * The accessible-card click zones (Kitty Giraudel, "Accessible Cards").
 *
 * ## Why CSS and not an onClick on a div
 *
 * A clickable `<div>` is invisible to a keyboard and a screen reader, and
 * wrapping a whole pane in an `<a>` would nest Delete and Timeline inside a link
 * — invalid markup, and a click on Delete that also navigates. Instead each zone
 * has ONE real link (the title; "Practice →"), and that link's `::after` is
 * stretched over its pane. The pane is `position: relative`, so the pseudo-
 * element fills exactly that pane and no other. Delete and Timeline sit ABOVE it
 * (`z-index: 2`) inside an action row that is itself raised — so the space
 * around them is dead, and a near-miss on Delete opens nothing.
 *
 * Hover and keyboard focus tint the zone through `:has()`, the same state the
 * link itself is in. A style object cannot carry `::after` or `:has()`, which is
 * why this one piece is a stylesheet string (the `LINK_CSS` precedent).
 */
export const WAR_ROOM_CARD_CSS = `
[data-war-room-card] [data-zone] { position: relative; transition: background-color 0.12s; }
[data-war-room-card] [data-zone-link] { color: var(--accent-primary); text-decoration: none; }
[data-war-room-card] [data-zone-link]::after { content: ""; position: absolute; inset: 0; z-index: 1; }
[data-war-room-card] [data-zone-above] { position: relative; z-index: 2; }
[data-war-room-card] [data-zone]:has([data-zone-link]:hover),
[data-war-room-card] [data-zone]:has([data-zone-link]:focus-visible) {
  background-color: var(--accent-bg-soft);
  box-shadow: inset 0 0 0 1px var(--accent-primary);
}
[data-war-room-card] [data-zone-link]:focus-visible { outline: 2px solid var(--accent-primary); outline-offset: 2px; }
`;
