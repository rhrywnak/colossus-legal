// =============================================================================
// windowStyles.ts — the floating timeline window
// =============================================================================
//
// TIMELINE_SUBSET_MOCKUP_v1_2026-08-30.html Screen 1, approved as drawn —
// Roman: "the window's layout and colors ... are approved exactly". The sibling
// of `dockStyles.ts`: that is the button and the row in the header, this is the
// window they open.
//
// CONST: geometry and rhythm, not settings. There is no frontend config surface
// for a window's furniture and these are not per-deployment values — they are
// one approved drawing, transcribed. The mockup's own pixel values are in the
// comments beside each rule so a screenshot can be diffed against the drawing
// without opening the HTML.
//
// ## ⚑ THE MOCKUP'S PALETTE IS NOT THIS APP'S PALETTE
//
// Same deviation the Subsets section made, for the same reason: the mockup is
// standalone HTML with literal hexes and no dark theme. Every colour here is
// the app token that plays the mockup's role — `--accent-primary` for the
// indigo furniture, `--state-info-bg-soft` for its pale ground,
// `--burden-warning-text` / `--burden-warning-bg` for the amber gap badge.
// Transcribing the hexes would have reproduced the drawing and broken the theme.

import type { CSSProperties } from "react";

// ─── the floating window (mockup `.fw`) ─────────────────────────────────────

/** Mockup `.fw`: rounded, indigo-bordered, its own shadow, column layout. */
export const shell: CSSProperties = {
  display: "flex",
  flexDirection: "column",
  height: "100%",
  width: "100%",
  background: "var(--bg-surface)",
  border: "1px solid var(--accent-primary)",
  borderRadius: "12px",
  boxShadow: "0 12px 32px rgba(17, 24, 39, 0.18)",
  overflow: "hidden",
  boxSizing: "border-box",
};

/** Mockup `.fwbar`: pale indigo, `cursor:move` — the ONLY drag handle. */
export const bar: CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "0.5rem",
  padding: "0.55rem 0.75rem",
  background: "var(--state-info-bg-soft)",
  borderBottom: "1px solid var(--accent-primary)",
  cursor: "move",
  userSelect: "none",
};

export const barTitle: CSSProperties = {
  fontWeight: 800,
  fontSize: "0.86rem",
  color: "var(--text-primary)",
  whiteSpace: "nowrap",
  overflow: "hidden",
  textOverflow: "ellipsis",
};

export const barCount: CSSProperties = {
  color: "var(--accent-primary)",
  fontSize: "0.72rem",
  fontWeight: 600,
  whiteSpace: "nowrap",
};

export const barSelect: CSSProperties = {
  marginLeft: "0.25rem",
  fontSize: "0.72rem",
  border: "1px solid var(--accent-primary)",
  borderRadius: "6px",
  background: "var(--bg-surface)",
  color: "var(--text-primary)",
  padding: "0.05rem 0.25rem",
  maxWidth: "9rem",
};

export const barActions: CSSProperties = { marginLeft: "auto", display: "flex", gap: "0.25rem" };

export const barButton: CSSProperties = {
  border: "none",
  background: "transparent",
  fontSize: "0.9rem",
  cursor: "pointer",
  color: "var(--accent-primary)",
  width: "24px",
  height: "24px",
  borderRadius: "6px",
  fontFamily: "inherit",
  lineHeight: 1,
};

/** Mockup `.fwdesc`: the subset's description, serif, on a muted strip. */
export const description: CSSProperties = {
  fontFamily: "Georgia, 'Times New Roman', serif",
  fontSize: "0.81rem",
  color: "var(--text-secondary)",
  padding: "0.5rem 0.875rem",
  borderBottom: "1px solid var(--border-default)",
  background: "var(--bg-page)",
};

/** Mockup `.fwbody`: the ONLY thing that scrolls. The page beneath never moves. */
export const body: CSSProperties = {
  flex: 1,
  overflowY: "auto",
  padding: "0.625rem 0.75rem",
};

