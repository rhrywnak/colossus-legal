// =============================================================================
// settingsPageStyles — the mockup's geometry, in tokens
// =============================================================================
//
// ADMIN_SETTINGS_MOCKUP_v3 (2026-09-19) is the ruled design. Its palette is
// restated here through the design tokens rather than through its hex literals,
// so the page follows the rest of the application when the theme moves. The
// mapping, stated once so a later reader can check the reproduction:
//
//   mockup --ink     → --text-primary        mockup --line    → --border-default
//   mockup --soft    → --text-secondary      mockup --card    → --bg-surface
//   mockup --faint   → --text-muted          mockup --page    → --bg-page
//   mockup --accent  → --accent-primary      mockup --amber   → --state-warning-strong
//   nav .on          → --accent-bg-soft      mockup --amberbg → --state-warning-bg-soft
//
// Sizes are the mockup's own, in rem where it used px at the 16px root: its
// 13.5px label is 0.84rem, its 11px key line 0.69rem. Nothing here is specific
// to this application; it is the shape of a settings page.

import type React from "react";

/** The mockup's 210px rail. */
export const RAIL_WIDTH = "13.1rem";

export const pageStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "flex-start",
  gap: "0.5rem",
  padding: "1.1rem 1.5rem 1.75rem",
  maxWidth: "73.75rem",
  margin: "0 auto",
  background: "var(--bg-page)",
};

export const railStyle: React.CSSProperties = {
  width: RAIL_WIDTH,
  flexShrink: 0,
  paddingTop: "0.15rem",
  display: "flex",
  flexDirection: "column",
  gap: "0.1rem",
};

export const railItemStyle = (selected: boolean): React.CSSProperties => ({
  display: "flex",
  justifyContent: "space-between",
  alignItems: "baseline",
  gap: "0.5rem",
  width: "100%",
  padding: "0.375rem 0.75rem",
  border: "none",
  borderRadius: "0.375rem",
  textAlign: "left",
  cursor: "pointer",
  fontSize: "0.81rem",
  fontFamily: "inherit",
  background: selected ? "var(--accent-bg-soft)" : "transparent",
  color: selected ? "var(--accent-primary)" : "var(--text-secondary)",
  fontWeight: selected ? 700 : 400,
});

export const railCountStyle: React.CSSProperties = {
  color: "var(--text-muted)",
  fontSize: "0.72rem",
  fontWeight: 400,
};

export const mainStyle: React.CSSProperties = {
  flex: 1,
  minWidth: 0,
  padding: "0 0.5rem 1rem 1rem",
};

export const searchStyle: React.CSSProperties = {
  width: "100%",
  boxSizing: "border-box",
  fontSize: "0.875rem",
  fontFamily: "inherit",
  padding: "0.625rem 0.8rem",
  border: "1px solid var(--border-default)",
  borderRadius: "0.5rem",
  background: "var(--bg-surface)",
  color: "var(--text-primary)",
};

export const metaStyle: React.CSSProperties = {
  fontSize: "0.75rem",
  color: "var(--text-muted)",
  margin: "0.3rem 0 0.875rem",
  lineHeight: 1.5,
};

export const groupHeadingStyle: React.CSSProperties = {
  fontSize: "0.875rem",
  fontWeight: 800,
  margin: "0.875rem 0 0.5rem",
  color: "var(--text-primary)",
};

export const blockButtonStyle: React.CSSProperties = {
  width: "100%",
  background: "var(--bg-surface)",
  border: "1px solid var(--border-default)",
  borderRadius: "0.5rem",
  marginBottom: "0.5rem",
  display: "flex",
  justifyContent: "space-between",
  alignItems: "center",
  gap: "0.75rem",
  padding: "0.625rem 1rem",
  fontSize: "0.84rem",
  fontFamily: "inherit",
  fontWeight: 700,
  color: "var(--text-primary)",
  textAlign: "left",
  cursor: "pointer",
};

export const blockCountStyle: React.CSSProperties = {
  fontWeight: 600,
  fontSize: "0.75rem",
  color: "var(--text-muted)",
};

/** The in-group filter, pinned to the top of a long group. */
export const groupFilterStyle: React.CSSProperties = {
  position: "sticky",
  top: 0,
  zIndex: 1,
  background: "var(--state-warning-bg-soft)",
  border: "1px solid var(--border-default)",
  borderRadius: "0.5rem",
  padding: "0.5rem 0.75rem",
  marginBottom: "0.5rem",
  fontSize: "0.78rem",
  color: "var(--text-secondary)",
  display: "flex",
  gap: "0.5rem",
  alignItems: "center",
  flexWrap: "wrap",
};