/** Mockup `.pdiv`: a thin labelled rule between phases. */
export function phaseDivider(color: string): CSSProperties {
  return {
    display: "flex",
    alignItems: "center",
    gap: "0.5rem",
    fontSize: "0.68rem",
    fontWeight: 700,
    letterSpacing: "0.06em",
    textTransform: "uppercase",
    color,
    margin: "0.625rem 0 0.4rem",
  };
}

export const dividerRule: CSSProperties = {
  flex: 1,
  height: "1px",
  background: "var(--border-default)",
};

/** Mockup `.fev`: `grid-template-columns:78px 12px 1fr`. */
export function eventRow(removed: boolean): CSSProperties {
  return {
    display: "grid",
    gridTemplateColumns: "78px 12px 1fr",
    gap: "0.5rem",
    padding: "0.5rem",
    border: removed ? "1px dashed var(--burden-warning-text)" : "1px solid var(--border-default)",
    borderRadius: "8px",
    marginBottom: "0.375rem",
    background: removed ? "var(--burden-warning-bg)" : "var(--bg-surface)",
    textAlign: "left",
    width: "100%",
    boxSizing: "border-box",
    cursor: "pointer",
    fontFamily: "inherit",
  };
}

export const eventDate: CSSProperties = {
  fontSize: "0.7rem",
  color: "var(--text-muted)",
  whiteSpace: "nowrap",
  paddingTop: "0.05rem",
};

export function eventDot(color: string): CSSProperties {
  return {
    width: "8px",
    height: "8px",
    borderRadius: "50%",
    marginTop: "0.35rem",
    background: color,
  };
}

export const eventTitle: CSSProperties = {
  margin: 0,
  fontSize: "0.79rem",
  fontWeight: 700,
  color: "var(--text-primary)",
};

export const removedTitle: CSSProperties = {
  ...eventTitle,
  color: "var(--text-muted)",
  textDecoration: "line-through",
};

export const eventFact: CSSProperties = {
  margin: "0.2rem 0 0",
  fontSize: "0.73rem",
  color: "var(--text-secondary)",
};

/** Mockup `.gap .badge`: the amber "not on the chronology" pill. */
export const gapBadge: CSSProperties = {
  display: "inline-block",
  fontSize: "0.62rem",
  fontWeight: 700,
  color: "var(--burden-warning-text)",
  background: "var(--bg-surface)",
  border: "1px solid var(--burden-warning-text)",
  borderRadius: "999px",
  padding: "0.05rem 0.45rem",
  marginLeft: "0.35rem",
};

/** Mockup `.fev .n`: the author's story note — italic, indigo. */
export const storyNote: CSSProperties = {
  marginTop: "0.3rem",
  fontSize: "0.7rem",
  fontStyle: "italic",
  color: "var(--accent-primary)",
};

/** Mockup `.fwfoot`. */
export const foot: CSSProperties = {
  borderTop: "1px solid var(--border-default)",
  padding: "0.4rem 0.875rem",
  fontSize: "0.7rem",
  display: "flex",
  gap: "0.875rem",
  alignItems: "center",
  background: "var(--bg-page)",
};

export const footLink: CSSProperties = {
  color: "var(--accent-primary)",
  textDecoration: "none",
  fontWeight: 600,
  background: "none",
  border: "none",
  cursor: "pointer",
  fontFamily: "inherit",
  fontSize: "0.7rem",
  padding: 0,
};

export const footCount: CSSProperties = { marginLeft: "auto", color: "var(--text-muted)" };

/** The minimized bar, pinned bottom-right (§5C). */
export const minimizedBar: CSSProperties = {
  ...bar,
  borderRadius: "10px",
  border: "1px solid var(--accent-primary)",
  boxShadow: "0 8px 20px rgba(17, 24, 39, 0.16)",
  height: "100%",
  boxSizing: "border-box",
};

export const state: CSSProperties = {
  padding: "0.875rem",
  fontSize: "0.78rem",
  color: "var(--text-muted)",
};

export const errorState: CSSProperties = {
  ...state,
  color: "var(--burden-warning-text)",
  fontWeight: 600,
};