export const groupFilterInputStyle: React.CSSProperties = {
  flex: 1,
  minWidth: "9rem",
  border: "1px solid var(--border-default)",
  borderRadius: "0.375rem",
  padding: "0.3rem 0.55rem",
  fontSize: "0.78rem",
  fontFamily: "inherit",
  background: "var(--bg-surface)",
  color: "var(--text-primary)",
};

export const rowStyle: React.CSSProperties = {
  background: "var(--bg-surface)",
  border: "1px solid var(--border-default)",
  borderRadius: "0.5rem",
  padding: "0.7rem 1rem",
  marginBottom: "0.5rem",
};

export const rowLabelStyle: React.CSSProperties = {
  fontSize: "0.84rem",
  fontWeight: 700,
  lineHeight: 1.5,
  color: "var(--text-primary)",
};

export const rowKeyStyle: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: "0.69rem",
  color: "var(--text-muted)",
  marginTop: "0.1rem",
  wordBreak: "break-all",
};

export const editRowStyle: React.CSSProperties = {
  display: "flex",
  alignItems: "center",
  gap: "0.5rem",
  marginTop: "0.5rem",
  flexWrap: "wrap",
};

/**
 * The value field. Narrow for a bounded parameter, wide for free text.
 *
 * Cosmetic only, and deliberately derived from the bounds line rather than from
 * a kind on the wire: a page that knew a row was numeric would be one short step
 * from checking the number, and every rule about what a value may be lives on
 * the backend beside the stored bounds. The worst this can get wrong is a text
 * box wider than it needed to be.
 */
export const valueFieldStyle = (narrow: boolean): React.CSSProperties => ({
  flex: narrow ? "0 0 auto" : 1,
  width: narrow ? "7rem" : undefined,
  minWidth: narrow ? undefined : "12rem",
  maxWidth: narrow ? "7rem" : "25rem",
  border: "1px solid var(--border-default)",
  borderRadius: "0.375rem",
  padding: "0.375rem 0.625rem",
  fontSize: "0.78rem",
  fontFamily: "inherit",
  background: "var(--bg-surface)",
  color: "var(--text-primary)",
});

export const saveButtonStyle: React.CSSProperties = {
  fontSize: "0.75rem",
  color: "var(--accent-on-fill)",
  background: "var(--accent-primary)",
  border: "none",
  borderRadius: "0.375rem",
  padding: "0.375rem 0.875rem",
  fontWeight: 700,
  fontFamily: "inherit",
  cursor: "pointer",
};

export const footStyle: React.CSSProperties = {
  display: "flex",
  gap: "0.875rem",
  marginTop: "0.375rem",
  fontSize: "0.72rem",
  color: "var(--text-muted)",
  flexWrap: "wrap",
  lineHeight: 1.5,
};

export const changedStyle: React.CSSProperties = {
  color: "var(--state-warning-strong)",
  fontWeight: 700,
};

export const markStyle: React.CSSProperties = {
  background: "var(--state-warning-bg-soft)",
  borderRadius: "0.19rem",
  padding: "0 0.13rem",
};

export const noteStyle: React.CSSProperties = {
  fontSize: "0.78rem",
  lineHeight: 1.6,
  color: "var(--state-warning-strong)",
  background: "var(--state-warning-bg-soft)",
  border: "1px solid var(--border-default)",
  borderRadius: "0.5rem",
  padding: "0.6rem 0.8rem",
  margin: "0 0 0.75rem",
};

export const ellipsisStyle: React.CSSProperties = {
  fontSize: "0.75rem",
  color: "var(--text-muted)",
  padding: "0.25rem 0.13rem 0.625rem",
};

export const errorStyle: React.CSSProperties = {
  fontSize: "0.78rem",
  color: "var(--state-danger-strong)",
  marginTop: "0.3rem",
  lineHeight: 1.5,
};

export const confirmationStyle: React.CSSProperties = {
  margin: "0 0 0.875rem",
  padding: "0.6rem 0.8rem",
  border: "1px solid var(--border-default)",
  borderRadius: "0.5rem",
  background: "var(--state-success-bg-soft)",
  color: "var(--text-primary)",
  fontSize: "0.81rem",
  lineHeight: 1.5,
};
